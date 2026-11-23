//! `sdc import`, brownfield adoption (docs/spec.md, Brownfield adoption): proposes an
//! ownership mapping for existing files, and on acceptance writes the proposed libs and the
//! pending acquisitions that the next `sdc plan` turns into `create` with strategy
//! `acquire`.

#![forbid(unsafe_code)]

pub mod accept;
pub mod error;
pub mod proposal;
pub mod scan;

pub use accept::accept;
pub use error::AdoptError;
pub use proposal::{propose, Proposal};
