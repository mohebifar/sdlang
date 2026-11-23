//! Lexer and parser for sdlang (`.sd`) source, following `docs/grammar.ebnf`.
//!
//! ```
//! let file = sdlang_syntax::parse_source("import nextjs\n").unwrap();
//! assert_eq!(file.statements.len(), 1);
//! ```

#![forbid(unsafe_code)]

pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod strings;

pub use ast::File;
pub use parser::{parse, ParseError};

/// Normalises line endings, then parses. Spans refer to the normalised text, so a
/// caller that renders diagnostics should normalise with [`lexer::normalize`] itself
/// and call [`parse`].
pub fn parse_source(src: &str) -> Result<File, ParseError> {
    parse(&lexer::normalize(src))
}
