//! Applies an int lang plan (SPEC.md, Apply; PROTOCOL.md): transaction by transaction, in
//! candidate worktrees, with agents, generators, checks and reviews as external commands.

#![forbid(unsafe_code)]

pub mod checks;
pub mod engine;
pub mod error;
pub mod evidence;
pub mod orders;
pub mod workspace;

pub use engine::{apply, ApplyOptions, ApplyReport, Outcome};
