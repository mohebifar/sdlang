//! Compatibility of a change to a consumed public projection, computed from the old and
//! new canonical documents without package code (SPEC.md, Packages -> Compatibility;
//! EXTENSIONS.md, Entity changes).

use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Compatibility {
    /// Dependents get `verify`; the label names what was compatible.
    Compatible(&'static str),
    /// Dependents get `update`.
    Incompatible,
}

/// Classifies the change from `old` to `new` (public projection documents) for a
/// dependent that reaches the target over `relation`.
pub fn classify(old: &Value, new: &Value, relation: &str) -> Compatibility {
    use Compatibility::*;
    if old == new {
        return Compatible("unchanged");
    }
    let same = |key: &str| old.get(key) == new.get(key);
    // Prose and annotations cannot be compared, and a kind change is a different contract.
    if !same("kind") || !same("summary") || !same("annotations") {
        return Incompatible;
    }
    let ok = match old["kind"].as_str().unwrap_or_default() {
        "core:action" => action(old, new),
        "core:lib" => lib(old, new),
        "core:entity" => return entity(old, new, relation),
        _ => false,
    };
    if ok {
        Compatible("compatible")
    } else {
        Incompatible
    }
}

fn empty() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn object<'a>(v: &'a Value, key: &str) -> &'a Map<String, Value> {
    v.get(key)
        .and_then(Value::as_object)
        .unwrap_or_else(|| empty())
}

fn action(old: &Value, new: &Value) -> bool {
    let (oi, ni) = (object(old, "interface"), object(new, "interface"));
    if oi.keys().any(|k| k != "input" && k != "output")
        || ni.keys().any(|k| k != "input" && k != "output")
    {
        return false;
    }
    let null = Value::Null;
    let get = |m: &Map<String, Value>, k: &str| m.get(k).cloned().unwrap_or(null.clone());
    // The new input accepts every old input; the new output is one of the old outputs.
    let input = subtype(&get(oi, "input"), &get(ni, "input"));
    let output = subtype(&get(ni, "output"), &get(oi, "output"));
    let (om, nm) = (object(old, "members"), object(new, "members"));
    if om.keys().any(|k| k != "errors") || nm.keys().any(|k| k != "errors") {
        return false;
    }
    let codes = |m: &Map<String, Value>| -> Vec<Value> {
        m.get("errors")
            .and_then(Value::as_object)
            .map(|e| e.values().cloned().collect())
            .unwrap_or_default()
    };
    let (old_codes, new_codes) = (codes(om), codes(nm));
    let errors = new_codes.iter().all(|c| old_codes.contains(c));
    input && output && errors
}

