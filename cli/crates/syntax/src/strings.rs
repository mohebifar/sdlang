//! String literals: parts, escapes, mentions, interpolations and prose dedent.
//!
//! Scanning follows GRAMMAR.ebnf's ordered state machine. Inside a literal, at each
//! position: the closing delimiter (handled by the lexer), a literal opener (`@@{`,
//! `$${`), a mention (`@{path}`) or interpolation (`${word}`), an escape, and finally
//! one character of text.

use serde::Serialize;

use crate::ast::Span;
use crate::lexer::is_keyword;

/// Which delimiter a literal used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StringKind {
    /// `"..."`: one line, backslash escapes.
    Line,
    /// `"""..."""`: may span lines; block literals are dedented.
    Prose,
}

/// One piece of a string literal after scanning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Part {
    /// Literal text, with escapes and literal openers already resolved.
    Text(String),
    /// `@{path}`: a reference to a declaration, or `self`.
    Mention(String),
    /// `${word}` or `${word.word}`: a template field.
    Interp(String),
}

/// A scanned string literal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StringLit {
    pub kind: StringKind,
    pub parts: Vec<Part>,
    pub span: Span,
}

impl StringLit {
    /// The literal's text with mentions and interpolations written back in source form.
    pub fn to_source_text(&self) -> String {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(t) => out.push_str(t),
                Part::Mention(m) => {
                    out.push_str("@{");
                    out.push_str(m);
                    out.push('}');
                }
                Part::Interp(i) => {
                    out.push_str("${");
                    out.push_str(i);
                    out.push('}');
                }
            }
        }
        out
    }
}

/// A scanning error. `offset` is a byte offset into the text that was scanned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringError {
    pub message: String,
    pub offset: usize,
}

fn err(message: impl Into<String>, offset: usize) -> StringError {
    StringError {
        message: message.into(),
        offset,
    }
}

/// Scans the body of a line string (the text between the quotes).
pub fn scan_line(body: &str) -> Result<Vec<Part>, StringError> {
    if let Some(i) = body.find('\n') {
        return Err(err("line break inside a line string", i));
    }
    scan_parts(body, false)
}

/// Scans the body of a prose string (the text between the triple quotes),
/// dedenting it first if it is a block literal.
pub fn scan_prose(raw: &str) -> Result<Vec<Part>, StringError> {
    let text = dedent(raw)?;
    // Offsets after dedent no longer match the source; report errors at the start.
    scan_parts(&text, true).map_err(|e| err(e.message, 0))
}

fn is_word(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn valid_mention(inner: &str) -> bool {
    if inner == "self" {
        return true;
    }
    let mut segments = inner.split('.');
    let first = segments.next().unwrap_or("");
    is_word(first) && !is_keyword(first) && segments.all(is_word)
}

fn valid_interp(inner: &str) -> bool {
    let segments: Vec<&str> = inner.split('.').collect();
    (1..=2).contains(&segments.len()) && segments.iter().all(|s| is_word(s))
}

fn scan_parts(body: &str, prose: bool) -> Result<Vec<Part>, StringError> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < body.len() {
        let rest = &body[i..];
        if rest.starts_with("@@{") || rest.starts_with("$${") {
            text.push_str(&rest[1..3]);
            i += 3;
            continue;
        }
        if rest.starts_with("@{") || rest.starts_with("${") {
            let mention = rest.starts_with('@');
            let close = rest.find('}');
            let inner = close.map(|c| &rest[2..c]).unwrap_or("");
            let ok = close.is_some()
                && if mention {
                    valid_mention(inner)
                } else {
                    valid_interp(inner)
                };
            if !ok {
                let what = if mention { "mention" } else { "interpolation" };
                return Err(err(
                    format!("malformed {what} '{}...}}' in string", &rest[..2]),
                    i,
                ));
            }
            if !text.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut text)));
            }
            parts.push(if mention {
                Part::Mention(inner.to_string())
            } else {
                Part::Interp(inner.to_string())
            });
            i += close.unwrap_or(0) + 1;
            continue;
        }
        if !prose && rest.starts_with('\\') {
            let next = rest[1..].chars().next();
            match next {
                Some(c @ ('"' | '\\')) => {
                    text.push(c);
                    i += 2;
                }
                Some('n') => {
                    text.push('\n');
                    i += 2;
                }
                Some('t') => {
                    text.push('\t');
                    i += 2;
                }
                Some('u') if rest[2..].starts_with('{') => {
                    let hex_end = rest[3..]
                        .find(|c: char| !c.is_ascii_hexdigit())
                        .map(|n| n + 3);
                    let Some(end) = hex_end.filter(|&e| e > 3 && rest[e..].starts_with('}')) else {
                        return Err(err("bad escape \\u", i));
                    };
                    let hex = &rest[3..end];
                    let scalar = u32::from_str_radix(hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| {
                            err(format!("\\u{{{hex}}} is not a Unicode scalar value"), i)
                        })?;
                    text.push(scalar);
                    i += end + 1;
                }
                other => {
                    let shown = other.map(String::from).unwrap_or_default();
                    return Err(err(format!("bad escape \\{shown}"), i));
                }
            }
            continue;
        }
        if prose && rest.starts_with("\\\"\"\"") {
            text.push_str("\"\"\"");
            i += 4;
            continue;
        }
        let c = rest.chars().next().expect("non-empty rest");
        text.push(c);
        i += c.len_utf8();
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

