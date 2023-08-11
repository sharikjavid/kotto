use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{event, Level};

use crate::error::Error;

#[derive(Serialize, Deserialize)]
pub struct Config {
}

impl Config {
    pub fn path() -> PathBuf {
        home::home_dir().unwrap().join(".config").join("trackway").join("config.toml")
    }

    pub async fn load() -> Result<Self, Error> {
        let path = Self::path();

        let parent = path.parent().unwrap();
        if ! parent.exists() {
            event!(Level::INFO, "config dir {} does not exist, creating", parent.display());
            fs::create_dir_all(parent).await?;
        }

        event!(Level::INFO, "using config {}", path.display());
        let mut f = fs::OpenOptions::new().read(true).write(true).create(true).open(path).await?;

        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;

        Ok(toml::from_str(&buf)?)
    }

    pub async fn save(self) -> Result<(), Error> {
        let mut f = fs::OpenOptions::new().write(true).open(Self::path()).await?;
        let serialized = toml::to_string(&self)?;
        f.write(serialized.as_bytes()).await?;
        Ok(())
    }
}