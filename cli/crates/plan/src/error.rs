//! Planner errors. Graph and source errors are reported by `intlang-graph` before the
//! planner runs; these are about state, files and materialisation.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanError {
    pub message: String,
}

impl PlanError {
    pub fn new(message: impl Into<String>) -> Self {
        PlanError {
            message: message.into(),
        }
    }
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PlanError {}

impl From<intlang_canon::GraphError> for PlanError {
    fn from(e: intlang_canon::GraphError) -> Self {
        PlanError::new(e.to_string())
    }
}

impl From<intlang_canon::CanonError> for PlanError {
    fn from(e: intlang_canon::CanonError) -> Self {
        PlanError::new(e.to_string())
    }
}

pub type PResult<T> = Result<T, PlanError>;
