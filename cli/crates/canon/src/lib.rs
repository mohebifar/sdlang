//! Canonical JSON, digests and declaration projections for int lang, matching
//! `tools/canon.py` byte for byte.
//!
//! ```
//! let value = serde_json::json!({ "b": 1, "a": 2 });
//! assert_eq!(intlang_canon::canonical(&value).unwrap(), r#"{"a":2,"b":1}"#);
//! ```

#![forbid(unsafe_code)]

pub mod canonical_json;
pub mod projections;

pub use canonical_json::{canonical, digest, digest_text, CanonError};
pub use projections::{projections, public_hashes, GraphError, Projections};
