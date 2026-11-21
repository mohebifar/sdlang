//! The int lang planner: state records, drift, propagation and transactions, producing
//! the plan document (`schemas/plan.schema.json`) as SPEC.md's State, plan and apply
//! section defines it.

#![forbid(unsafe_code)]

pub mod compat;
pub mod error;
pub mod materialise;
pub mod planner;
pub mod state;

pub use error::PlanError;
pub use planner::{
    pending_imports, plan, write_pending_imports, Acquisition, PlanInputs, PENDING_IMPORTS,
};
