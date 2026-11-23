//! Loads an sdlang spec tree and its packages, resolves names and lowers the result to
//! the canonical graph (`schemas/graph.schema.json`).

#![forbid(unsafe_code)]

pub mod error;
pub mod loader;
pub mod lower;
pub mod schema;
pub mod scope;
pub mod selectors;

use std::path::Path;

pub use error::GraphError;

/// Loads the project at `root` and lowers it to its canonical graph.
pub fn build(root: &Path) -> Result<serde_json::Value, GraphError> {
    let project = loader::load(root)?;
    lower::lower_project(&project)
}
