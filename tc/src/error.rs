use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;

use deno_core::anyhow::Error as AnyError;

#[derive(Debug)]
pub enum Error {
    Cascade(AnyError),
}

impl From<AnyError> for Error {
    fn from(value: AnyError) -> Self {
        Self::Cascade(value)
    }
}

macro_rules! impl_cascading_errors {
    {$($ty:path$(,)?)+} => {
        $(impl From<$ty> for Error {
            fn from(value: $ty) -> Self {
                Self::Cascade(AnyError::from(value))
            }
        })+
    }
}

impl_cascading_errors!(
    io::Error,
    serde_json::Error,
    deno_core::ModuleResolutionError,
    serde_v8::Error,
    toml::de::Error,
    toml::ser::Error,
    tokio::task::JoinError
);

impl Display for Error {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}