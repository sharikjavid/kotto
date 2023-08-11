use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use crate::AnyError;

use tracing_subscriber::prelude::*;

pub fn setup_tracing() -> Result<(), AnyError> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(subscriber)?;
    tracing_log::LogTracer::init().map_err(Into::into)
}

pub fn add_extension_to_path<P: AsRef<Path>, S: AsRef<OsStr>>(path: P, ext: S) -> Option<PathBuf> {
    path.as_ref().file_name().map(|file_name| {
        let mut file_name = OsString::from(file_name);
        file_name.push(ext);
        path.as_ref().with_file_name(file_name)
    })
}