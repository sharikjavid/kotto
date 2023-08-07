use std::error::Error as StdError;
use std::path::Path;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use serde::{Serialize, Deserialize};

use deno_core::{JsRuntime, ModuleId, serde_v8, v8};

mod ts_module_loader;
mod compile;
mod emit;

use crate::error::Error;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct AppName(String);

impl Deref for AppName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: AppName,
    pub task: String,
    pub commands: HashMap<String, CommandConfig>
}

#[derive(Serialize, Deserialize)]
pub struct CommandConfig {
    pub run: String,
    pub description: String
}

#[derive(Serialize, Deserialize)]
pub struct RunCommandOptions {
    pub name: String,
    pub query: String
}

#[derive(Serialize, Deserialize)]
pub struct RunCommandOutput {
    pub output: String
}

pub struct Runtime {
    rt: JsRuntime,
    module_id: ModuleId
}

impl Runtime {
    pub async fn new(module_path: &Path) -> Result<Self, Error> {
        let main_module = deno_core::resolve_path(module_path.display().to_string().as_str(), Path::new("/"))?;
        let mut rt = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(ts_module_loader::TsModuleLoader)),
            ..Default::default()
        });

        let module_id = rt.load_main_module(&main_module, None).await?;
        let result = rt.mod_evaluate(module_id);
        rt.run_event_loop(false).await?;
        let _ = result.await.map_err(Into::<Box<dyn StdError>>::into)?;

        Ok(Self { rt, module_id })
    }

    pub fn get_config(&mut self) -> Result<ModuleConfig, Error> {
        let main_module = self.rt.get_module_namespace(self.module_id)?;
        let mut scope = self.rt.handle_scope();
        let attribute_key = v8::String::new(&mut scope, "config").unwrap();
        let attribute_value = main_module.open(&mut scope).get(&mut scope, attribute_key.into()).unwrap();
        let output = serde_v8::from_v8(&mut scope, attribute_value)?;
        Ok(output)
    }

    pub async fn call(&mut self, opts: RunCommandOptions) -> Result<RunCommandOutput, Error> {
        let config = self.get_config()?;
        let attribute_name = &config.commands.get(&opts.name).unwrap().run;

        let main_module = self.rt.get_module_namespace(self.module_id)?;
        let mut scope = self.rt.handle_scope();
        let attribute_key = v8::String::new(&mut scope, attribute_name).unwrap();
        let attribute_value = main_module.open(&mut scope).get(&mut scope, attribute_key.into()).unwrap();

        let function: v8::Local<v8::Function> = attribute_value.try_into().unwrap();
        let mod_local = v8::Local::new(&mut scope, &main_module);
        let query_argument = v8::String::new(&mut scope, &opts.query).unwrap().into();
        let result = function.call(&mut scope, mod_local.into(), &[query_argument]).unwrap();
        let output: String = serde_v8::from_v8(&mut scope, result)?;

        Ok(RunCommandOutput {
            output
        })
    }
}

