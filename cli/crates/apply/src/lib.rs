//! Applies an sdlang plan (docs/spec.md, Apply; docs/protocol.md): transaction by transaction, in
//! candidate worktrees, with agents, generators, checks and reviews as external commands.

#![forbid(unsafe_code)]

pub mod checks;
pub mod engine;
pub mod error;
pub mod evidence;
pub mod imports;
pub mod mcp;
pub mod names;
pub mod orders;
pub mod process;
pub mod workspace;

pub use engine::{apply, ApplyOptions, ApplyReport, Outcome};
