//! Renders parse errors as plain text with a source excerpt.

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::{self, Config};

use crate::parser::ParseError;

/// 1-based line and column (in characters) of a byte offset.
pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

/// Renders `error` against `source` (the normalised text it was parsed from).
/// The result names the file, line and column and underlines the span.
pub fn render(file_name: &str, source: &str, error: &ParseError) -> String {
    let file = SimpleFile::new(file_name, source);
    let start = error.span.start.min(source.len());
    let end = error.span.end.clamp(start, source.len());
    let diagnostic = Diagnostic::error()
        .with_message(&error.message)
        .with_labels(vec![Label::primary((), start..end)]);
    term::emit_into_string(&Config::default(), &file, &diagnostic).unwrap_or_else(|_| {
        let (line, column) = line_col(source, start);
        format!(
            "error: {}\n  --> {file_name}:{line}:{column}\n",
            error.message
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    #[test]
    fn line_and_column() {
        assert_eq!(line_col("ab\ncd", 4), (2, 2));
        assert_eq!(line_col("ab", 0), (1, 1));
    }

    #[test]
    fn excerpt_names_location_and_underlines() {
        let src = "action a {\n  input: { a: string b: string }\n}\n";
        let err = ParseError::at(
            "expected ',' or a line break or '}', found 'b'",
            Span::new(32, 33),
        );
        let text = render("spec/a.int", src, &err);
        assert!(text.contains("spec/a.int:2:22"), "{text}");
        assert!(text.contains('^'), "{text}");
    }
}