/// Prose dedent per GRAMMAR.ebnf. A literal whose opening delimiter is directly
/// followed by a line break is a block literal; any other is returned unchanged.
pub fn dedent(raw: &str) -> Result<String, StringError> {
    let Some(body) = raw.strip_prefix('\n') else {
        return Ok(raw.to_string());
    };
    let lines: Vec<&str> = body.split('\n').collect();
    let (closing, content) = lines.split_last().expect("split yields at least one line");
    if !closing.trim_matches([' ', '\t']).is_empty() {
        return Err(err(
            "the closing \"\"\" of a block literal must be on its own line",
            raw.len(),
        ));
    }
    for line in content.iter().chain(std::iter::once(closing)) {
        let lead_len = line.len() - line.trim_start_matches([' ', '\t']).len();
        if line[..lead_len].contains('\t') {
            return Err(err("tab in prose indentation", 0));
        }
    }
    let indent = content
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    let out: Vec<&str> = content
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                ""
            } else {
                &l[indent..]
            }
        })
        .collect();
    let mut joined = out.join("\n");
    if !out.is_empty() {
        joined.push('\n');
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Part {
        Part::Text(s.to_string())
    }

    #[test]
    fn order_tries_openers_before_mentions() {
        let parts = scan_line("a @{login} b ${meta.name}").unwrap();
        assert_eq!(
            parts,
            vec![
                text("a "),
                Part::Mention("login".into()),
                text(" b "),
                Part::Interp("meta.name".into()),
            ]
        );
    }

    #[test]
    fn openers_produce_literal_text() {
        let parts = scan_line("write @@{x} and $${y} and a@b.com").unwrap();
        assert_eq!(parts, vec![text("write @{x} and ${y} and a@b.com")]);
    }

    #[test]
    fn bad_mention_is_an_error() {
        assert!(scan_line("go to @{ login }").is_err());
        assert!(
            scan_line("go to @{type}").is_err(),
            "first segment is a keyword"
        );
        assert!(scan_line("go to @{login").is_err());
        assert!(scan_line("@{self}").is_ok());
        assert!(scan_line("@{auth.login}").is_ok());
    }

    #[test]
    fn escapes() {
        assert_eq!(
            scan_line(r#"a\"b\\c\nd\te"#).unwrap(),
            vec![text("a\"b\\c\nd\te")]
        );
        assert_eq!(scan_line(r"\u{1F600}").unwrap(), vec![text("\u{1F600}")]);
        assert!(scan_line(r"\q").is_err());
        assert_eq!(
            scan_prose(r#"say \""" here"#).unwrap(),
            vec![text("say \"\"\" here")]
        );
        assert_eq!(
            scan_prose(r"keep \n as is").unwrap(),
            vec![text(r"keep \n as is")]
        );
    }

    #[test]
    fn scalar_values_only() {
        assert!(scan_line(r"\u{D800}").is_err());
        assert!(scan_line(r"\u{110000}").is_err());
    }

    #[test]
    fn raw_newline_in_line_string() {
        assert!(scan_line("a\nb").is_err());
    }

    #[test]
    fn dedent_block_literal() {
        let raw = "\n    One.\n\n      Two.\n  ";
        assert_eq!(dedent(raw).unwrap(), "One.\n\n  Two.\n");
    }

    #[test]
    fn inline_prose_not_dedented() {
        assert_eq!(dedent("  like this  ").unwrap(), "  like this  ");
    }

    #[test]
    fn tab_indentation_is_an_error() {
        assert!(dedent("\n\ttext\n  ").is_err());
    }

    #[test]
    fn closing_must_be_on_own_line() {
        assert!(dedent("\n    text").is_err());
    }
}
