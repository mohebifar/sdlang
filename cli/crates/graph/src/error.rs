//! Errors that point at a span in the file that caused them.

use std::fmt;
use std::path::PathBuf;

use intlang_syntax::ast::Span;
use intlang_syntax::{diagnostics, ParseError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphError {
    pub message: String,
    /// The file, its normalised text and the span, when the error has a location.
    pub location: Option<(PathBuf, String, Span)>,
}

impl GraphError {
    pub fn plain(message: impl Into<String>) -> Self {
        GraphError {
            message: message.into(),
            location: None,
        }
    }

    pub fn at(file: &crate::loader::SourceFile, span: Span, message: impl Into<String>) -> Self {
        GraphError {
            message: message.into(),
            location: Some((file.path.clone(), file.text.clone(), span)),
        }
    }

    /// The error as text, with a source excerpt when it has a location.
    pub fn render(&self) -> String {
        match &self.location {
            Some((path, text, span)) => diagnostics::render(
                &path.display().to_string(),
                text,
                &ParseError::at(self.message.clone(), *span),
            ),
            None => format!("error: {}\n", self.message),
        }
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some((path, text, span)) => {
                let (line, column) = diagnostics::line_col(text, span.start);
                write!(f, "{}:{line}:{column}: {}", path.display(), self.message)
            }
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for GraphError {}

pub type GResult<T> = Result<T, GraphError>;
