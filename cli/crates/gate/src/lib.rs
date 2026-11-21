//! `spec check`, the read-only gate (SPEC.md, Team workflow): whether the spec is applied,
//! owned files match state, ownership is sound and evidence is current. It runs no agent
//! and writes nothing in the project.

#![forbid(unsafe_code)]

pub mod currency;
pub mod error;
pub mod findings;
pub mod ownership;
pub mod rerun;

pub use error::GateError;
pub use findings::{check, CheckOptions, Finding};
