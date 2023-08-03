use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::rc::Rc;
use serde::{Serialize, Deserialize};

use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use deno_core::{JsRuntime, serde_v8, v8};

use crate::error::Error;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct AppName(String);

impl AppName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    name: String,
    description: String,
    commands: HashMap<String, CommandConfig>
}

#[derive(Serialize, Deserialize)]
pub struct CommandConfig {
    run: String,
    description: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistryEntry {
    pub name: AppName,
    pub source: String,
    pub version: String,
    //#[serde(default)]
    //pub dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Registry {
    version: u32,
    #[serde(default)]
    app: Vec<RegistryEntry>
}

pub struct AppsCache {
    home: PathBuf,
    registry: Vec<RegistryEntry>,
}

impl AppsCache {
    pub async fn load<P: AsRef<Path>>(home: P) -> Result<Self, Error> {
        let path = home.as_ref().join("apps.toml");
        let mut f = OpenOptions::new().read(true).open(path).await?;

        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;

        let index: Registry = toml::from_str(&buf)?;

        Ok(Self {
            home: home.as_ref().to_owned(),
            registry: index.app,
        })
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    fn local_cache_path(&self, name: &AppName) -> PathBuf {
        self.make_path(Path::new("cache").join(name.as_str()).join("mod.ts"))
    }

    pub fn make_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.home.join(path)
    }

    pub async fn save(&mut self) -> Result<(), Error> {
        let as_str = toml::to_string(&Registry {
            version: 1,
            app: self.registry.clone()
        })?;

        let path = self.home.join("apps.toml");
        let mut f = OpenOptions::new().write(true).open(path).await?;

        f.write_all(as_str.as_bytes()).await?;

        Ok(())
    }

    pub async fn get_source(&self, name: &AppName) -> Result<Option<PathBuf>, Error> {
        let app = if let Some(app) = self.get_entry(name) { app } else { return Ok(None) };
        let cached_path = self.local_cache_path(&app.name);

        // lookup if already exists, at the right version, otherwise download here TODO

        Ok(Some(cached_path))
    }

    pub fn get_entry(&self, name: &AppName) -> Option<RegistryEntry> {
        self.registry.iter().find(|e| &e.name == name).map(|e| e.clone())
    }

    pub async fn insert(&mut self, entry: RegistryEntry) -> Option<RegistryEntry> {
        let output = match self.registry.iter_mut().find(|e| {
            e.name == entry.name
        }) {
            Some(existing) => {
                let output = existing.clone();
                *existing = entry;
                Some(output)
            }
            None => {
                self.registry.push(entry);
                None
            }
        };
        self.save().await.unwrap();
        output
    }

    pub async fn remove(&mut self, name: &AppName) -> Option<RegistryEntry> {
        let (idx, entry) = self.registry.iter().enumerate().find(|(_, e)| &e.name == name)?;
        let output = entry.clone();
        self.registry.remove(idx);
        self.save().await.unwrap();
        Some(output)
    }
}

pub struct AppsManager {
    cache: AppsCache,
    apps: HashMap<AppName, Runtime>
}

impl AppsManager {
    pub async fn new() -> Result<Self, Error> {
        let home = home::home_dir().expect("could not determine $HOME").join(".trackway");
        let cache = AppsCache::load(home).await?;

        let mut apps = HashMap::new();
        for entry in &cache.registry {
            let module_path = cache.get_source(&entry.name).await?.unwrap();
            apps.insert(entry.name.clone(), Runtime::new(&module_path).await?);
        }

        Ok(Self {
            cache,
            apps
        })
    }

    pub async fn install_app(&mut self, entry: RegistryEntry) -> Result<AppConfig, Error> {
        let name = entry.name.clone();
        self.cache.insert(entry).await;

        // SAFETY: Never `Ok(None)` because of `.insert` above
        let module_path = self.cache.get_source(&name).await?.unwrap();

        let runtime = self.apps.entry(name).or_insert(Runtime::new(&module_path).await?);
        let config = runtime.get_config().await?;

        Ok(config)
    }

    pub async fn uninstall_app(&mut self, name: &AppName) -> Result<(), Error> {
        self.cache.remove(name).await;
        self.apps.remove(name);
        Ok(())
    }

    pub async fn call(&mut self, name: &AppName, command: &str) -> Option<Result<CommandOutput, Error>> {
        Some(self.apps.get_mut(name)?.call(command).await)
    }
}

struct Runtime {
    rt: JsRuntime,
    module: v8::Global<v8::Object>
}

impl Runtime {
    async fn new(module_path: &Path) -> Result<Self, Error> {
        let main_module = deno_core::resolve_path(module_path.display().to_string().as_str(), Path::new("/"))?;
        let mut rt = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(crate::ts_module_loader::TsModuleLoader)),
            ..Default::default()
        });

        let mod_id = rt.load_main_module(&main_module, None).await?;
        let result = rt.mod_evaluate(mod_id);
        rt.run_event_loop(false).await?;
        let _ = result.await.map_err(Into::<Box<dyn StdError>>::into)?;
        let module = rt.get_module_namespace(mod_id)?;

        Ok(Self { rt, module })
    }

    async fn get_config(&mut self) -> Result<AppConfig, Error> {
        let mut scope = self.rt.handle_scope();
        let config_key = v8::String::new(&mut scope, "config").unwrap();

        // TODO handle error
        let config = self.module.open(&mut scope).get(&mut scope, config_key.into()).unwrap();

        Ok(serde_v8::from_v8(&mut scope, config)?)
    }

    async fn call(&mut self, command: &str) -> Result<CommandOutput, Error> {
        let mut scope = self.rt.handle_scope();
        let command_key = v8::String::new(&mut scope, command).unwrap();

        let command = self.module.open(&mut scope).get(&mut scope, command_key.into()).unwrap();
        let function: v8::Local<v8::Function> = command.try_into().unwrap();
        let mod_local = v8::Local::new(&mut scope, &self.module);
        let result = function.call(&mut scope, mod_local.into(), &[]).unwrap();

        Ok(serde_v8::from_v8(&mut scope, result)?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RunCommand {
    pub app: AppName,
    pub command: String
}

#[derive(Serialize, Deserialize)]
pub struct CommandOutput {
    pub output: String
}

