use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum Error {
    Cascade(Box<dyn StdError>)
}

impl From<Box<dyn StdError>> for Error {
    fn from(value: Box<dyn StdError>) -> Self {
        Self::Cascade(value)
    }
}

macro_rules! impl_cascading_errors {
    {$($ty:path$(,)?)+} => {
        $(impl From<$ty> for Error {
            fn from(value: $ty) -> Self {
                Self::from(Into::<Box<dyn StdError>>::into(value))
            }
        })+
    }
}

impl_cascading_errors!(
    io::Error,
    serde_json::Error,
    deno_core::anyhow::Error,
    deno_core::ModuleResolutionError,

);

impl Display for Error {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}