//! Canonical JSON text and sha256 digests (SPEC.md, "Projections and hashes").
//!
//! No insignificant whitespace; UTF-8; every string and key NFC-normalised; keys sorted
//! by Unicode code point, and two keys equal after NFC are an error; integers and
//! integral floats in plain decimal with no exponent; other numbers as JavaScript's
//! `Number.prototype.toString` writes them; non-finite numbers rejected.
//!
//! Numbers are read from their JSON text (serde_json's `arbitrary_precision`), so an
//! integer of any size is written exactly.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Number, Value};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonError {
    /// A number that does not fit a finite double, with its JSON text.
    NonFinite(String),
    /// Two object keys that are equal after NFC normalisation.
    DuplicateKey(String),
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonError::NonFinite(text) => write!(f, "non-finite number in canonical JSON: {text}"),
            CanonError::DuplicateKey(key) => {
                write!(f, "duplicate key after NFC normalisation: {key:?}")
            }
        }
    }
}

impl std::error::Error for CanonError {}

/// The canonical JSON text of a value.
pub fn canonical(value: &Value) -> Result<String, CanonError> {
    let mut out = String::new();
    encode(value, &mut out)?;
    Ok(out)
}

/// `sha256:` followed by the lowercase hex sha256 of the value's canonical text.
pub fn digest(value: &Value) -> Result<String, CanonError> {
    Ok(digest_text(&canonical(value)?))
}

/// `sha256:` followed by the lowercase hex sha256 of `text`'s UTF-8 bytes.
pub fn digest_text(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for byte in hash {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

fn encode(value: &Value, out: &mut String) -> Result<(), CanonError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(n)?),
        Value::String(s) => string(&nfc(s), out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // String order is byte order of UTF-8, which is Unicode code point order.
            let mut sorted = BTreeMap::new();
            for (key, item) in map {
                let key = nfc(key);
                if sorted.contains_key(&key) {
                    return Err(CanonError::DuplicateKey(key));
                }
                sorted.insert(key, item);
            }
            out.push('{');
            for (i, (key, item)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(&key, out);
                out.push(':');
                encode(item, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// JSON string escaping as Python's `json.dumps(..., ensure_ascii=False)` does it: the
/// quote, the backslash and control characters only.
fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn is_integer_literal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn number(n: &Number) -> Result<String, CanonError> {
    let text = n.to_string();
    if is_integer_literal(&text) {
        let digits = text.strip_prefix('-').unwrap_or(&text);
        return Ok(if digits.bytes().all(|b| b == b'0') {
            "0".to_string()
        } else {
            text
        });
    }
    let x: f64 = text
        .parse()
        .map_err(|_| CanonError::NonFinite(text.clone()))?;
    if !x.is_finite() {
        return Err(CanonError::NonFinite(text));
    }
    Ok(format_f64(x))
}

/// A finite double: integral values as exact plain decimal, others as JavaScript writes them.
pub fn format_f64(x: f64) -> String {
    if x.fract() == 0.0 {
        integral_digits(x)
    } else {
        js_number(x)
    }
}

/// The exact decimal digits of an integral double.
fn integral_digits(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let a = x.abs();
    if a < 9_007_199_254_740_992.0 {
        return format!("{sign}{}", a as u64);
    }
    // a = mantissa * 2^shift with shift > 0; multiply out in base 1e9.
    let bits = a.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let shift = exponent - 1075;
    const BASE: u64 = 1_000_000_000;
    let mut limbs: Vec<u64> = vec![
        mantissa % BASE,
        mantissa / BASE % BASE,
        mantissa / BASE / BASE,
    ];
    for _ in 0..shift {
        let mut carry = 0;
        for limb in limbs.iter_mut() {
            let v = *limb * 2 + carry;
            *limb = v % BASE;
            carry = v / BASE;
        }
        if carry > 0 {
            limbs.push(carry);
        }
    }
    while limbs.len() > 1 && *limbs.last().unwrap_or(&1) == 0 {
        limbs.pop();
    }
    let mut out = String::from(sign);
    let mut iter = limbs.iter().rev();
    if let Some(top) = iter.next() {
        out.push_str(&top.to_string());
    }
    for limb in iter {
        out.push_str(&format!("{limb:09}"));
    }
    out
}

/// Number::toString from ECMA-262 for a finite, non-integral double.
fn js_number(x: f64) -> String {
    let sign = if x < 0.0 { "-" } else { "" };
    // Rust's `{:e}` prints the shortest digits that round-trip.
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').expect("{:e} has an exponent");
    let exp: i32 = exp.parse().expect("integer exponent");
    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    let k = digits.len() as i32;
    let n = exp + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let fraction = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        let e_sign = if e > 0 { "+" } else { "-" };
        format!("{}{fraction}e{e_sign}{}", &digits[..1], e.abs())
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn canon(text: &str) -> String {
        canonical(&serde_json::from_str(text).unwrap()).unwrap()
    }

    #[test]
    fn sorted_keys_by_code_point() {
        assert_eq!(canon(r#"{"b":1,"a":2}"#), r#"{"a":2,"b":1}"#);
        assert_eq!(
            canon("{\"\u{1F600}\":1,\"\u{FFFD}\":2}"),
            "{\"\u{FFFD}\":2,\"\u{1F600}\":1}"
        );
    }

    #[test]
    fn duplicate_keys_after_nfc() {
        let value = json!({ "e\u{301}": 1, "\u{e9}": 2 });
        assert!(matches!(
            canonical(&value),
            Err(CanonError::DuplicateKey(_))
        ));
    }

    #[test]
    fn nfc_strings() {
        assert_eq!(canonical(&json!("e\u{301}")).unwrap(), "\"\u{e9}\"");
    }

    #[test]
    fn escapes() {
        assert_eq!(
            canonical(&json!("\u{1}\n\t\"\\/\u{7f}")).unwrap(),
            "\"\\u0001\\n\\t\\\"\\\\/\u{7f}\""
        );
    }

    #[test]
    fn integers() {
        assert_eq!(canon("2.0"), "2");
        assert_eq!(canon("-0"), "0");
        assert_eq!(canon("-0.0"), "0");
        assert_eq!(canon("1E2"), "100");
        assert_eq!(canon("1e23"), "99999999999999991611392");
        assert_eq!(canon("12345678901234567890123"), "12345678901234567890123");
        assert_eq!(canon("-9007199254740993.0"), "-9007199254740992");
    }

    #[test]
    fn other_numbers_as_javascript() {
        assert_eq!(canon("1.5"), "1.5");
        assert_eq!(canon("0.1"), "0.1");
        assert_eq!(canon("123.456"), "123.456");
        assert_eq!(canon("0.00001"), "0.00001");
        assert_eq!(canon("0.000001"), "0.000001");
        assert_eq!(canon("1e-7"), "1e-7");
        assert_eq!(canon("-1.5e-7"), "-1.5e-7");
        assert_eq!(canon("5e-324"), "5e-324");
    }

    #[test]
    fn non_finite() {
        assert!(matches!(
            canonical(&serde_json::from_str("1e999").unwrap()),
            Err(CanonError::NonFinite(_))
        ));
    }

    #[test]
    fn digest_format() {
        let d = digest(&json!({"b": 1, "a": [true, null, "x"]})).unwrap();
        assert_eq!(
            d,
            "sha256:54a65415ad370228851a1da4b31b6fd42dc58b19a50d35cae759325f7388ce64"
        );
    }
}
