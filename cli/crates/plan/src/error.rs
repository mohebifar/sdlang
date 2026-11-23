//! Planner errors. Graph and source errors are reported by `sdlang-graph` before the
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

impl From<sdlang_canon::GraphError> for PlanError {
    fn from(e: sdlang_canon::GraphError) -> Self {
        PlanError::new(e.to_string())
    }
}

impl From<sdlang_canon::CanonError> for PlanError {
    fn from(e: sdlang_canon::CanonError) -> Self {
        PlanError::new(e.to_string())
    }
}

pub type PResult<T> = Result<T, PlanError>;
