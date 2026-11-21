//! Tokens for `.int` source, produced with logos.
//!
//! Line breaks are tokens (the grammar accepts them only where it says so); spaces,
//! tabs and `//` comments are skipped. A string literal is one token carrying its
//! scanned parts. CRLF must be normalised to LF before lexing (see [`normalize`]).

use std::borrow::Cow;

use logos::Logos;
use serde::Serialize;

use crate::ast::Span;
use crate::parser::ParseError;
use crate::strings::{self, StringKind, StringLit};

/// Words reserved where the grammar requires an identifier.
pub const KEYWORDS: &[&str] = &[
    "import",
    "export",
    "extern",
    "use",
    "for",
    "as",
    "moved",
    "let",
    "type",
    "kind",
    "annotation",
    "implements",
    "project",
    "stack",
    "module",
    "package",
    "ref",
    "self",
    "select",
    "where",
    "and",
    "or",
    "not",
    "exists",
    "meta",
    "matches",
    "in",
    "contains",
    "true",
    "false",
    "null",
];

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

/// Replaces CRLF with LF. Spans always refer to the normalised text.
pub fn normalize(src: &str) -> Cow<'_, str> {
    if src.contains("\r\n") {
        Cow::Owned(src.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(src)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Punct {
    Arrow,
    EqEq,
    NotEq,
    Le,
    Ge,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Colon,
    Eq,
    Pipe,
    Question,
    Dot,
    At,
    Star,
    Hash,
    Lt,
    Gt,
}

impl Punct {
    pub fn as_str(self) -> &'static str {
        match self {
            Punct::Arrow => "->",
            Punct::EqEq => "==",
            Punct::NotEq => "!=",
            Punct::Le => "<=",
            Punct::Ge => ">=",
            Punct::LBrace => "{",
            Punct::RBrace => "}",
            Punct::LBracket => "[",
            Punct::RBracket => "]",
            Punct::LParen => "(",
            Punct::RParen => ")",
            Punct::Comma => ",",
            Punct::Colon => ":",
            Punct::Eq => "=",
            Punct::Pipe => "|",
            Punct::Question => "?",
            Punct::Dot => ".",
            Punct::At => "@",
            Punct::Star => "*",
            Punct::Hash => "#",
            Punct::Lt => "<",
            Punct::Gt => ">",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Newline,
    Word(String),
    Number(String),
    Str(StringLit),
    Punct(Punct),
    Eof,
}

impl TokenKind {
    /// How the token is shown in an error message.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Newline => "a line break".into(),
            TokenKind::Word(w) => format!("'{w}'"),
            TokenKind::Number(n) => format!("number {n}"),
            TokenKind::Str(s) => match s.kind {
                StringKind::Line => "a string".into(),
                StringKind::Prose => "a prose string".into(),
            },
            TokenKind::Punct(p) => format!("'{}'", p.as_str()),
            TokenKind::Eof => "the end of the file".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum RawError {
    #[default]
    UnexpectedChar,
    UnterminatedLine,
    UnterminatedProse,
    LineBreakInLine,
}

/// Finds the end of a line string; the opening quote has been consumed.
fn line_string<'s>(lex: &mut logos::Lexer<'s, Raw<'s>>) -> Result<&'s str, RawError> {
    let rest = lex.remainder();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                lex.bump(i + 1);
                return Ok(&rest[..i]);
            }
            '\n' => {
                lex.bump(i);
                return Err(RawError::LineBreakInLine);
            }
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }
    lex.bump(rest.len());
    Err(RawError::UnterminatedLine)
}

/// Finds the end of a prose string; the opening triple quote has been consumed.
fn prose_string<'s>(lex: &mut logos::Lexer<'s, Raw<'s>>) -> Result<&'s str, RawError> {
    let rest = lex.remainder();
    let mut i = 0;
    while i < rest.len() {
        if rest[i..].starts_with("\\\"\"\"") {
            i += 4;
        } else if rest[i..].starts_with("\"\"\"") {
            lex.bump(i + 3);
            return Ok(&rest[..i]);
        } else {
            i += rest[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    lex.bump(rest.len());
    Err(RawError::UnterminatedProse)
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = RawError)]
#[logos(skip r"[ \t]+")]
// A comment runs to the end of the line by definition, so the greedy match is intended.
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
enum Raw<'s> {
    #[token("\n")]
    Newline,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Word,
    #[regex(r"-?[0-9]+(\.[0-9]+)?")]
    Number,
    #[token("\"\"\"", prose_string)]
    Prose(&'s str),
    #[token("\"", line_string)]
    Line(&'s str),
    #[token("->")]
    Arrow,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("=")]
    Eq,
    #[token("|")]
    Pipe,
    #[token("?")]
    Question,
    #[token(".")]
    Dot,
    #[token("@")]
    At,
    #[token("*")]
    Star,
    #[token("#")]
    Hash,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
}

/// Lexes normalised source into tokens, ending with [`TokenKind::Eof`].
pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut lexer = Raw::lexer(src);
    while let Some(result) = lexer.next() {
        let range = lexer.span();
        let span = Span::new(range.start, range.end);
        let raw = match result {
            Ok(raw) => raw,
            Err(e) => {
                let message = match e {
                    RawError::UnexpectedChar => {
                        let c = src[range.start..].chars().next().unwrap_or(' ');
                        format!("unexpected character {c:?}")
                    }
                    RawError::UnterminatedLine => "unterminated string".into(),
                    RawError::UnterminatedProse => "unterminated prose string".into(),
                    RawError::LineBreakInLine => "line break inside a line string".into(),
                };
                return Err(ParseError::at(
                    message,
                    Span::new(span.start, span.start + 1),
                ));
            }
        };
        let text = &src[range.clone()];
        let kind = match raw {
            Raw::Newline => TokenKind::Newline,
            Raw::Word => TokenKind::Word(text.to_string()),
            Raw::Number => TokenKind::Number(text.to_string()),
            Raw::Line(body) => {
                let parts = strings::scan_line(body).map_err(|e| {
                    let at = span.start + 1 + e.offset;
                    ParseError::at(e.message, Span::new(at, at + 1))
                })?;
                TokenKind::Str(StringLit {
                    kind: StringKind::Line,
                    parts,
                    span,
                })
            }
            Raw::Prose(body) => {
                let parts =
                    strings::scan_prose(body).map_err(|e| ParseError::at(e.message, span))?;
                TokenKind::Str(StringLit {
                    kind: StringKind::Prose,
                    parts,
                    span,
                })
            }
            Raw::Arrow => TokenKind::Punct(Punct::Arrow),
            Raw::EqEq => TokenKind::Punct(Punct::EqEq),
            Raw::NotEq => TokenKind::Punct(Punct::NotEq),
            Raw::Le => TokenKind::Punct(Punct::Le),
            Raw::Ge => TokenKind::Punct(Punct::Ge),
            Raw::LBrace => TokenKind::Punct(Punct::LBrace),
            Raw::RBrace => TokenKind::Punct(Punct::RBrace),
            Raw::LBracket => TokenKind::Punct(Punct::LBracket),
            Raw::RBracket => TokenKind::Punct(Punct::RBracket),
            Raw::LParen => TokenKind::Punct(Punct::LParen),
            Raw::RParen => TokenKind::Punct(Punct::RParen),
            Raw::Comma => TokenKind::Punct(Punct::Comma),
            Raw::Colon => TokenKind::Punct(Punct::Colon),
            Raw::Eq => TokenKind::Punct(Punct::Eq),
            Raw::Pipe => TokenKind::Punct(Punct::Pipe),
            Raw::Question => TokenKind::Punct(Punct::Question),
            Raw::Dot => TokenKind::Punct(Punct::Dot),
            Raw::At => TokenKind::Punct(Punct::At),
            Raw::Star => TokenKind::Punct(Punct::Star),
            Raw::Hash => TokenKind::Punct(Punct::Hash),
            Raw::Lt => TokenKind::Punct(Punct::Lt),
            Raw::Gt => TokenKind::Punct(Punct::Gt),
        };
        tokens.push(Token { kind, span });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(src.len(), src.len()),
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    fn word(w: &str) -> TokenKind {
        TokenKind::Word(w.into())
    }

    #[test]
    fn newlines_kept_spaces_and_comments_skipped() {
        assert_eq!(
            kinds("a  b // note\n\tc"),
            vec![
                word("a"),
                word("b"),
                TokenKind::Newline,
                word("c"),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn crlf_normalised_first() {
        let src = normalize("a\r\nb");
        assert_eq!(src, "a\nb");
        assert_eq!(
            kinds(&src),
            vec![word("a"), TokenKind::Newline, word("b"), TokenKind::Eof]
        );
    }

    #[test]
    fn multi_char_punctuation() {
        assert_eq!(
            kinds("-> == != <= >="),
            vec![
                TokenKind::Punct(Punct::Arrow),
                TokenKind::Punct(Punct::EqEq),
                TokenKind::Punct(Punct::NotEq),
                TokenKind::Punct(Punct::Le),
                TokenKind::Punct(Punct::Ge),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            kinds("-1.5"),
            vec![TokenKind::Number("-1.5".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("a->b"),
            vec![
                word("a"),
                TokenKind::Punct(Punct::Arrow),
                word("b"),
                TokenKind::Eof
            ]
        );
        assert!(lex("-x").is_err());
    }

    #[test]
    fn strings_are_one_token() {
        let toks = lex("\"a @{b}\" \"\"\"\n  c\n\"\"\"").unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::Str(s) if s.parts.len() == 2));
        assert!(matches!(&toks[1].kind, TokenKind::Str(s) if s.kind == StringKind::Prose));
        assert_eq!(toks[0].span, Span::new(0, 8));
    }

    #[test]
    fn comment_markers_inside_strings_are_text() {
        let toks = lex("\"http://x\"").unwrap();
        assert_eq!(toks.len(), 2);
    }

    #[test]
    fn string_errors() {
        assert!(lex("\"a\nb\"").is_err());
        assert!(lex("\"abc").is_err());
        assert!(lex("\"\"\"abc").is_err());
    }
}
