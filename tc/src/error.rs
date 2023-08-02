use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Cascade(Box<dyn StdError>)
}

impl From<Box<dyn StdError>> for Error {
    fn from(value: Box<dyn StdError>) -> Self {
        Self::Cascade(value)
    }
}

impl Display for Error {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}