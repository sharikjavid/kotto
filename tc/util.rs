use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use crate::AnyError;

use tracing_subscriber::prelude::*;

pub fn setup_tracing() -> Result<(), AnyError> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    tracing::subscriber::set_global_default(subscriber)?;
    tracing_log::LogTracer::init().map_err(Into::into)
}

pub fn add_extension_to_path<P: AsRef<Path>, S: AsRef<OsStr>>(path: P, ext: S) -> PathBuf {
    path.as_ref().with_extension(ext)
}