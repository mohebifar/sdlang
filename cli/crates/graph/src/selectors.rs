//! Selectors: their canonical form in the graph and their evaluation over declarations
//! (docs/extensions.md, Rules and selectors).

use sdlang_syntax::ast::{Access, CompareOp, Operand, Predicate};
use serde_json::Value;

/// The predicate in canonical source form: single spaces, and every `and`, `or` and
/// `not` group below the top level parenthesised.
pub fn canonical_where(predicate: &Predicate) -> String {
    render(predicate, true)
}

fn render(p: &Predicate, top: bool) -> String {
    let wrap = |s: String| if top { s } else { format!("({s})") };
    match p {
        Predicate::Or { terms } => wrap(join(terms, " or ")),
        Predicate::And { terms } => wrap(join(terms, " and ")),
        Predicate::Not { inner } => wrap(format!("not {}", render(inner, false))),
        Predicate::Exists { access } => format!("exists {}", access_text(access)),
        Predicate::Compare {
            access,
            op,
            operand,
        } => {
            format!(
                "{} {} {}",
                access_text(access),
                op_text(*op),
                operand_text(operand)
            )
        }
    }
}

fn join(terms: &[Predicate], sep: &str) -> String {
    terms
        .iter()
        .map(|t| render(t, false))
        .collect::<Vec<_>>()
        .join(sep)
}

fn access_text(a: &Access) -> String {
    match a {
        Access::Meta { field } => format!("meta.{}", field.text),
        Access::Field { path } => path
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>()
            .join("."),
    }
}

fn op_text(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
        CompareOp::Matches => "matches",
        CompareOp::In => "in",
        CompareOp::Contains => "contains",
    }
}

