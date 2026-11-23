//! Positions: byte offsets in the normalised text (what the parser's spans use) to LSP
//! line and character, counted in UTF-16 code units as LSP requires by default.

use std::path::{Path, PathBuf};

use lsp_types::{Position, Range, Url};
use sdlang_syntax::ast::Span;

/// Line starts of one text, for converting offsets and positions.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    pub fn position(&self, text: &str, offset: usize) -> Position {
        let offset = offset.min(text.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let start = self.starts[line];
        let character = text[start..floor_char(text, offset)].encode_utf16().count();
        Position::new(line as u32, character as u32)
    }

    pub fn range(&self, text: &str, span: Span) -> Range {
        Range::new(
            self.position(text, span.start),
            self.position(text, span.end),
        )
    }

    /// The byte offset of an LSP position, clamped to the line it names.
    pub fn offset(&self, text: &str, pos: Position) -> usize {
        let Some(&start) = self.starts.get(pos.line as usize) else {
            return text.len();
        };
        let end = self
            .starts
            .get(pos.line as usize + 1)
            .map_or(text.len(), |e| e - 1);
        let mut units = 0;
        for (i, c) in text[start..end].char_indices() {
            if units >= pos.character as usize {
                return start + i;
            }
            units += c.len_utf16();
        }
        end
    }
}

fn floor_char(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

pub fn url_to_path(url: &Url) -> Option<PathBuf> {
    let path = url.to_file_path().ok()?;
    Some(std::fs::canonicalize(&path).unwrap_or(path))
}

pub fn path_to_url(path: &Path) -> Option<Url> {
    Url::from_file_path(path).ok()
}

/// The word (`[A-Za-z0-9_]+`) around `offset`, with its span.
pub fn word_at(text: &str, offset: usize) -> Option<(String, Span)> {
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let bytes = text.as_bytes();
    let offset = offset.min(bytes.len());
    let mut start = offset;
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    (start < end).then(|| (text[start..end].to_string(), Span::new(start, end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_positions_round_trip() {
        let text = "ab\nc\u{e9}d\n\u{1f600}x";
        let idx = LineIndex::new(text);
        assert_eq!(idx.position(text, 4), Position::new(1, 1));
        assert_eq!(idx.position(text, 6), Position::new(1, 2)); // after é
        let x = text.find('x').unwrap();
        assert_eq!(idx.position(text, x), Position::new(2, 2)); // the emoji is 2 units
        assert_eq!(idx.offset(text, Position::new(2, 2)), x);
        assert_eq!(idx.offset(text, Position::new(0, 99)), 2);
    }

    #[test]
    fn words() {
        assert_eq!(word_at("uses: [cart]", 8).unwrap().0, "cart");
        assert_eq!(word_at("a.b_c", 3).unwrap().0, "b_c");
        assert!(word_at("  ", 1).is_none());
    }
}
