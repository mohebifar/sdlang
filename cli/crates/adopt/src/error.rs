//! The adoption error: the import cannot be proposed or accepted.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptError {
    pub message: String,
}

impl AdoptError {
    pub fn new(message: impl Into<String>) -> AdoptError {
        AdoptError {
            message: message.into(),
        }
    }
}

impl fmt::Display for AdoptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AdoptError {}

macro_rules! from_error {
    ($($t:ty),*) => {$(
        impl From<$t> for AdoptError {
            fn from(e: $t) -> AdoptError {
                AdoptError::new(e.to_string())
            }
        }
    )*};
}

from_error!(
    sdlang_plan::PlanError,
    sdlang_apply::error::ApplyError,
    sdlang_canon::CanonError
);

pub type AdResult<T> = Result<T, AdoptError>;
