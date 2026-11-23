//! Apply errors: anything that stops the run itself, as opposed to a transaction failing.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyError {
    pub message: String,
}

impl ApplyError {
    pub fn new(message: impl Into<String>) -> Self {
        ApplyError {
            message: message.into(),
        }
    }
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApplyError {}

impl From<sdlang_plan::PlanError> for ApplyError {
    fn from(e: sdlang_plan::PlanError) -> Self {
        ApplyError::new(e.message)
    }
}

impl From<sdlang_canon::GraphError> for ApplyError {
    fn from(e: sdlang_canon::GraphError) -> Self {
        ApplyError::new(e.to_string())
    }
}

impl From<sdlang_canon::CanonError> for ApplyError {
    fn from(e: sdlang_canon::CanonError) -> Self {
        ApplyError::new(e.to_string())
    }
}

pub type AResult<T> = Result<T, ApplyError>;
