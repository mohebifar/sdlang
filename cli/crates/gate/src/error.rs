//! The gate's error: something stopped the check from running, as opposed to a finding.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateError {
    pub message: String,
}

impl GateError {
    pub fn new(message: impl Into<String>) -> GateError {
        GateError {
            message: message.into(),
        }
    }
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GateError {}

macro_rules! from_error {
    ($($t:ty),*) => {$(
        impl From<$t> for GateError {
            fn from(e: $t) -> GateError {
                GateError::new(e.to_string())
            }
        }
    )*};
}

from_error!(
    intlang_plan::PlanError,
    intlang_apply::error::ApplyError,
    intlang_canon::CanonError
);

pub type GResult<T> = Result<T, GateError>;
