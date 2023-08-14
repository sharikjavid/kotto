use crate::AnyError;

use tracing_subscriber::prelude::*;

pub fn setup_tracing() -> Result<(), AnyError> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    tracing::subscriber::set_global_default(subscriber)?;
    tracing_log::LogTracer::init().map_err(Into::into)
}