fn operand_text(o: &Operand) -> String {
    match o {
        Operand::String(s) => {
            let mut out = String::new();
            out.push('"');
            for c in s.to_source_text().chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Operand::Number { text } => text.clone(),
        Operand::Bool { value } => value.to_string(),
        Operand::Null => "null".into(),
        Operand::Path(p) => p.dotted(),
        Operand::List { items } => {
            format!(
                "[{}]",
                items
                    .iter()
                    .map(operand_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// What a predicate can see of a candidate declaration.
pub struct Candidate<'a> {
    pub module: &'a str,
    pub name: &'a str,
    pub kind: &'a str,
    /// The candidate's lowered fields.
    pub fields: &'a serde_json::Map<String, Value>,
}

/// Evaluates a predicate. `operand_value` turns an operand into the JSON value it is
/// compared with (paths become references).
pub fn matches(p: &Predicate, c: &Candidate, operand_value: &dyn Fn(&Operand) -> Value) -> bool {
    match p {
        Predicate::Or { terms } => terms.iter().any(|t| matches(t, c, operand_value)),
        Predicate::And { terms } => terms.iter().all(|t| matches(t, c, operand_value)),
        Predicate::Not { inner } => !matches(inner, c, operand_value),
        Predicate::Exists { access } => read(access, c).is_some(),
        Predicate::Compare {
            access,
            op,
            operand,
        } => {
            let Some(value) = read(access, c) else {
                return false;
            };
            let value = plain(&value);
            let rhs = plain(&operand_value(operand));
            match op {
                CompareOp::Eq => value == rhs,
                CompareOp::Ne => value != rhs,
                CompareOp::Lt | CompareOp::Le | CompareOp::Gt | CompareOp::Ge => {
                    match (number(&value), number(&rhs)) {
                        (Some(a), Some(b)) => match op {
                            CompareOp::Lt => a < b,
                            CompareOp::Le => a <= b,
                            CompareOp::Gt => a > b,
                            _ => a >= b,
                        },
                        _ => false,
                    }
                }
                CompareOp::Matches => match (value.as_str(), rhs.as_str()) {
                    (Some(s), Some(pattern)) => glob(pattern, s),
                    _ => false,
                },
                CompareOp::In => rhs.as_array().is_some_and(|items| items.contains(&value)),
                CompareOp::Contains => value.as_array().is_some_and(|items| items.contains(&rhs)),
            }
        }
    }
}

fn read(access: &Access, c: &Candidate) -> Option<Value> {
    match access {
        Access::Meta { field } => Some(Value::String(
            match field.text.as_str() {
                "module" => c.module,
                "name" => c.name,
                _ => c.kind,
            }
            .to_string(),
        )),
        Access::Field { path } => {
            let mut value = c.fields.get(&path[0].text)?.clone();
            for seg in &path[1..] {
                value = plain(&value).get(&seg.text)?.clone();
            }
            Some(value)
        }
    }
}

/// Strips the graph's tags that do not matter to comparison: prose, records and refs.
fn plain(v: &Value) -> Value {
    match v {
        Value::Object(m) if m.len() == 1 => {
            if let Some(t) = m.get("$prose") {
                return t.clone();
            }
            if let Some(r) = m.get("$record") {
                return r.clone();
            }
            if let Some(r) = m.get("$ref") {
                return r.clone();
            }
            v.clone()
        }
        Value::Array(items) => Value::Array(items.iter().map(plain).collect()),
        _ => v.clone(),
    }
}

fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Full-string, case-sensitive glob over `/`-separated segments: `*` matches within one
/// segment, `**` matches zero or more whole segments.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<&str> = pattern.split('/').collect();
    let t: Vec<&str> = text.split('/').collect();
    segments(&p, &t)
}

fn segments(p: &[&str], t: &[&str]) -> bool {
    match p.split_first() {
        None => t.is_empty(),
        Some((&"**", rest)) => (0..=t.len()).any(|skip| segments(rest, &t[skip..])),
        Some((first, rest)) => match t.split_first() {
            Some((seg, trest)) => {
                segment(first.as_bytes(), seg.as_bytes()) && segments(rest, trest)
            }
            None => false,
        },
    }
}

fn segment(p: &[u8], t: &[u8]) -> bool {
    match p.split_first() {
        None => t.is_empty(),
        Some((b'*', rest)) => (0..=t.len()).any(|skip| segment(rest, &t[skip..])),
        Some((c, rest)) => t.first() == Some(c) && segment(rest, &t[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdlang_syntax::ast::{Exportable, Expr, MemberValue, Statement};

    fn predicate(src: &str) -> Predicate {
        let file = sdlang_syntax::parse(&format!(
            "rule r {{\n  applies_to: select * where {src}\n}}\n"
        ))
        .unwrap();
        let Statement::Decl(d) = &file.statements[0] else {
            panic!()
        };
        let Exportable::Named(n) = &d.item else {
            panic!()
        };
        let MemberValue::Field(f) = &n.body.members[0].value else {
            panic!()
        };
        let Expr::Selector(s) = &f.value else {
            panic!()
        };
        s.predicate.clone().unwrap()
    }

    #[test]
    fn canonical_form() {
        assert_eq!(
            canonical_where(&predicate("route  matches \"/app/**\"")),
            "route matches \"/app/**\""
        );
        assert_eq!(
            canonical_where(&predicate("a == 1 and not exists b or c in [1, 2]")),
            "(a == 1 and (not exists b)) or c in [1, 2]"
        );
    }

    #[test]
    fn route_glob() {
        assert!(glob("/app/**", "/app"));
        assert!(glob("/app/**", "/app/settings/billing"));
        assert!(!glob("/app/**", "/application"));
        assert!(glob("/app/*", "/app/x"));
        assert!(!glob("/app/*", "/app/x/y"));
        assert!(glob("/a*c", "/abc"));
    }

    #[test]
    fn evaluation() {
        let mut fields = serde_json::Map::new();
        fields.insert("route".into(), Value::String("/app".into()));
        let c = Candidate {
            module: "dashboard",
            name: "home",
            kind: "k",
            fields: &fields,
        };
        let operand = |o: &Operand| match o {
            Operand::String(s) => Value::String(s.to_source_text()),
            _ => Value::Null,
        };
        assert!(matches(
            &predicate("route matches \"/app/**\""),
            &c,
            &operand
        ));
        assert!(matches(
            &predicate("meta.module == \"dashboard\""),
            &c,
            &operand
        ));
        assert!(!matches(&predicate("exists layout"), &c, &operand));
        assert!(matches(&predicate("not exists layout"), &c, &operand));
    }
}
