//! Canonical JSON, digests and declaration projections for sdlang, matching
//! `tools/canon.py` byte for byte.
//!
//! ```
//! let value = serde_json::json!({ "b": 1, "a": 2 });
//! assert_eq!(sdlang_canon::canonical(&value).unwrap(), r#"{"a":2,"b":1}"#);
//! ```

#![forbid(unsafe_code)]

pub mod canonical_json;
pub mod projections;

pub use canonical_json::{canonical, digest, digest_bytes, digest_text, CanonError};
pub use projections::{
    documents, projections, public_documents, public_hashes, Documents, GraphDocuments, GraphError,
    Projections,
};
