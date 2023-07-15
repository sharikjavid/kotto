use std::error::Error;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use serde::{Serialize, Deserialize};

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, copy, empty};
use tokio::process::Command;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppName(String);

#[derive(Serialize, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    pub source: PathBuf,
    pub dependencies: Vec<String>,
}

pub struct AppsIndex {
    handle: File,
    cached: Vec<IndexEntry>,
}

impl AppsIndex {
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let mut f = OpenOptions::new().read(true).write(true).open(path).await?;

        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;

        let index = serde_json::from_str(&buf)?;

        Ok(Self {
            handle: f,
            cached: index,
        })
    }

    pub async fn save(mut self) -> Result<(), Box<dyn Error>> {
        let as_str = serde_json::to_string(&self.cached)?;
        self.handle.write_all(as_str.as_bytes()).await?;
        Ok(())
    }

    pub fn cached(&self) -> &[IndexEntry] {
        self.cached.as_slice()
    }

    pub fn cached_mut(&mut self) -> &mut Vec<IndexEntry> {
        &mut self.cached
    }
}

pub struct Apps {
    home: PathBuf,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            home: home::home_dir().expect("could not determine $HOME").join(".trackway")
        }
    }
}

impl Apps {
    pub fn new() -> Self {
        Self::default()
    }

    fn make_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.home.join(path)
    }

    pub fn index(&self) -> impl Future<Output = Result<AppsIndex, Box<dyn Error>>> {
        AppsIndex::load(self.make_path("apps.toml"))
    }

    pub async fn run_with_input<I: AsyncRead + Unpin>(&self, app: &IndexEntry, mut input: I) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut cmd = Command::new("/opt/homebrew/bin/deno");

        cmd
            .arg("run")
            .arg(self.make_path("std").to_str().unwrap())
            .arg(self.make_path("cached").join(&app.name).to_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        let mut proc = cmd.spawn()?;

        let mut stdin = proc.stdin.take().unwrap();
        copy(&mut input, &mut stdin).await?;
        drop(stdin);

        let proc_output = proc.wait_with_output().await?;

        Ok(proc_output.stdout)
    }

    pub async fn run_without_input(&self, app: &IndexEntry) -> Result<Vec<u8>, Box<dyn Error>> {
        self.run_with_input(app, empty()).await
    }
}