fn lib(old: &Value, new: &Value) -> bool {
    let (oi, ni) = (object(old, "interface"), object(new, "interface"));
    if oi.keys().any(|k| k != "exports")
        || ni.keys().any(|k| k != "exports")
        || !(object(old, "members").is_empty() && object(new, "members").is_empty())
    {
        return false;
    }
    let names = |m: &Map<String, Value>| -> Vec<Value> {
        m.get("exports")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (old_names, new_names) = (names(oi), names(ni));
    old_names.iter().all(|n| new_names.contains(n))
}

/// Whether every value of type `a` is a value of type `b`.
pub fn subtype(a: &Value, b: &Value) -> bool {
    if a == b {
        return true;
    }
    let tag = |v: &Value| v.get("$type").and_then(Value::as_str).map(str::to_string);
    match (tag(a).as_deref(), tag(b).as_deref()) {
        (Some("optional"), Some("optional")) => subtype(&a["of"], &b["of"]),
        (Some("optional"), _) => false,
        (_, Some("optional")) => subtype(a, &b["of"]),
        (Some("union"), _) => a["of"]
            .as_array()
            .is_some_and(|xs| xs.iter().all(|x| subtype(x, b))),
        (_, Some("union")) => b["of"]
            .as_array()
            .is_some_and(|ys| ys.iter().any(|y| subtype(a, y))),
        (Some("list"), Some("list")) => subtype(&a["of"], &b["of"]),
        (Some("record"), Some("record")) => {
            let (af, bf) = (object(a, "fields"), object(b, "fields"));
            // Open for width: `a` may have more fields; each of `b`'s must be satisfied.
            bf.iter().all(|(k, tb)| match af.get(k) {
                Some(ta) => subtype(ta, tb),
                None => tag(tb).as_deref() == Some("optional"),
            })
        }
        _ => false,
    }
}

struct EntityCompat {
    readers: bool,
    writers: bool,
}

fn entity(old: &Value, new: &Value, relation: &str) -> Compatibility {
    let mut c = EntityCompat {
        readers: true,
        writers: true,
    };
    if object(old, "interface") != object(new, "interface") {
        return Compatibility::Incompatible;
    }
    let (om, nm) = (object(old, "members"), object(new, "members"));
    if om.keys().any(|k| k != "fields") || nm.keys().any(|k| k != "fields") {
        return Compatibility::Incompatible;
    }
    let old_fields = om
        .get("fields")
        .and_then(Value::as_object)
        .unwrap_or_else(|| empty());
    let new_fields = nm
        .get("fields")
        .and_then(Value::as_object)
        .unwrap_or_else(|| empty());
    for (name, o) in old_fields {
        match new_fields.get(name) {
            None => {
                c.readers = false;
                c.writers = false;
            }
            Some(n) => field_change(o, n, &mut c),
        }
    }
    for (name, n) in new_fields {
        if old_fields.contains_key(name) {
            continue;
        }
        let optional = n["type"].get("$type").and_then(Value::as_str) == Some("optional");
        if !optional && n.get("default").is_none() {
            c.writers = false;
        }
    }
    let ok = match relation {
        "reads" => c.readers,
        "writes" => c.writers,
        _ => c.readers && c.writers,
    };
    match (ok, relation) {
        (false, _) => Compatibility::Incompatible,
        (true, "reads") => Compatibility::Compatible("reader-compatible"),
        (true, "writes") => Compatibility::Compatible("writer-compatible"),
        (true, _) => Compatibility::Compatible("compatible"),
    }
}

fn field_change(old: &Value, new: &Value, c: &mut EntityCompat) {
    if old == new {
        return;
    }
    let rest = |v: &Value| -> Map<String, Value> {
        v.as_object()
            .map(|m| {
                m.iter()
                    .filter(|(k, _)| *k != "type")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };
    if rest(old) != rest(new) {
        c.readers = false;
        c.writers = false;
        return;
    }
    let (ot, nt) = (&old["type"], &new["type"]);
    let optional_of = |v: &Value| -> Option<Value> {
        (v.get("$type").and_then(Value::as_str) == Some("optional")).then(|| v["of"].clone())
    };
    if optional_of(nt).as_ref() == Some(ot) {
        // Made optional: readers may now see nothing.
        c.readers = false;
    } else if optional_of(ot).as_ref() == Some(nt) {
        // Made required: existing writers may omit it.
        c.writers = false;
    } else {
        c.readers = false;
        c.writers = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn b(name: &str) -> Value {
        json!({ "$type": "builtin", "name": name })
    }

    fn opt(v: Value) -> Value {
        json!({ "$type": "optional", "of": v })
    }

    fn record(fields: Value) -> Value {
        json!({ "$type": "record", "fields": fields })
    }

    fn action(input: Value, output: Value, errors: Value) -> Value {
        json!({ "kind": "core:action", "summary": null, "annotations": [],
                "interface": { "input": input, "output": output },
                "members": { "errors": errors } })
    }

    #[test]
    fn action_variance() {
        let base = action(
            record(json!({ "email": b("email") })),
            record(json!({ "sent": b("boolean") })),
            json!({ "rate_limited": "rate_limited" }),
        );
        let wider_output = action(
            record(json!({ "email": b("email") })),
            record(json!({ "sent": b("boolean"), "id": opt(b("uuid")) })),
            json!({ "rate_limited": "rate_limited" }),
        );
        assert!(matches!(
            classify(&base, &wider_output, "calls"),
            Compatibility::Compatible(_)
        ));
        let optional_output = action(
            record(json!({ "email": b("email") })),
            record(json!({ "sent": opt(b("boolean")) })),
            json!({ "rate_limited": "rate_limited" }),
        );
        assert_eq!(
            classify(&base, &optional_output, "calls"),
            Compatibility::Incompatible
        );
        let new_required_input = action(
            record(json!({ "email": b("email"), "x": b("string") })),
            record(json!({ "sent": b("boolean") })),
            json!({ "rate_limited": "rate_limited" }),
        );
        assert_eq!(
            classify(&base, &new_required_input, "calls"),
            Compatibility::Incompatible
        );
        let fewer_errors = action(
            record(json!({ "email": b("email") })),
            record(json!({ "sent": b("boolean") })),
            json!({}),
        );
        assert!(matches!(
            classify(&base, &fewer_errors, "calls"),
            Compatibility::Compatible(_)
        ));
        assert_eq!(
            classify(&fewer_errors, &base, "calls"),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn lib_exports() {
        let lib = |names: Value| json!({ "kind": "core:lib", "summary": null, "annotations": [], "interface": { "exports": names }, "members": {} });
        assert!(matches!(
            classify(&lib(json!(["a"])), &lib(json!(["a", "b"])), "uses"),
            Compatibility::Compatible(_)
        ));
        assert_eq!(
            classify(&lib(json!(["a", "b"])), &lib(json!(["a"])), "uses"),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn package_interface_is_invariant() {
        let page = |route: &str| json!({ "kind": "npm:x::page", "summary": null, "annotations": [], "interface": { "route": route }, "members": {} });
        assert_eq!(
            classify(&page("/a"), &page("/b"), "uses"),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn entities() {
        let entity = |fields: Value| json!({ "kind": "core:entity", "summary": "s", "annotations": [], "interface": {}, "members": { "fields": fields } });
        let base = entity(json!({ "id": { "type": b("uuid") } }));
        let optional_added =
            entity(json!({ "id": { "type": b("uuid") }, "at": { "type": opt(b("timestamp")) } }));
        assert_eq!(
            classify(&base, &optional_added, "reads"),
            Compatibility::Compatible("reader-compatible")
        );
        assert_eq!(
            classify(&base, &optional_added, "writes"),
            Compatibility::Compatible("writer-compatible")
        );
        let required_added =
            entity(json!({ "id": { "type": b("uuid") }, "n": { "type": b("string") } }));
        assert_eq!(
            classify(&base, &required_added, "writes"),
            Compatibility::Incompatible
        );
        assert!(matches!(
            classify(&base, &required_added, "reads"),
            Compatibility::Compatible(_)
        ));
        let made_optional = entity(json!({ "id": { "type": opt(b("uuid")) } }));
        assert_eq!(
            classify(&base, &made_optional, "reads"),
            Compatibility::Incompatible
        );
        assert!(matches!(
            classify(&base, &made_optional, "writes"),
            Compatibility::Compatible(_)
        ));
    }

    #[test]
    fn prose_is_incompatible() {
        let lib = |s: &str| json!({ "kind": "core:lib", "summary": s, "annotations": [], "interface": {}, "members": {} });
        assert_eq!(
            classify(&lib("a"), &lib("b"), "uses"),
            Compatibility::Incompatible
        );
    }
}
