//! The projection hashes of every declaration in a canonical graph
//! (`schemas/graph.schema.json`), ported from `tools/canon.py`.
//!
//! Until phase 3 reads interface fields from kind definitions, the interface table is
//! the reference's fixed `KIND_INTERFACE`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::{json, Map, Value};

use crate::canonical_json::{canonical, digest, CanonError};

/// Fields that form the public projection, per kind (the reference's KIND_INTERFACE).
fn interface_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "core:stack" => &["language", "runtime", "framework", "package_manager"],
        "core:use" => &["package", "capability", "options"],
        "core:rule" => &["applies_to"],
        "core:action" => &["input", "output"],
        "core:lib" => &["exports"],
        "npm:@intlang/nextjs::page" => &["route"],
        _ => &[],
    }
}

const BEHAVIOUR_FIELDS: &[&str] = &["prompt"];
const GUIDANCE_FIELDS: &[&str] = &["hint", "skills"];
const SET_FIELDS: &[&str] = &["uses", "calls", "reads", "writes", "requires", "skills"];

/// The five projection hashes of one declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projections {
    pub public: String,
    pub behaviour: String,
    pub body: String,
    pub guidance: String,
    pub context: String,
}

impl Projections {
    pub fn to_value(&self) -> Value {
        json!({
            "public": self.public,
            "behaviour": self.behaviour,
            "body": self.body,
            "guidance": self.guidance,
            "context": self.context,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    Canon(CanonError),
    /// The graph lacks something the projections need.
    Malformed(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::Canon(e) => e.fmt(f),
            GraphError::Malformed(m) => write!(f, "malformed graph: {m}"),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<CanonError> for GraphError {
    fn from(e: CanonError) -> Self {
        GraphError::Canon(e)
    }
}

type GResult<T> = Result<T, GraphError>;

fn malformed<T>(what: impl Into<String>) -> GResult<T> {
    Err(GraphError::Malformed(what.into()))
}

fn obj<'a>(value: &'a Value, key: &str) -> &'a Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    value
        .get(key)
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
}

fn str_field<'a>(value: &'a Value, key: &str) -> GResult<&'a str> {
    match value.get(key).and_then(Value::as_str) {
        Some(s) => Ok(s),
        None => malformed(format!("missing string field '{key}'")),
    }
}

fn get_or_null(map: &Map<String, Value>, key: &str) -> Value {
    map.get(key).cloned().unwrap_or(Value::Null)
}

/// A set-valued array sorted by the canonical JSON of its elements; other values unchanged.
fn sorted_set(value: &Value) -> GResult<Value> {
    let Value::Array(items) = value else {
        return Ok(value.clone());
    };
    let mut keyed = items
        .iter()
        .map(|v| Ok((canonical(v)?, v.clone())))
        .collect::<GResult<Vec<_>>>()?;
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Value::Array(keyed.into_iter().map(|(_, v)| v).collect()))
}

fn field_value(name: &str, value: &Value) -> GResult<Value> {
    if SET_FIELDS.contains(&name) {
        sorted_set(value)
    } else {
        Ok(value.clone())
    }
}

/// Replaces named type references to graph declarations by address plus public hash.
fn resolve_types(value: &Value, hashes: &BTreeMap<String, String>) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| resolve_types(v, hashes)).collect())
        }
        Value::Object(map) => {
            if map.get("$type").and_then(Value::as_str) == Some("named") {
                if let Some(name) = map.get("name").and_then(Value::as_str) {
                    if let Some(hash) = hashes.get(name) {
                        return json!({ "$type": "named", "name": name, "hash": hash });
                    }
                }
            }
            Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), resolve_types(v, hashes)))
                    .collect(),
            )
        }
        other => other.clone(),
    }
}

fn named_refs(value: &Value, acc: &mut BTreeSet<String>) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| named_refs(v, acc)),
        Value::Object(map) => {
            if map.get("$type").and_then(Value::as_str) == Some("named") {
                if let Some(name) = map.get("name").and_then(Value::as_str) {
                    acc.insert(name.to_string());
                }
            }
            map.values().for_each(|v| named_refs(v, acc));
        }
        _ => {}
    }
}

fn raw_public(decl: &Value) -> GResult<Value> {
    let kind = str_field(decl, "kind")?;
    let fields = obj(decl, "fields");
    let mut interface = Map::new();
    for &name in interface_fields(kind) {
        if let Some(v) = fields.get(name) {
            interface.insert(name.to_string(), field_value(name, v)?);
        }
    }
    let members = obj(decl, "members");
    let mut public_members = Map::new();
    if kind == "core:action" {
        if let Some(errors) = members.get("errors").and_then(Value::as_object) {
            let codes = errors
                .iter()
                .map(|(k, v)| {
                    let code = v
                        .get("code")
                        .cloned()
                        .unwrap_or_else(|| Value::String(k.clone()));
                    (k.clone(), code)
                })
                .collect();
            public_members.insert("errors".into(), Value::Object(codes));
        }
    }
    if kind == "core:entity" {
        if let Some(entity_fields) = members.get("fields") {
            public_members.insert("fields".into(), entity_fields.clone());
        }
    }
    let annotations = sorted_set(decl.get("annotations").unwrap_or(&Value::Array(vec![])))?;
    Ok(json!({
        "kind": kind,
        "summary": get_or_null(fields, "summary"),
        "interface": interface,
        "members": public_members,
        "annotations": annotations,
    }))
}

fn declarations(graph: &Value) -> GResult<BTreeMap<String, &Value>> {
    let Some(list) = graph.get("declarations").and_then(Value::as_array) else {
        return malformed("missing 'declarations' array");
    };
    let mut out = BTreeMap::new();
    for decl in list {
        out.insert(str_field(decl, "address")?.to_string(), decl);
    }
    Ok(out)
}

/// Tarjan's algorithm over `deps`, visiting addresses in sorted order; components come
/// out in reverse topological order, each sorted by address.
fn strongly_connected(deps: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    struct State<'a> {
        deps: &'a BTreeMap<String, Vec<String>>,
        index: BTreeMap<&'a str, usize>,
        low: BTreeMap<&'a str, usize>,
        stack: Vec<&'a str>,
        on_stack: BTreeSet<&'a str>,
        counter: usize,
        components: Vec<Vec<String>>,
    }
    fn strong<'a>(s: &mut State<'a>, v: &'a str) {
        s.index.insert(v, s.counter);
        s.low.insert(v, s.counter);
        s.counter += 1;
        s.stack.push(v);
        s.on_stack.insert(v);
        for w in &s.deps[v] {
            let w = w.as_str();
            if !s.index.contains_key(w) {
                strong(s, w);
                let lw = s.low[w];
                let lv = s.low.get_mut(v).expect("visited");
                *lv = (*lv).min(lw);
            } else if s.on_stack.contains(w) {
                let iw = s.index[w];
                let lv = s.low.get_mut(v).expect("visited");
                *lv = (*lv).min(iw);
            }
        }
        if s.low[v] == s.index[v] {
            let mut component = Vec::new();
            while let Some(w) = s.stack.pop() {
                s.on_stack.remove(w);
                component.push(w.to_string());
                if w == v {
                    break;
                }
            }
            component.sort();
            s.components.push(component);
        }
    }
    let mut s = State {
        deps,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        counter: 0,
        components: Vec::new(),
    };
    for v in deps.keys() {
        if !s.index.contains_key(v.as_str()) {
            strong(&mut s, v);
        }
    }
    s.components
}

/// Public hashes for every declaration. Mutually recursive type references are hashed
/// together as one strongly connected component.
pub fn public_hashes(graph: &Value) -> GResult<BTreeMap<String, String>> {
    let decls = declarations(graph)?;
    let mut raw = BTreeMap::new();
    for (address, decl) in &decls {
        raw.insert(address.clone(), raw_public(decl)?);
    }
    let deps: BTreeMap<String, Vec<String>> = raw
        .iter()
        .map(|(a, r)| {
            let mut refs = BTreeSet::new();
            named_refs(r, &mut refs);
            (
                a.clone(),
                refs.into_iter().filter(|x| decls.contains_key(x)).collect(),
            )
        })
        .collect();
    let mut hashes = BTreeMap::new();
    for component in strongly_connected(&deps) {
        let single = component.len() == 1 && !deps[&component[0]].contains(&component[0]);
        if single {
            let a = &component[0];
            let hash = digest(&resolve_types(&raw[a], &hashes))?;
            hashes.insert(a.clone(), hash);
        } else {
            let outside: BTreeMap<String, String> = hashes
                .iter()
                .filter(|(k, _)| !component.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let members: Map<String, Value> = component
                .iter()
                .map(|a| (a.clone(), resolve_types(&raw[a], &outside)))
                .collect();
            let component_hash = digest(&Value::Object(members))?;
            for a in &component {
                let hash = digest(&json!({ "component": component_hash, "address": a }))?;
                hashes.insert(a.clone(), hash);
            }
        }
    }
    Ok(hashes)
}

fn expect_texts(members: &Map<String, Value>, collection: &str, key: &str) -> Value {
    let map = members.get(collection).and_then(Value::as_object);
    Value::Object(
        map.into_iter()
            .flatten()
            .map(|(k, v)| (k.clone(), v.get(key).cloned().unwrap_or(Value::Null)))
            .collect(),
    )
}

/// The projection hashes of every declaration, keyed by address.
pub fn projections(graph: &Value) -> GResult<BTreeMap<String, Projections>> {
    let decls = declarations(graph)?;
    let modules: BTreeMap<&str, &Value> = graph
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| Some((m.get("path")?.as_str()?, m)))
        .collect();
    let public = public_hashes(graph)?;
    let project = graph
        .get("declarations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|d| d.get("kind").and_then(Value::as_str) == Some("core:project"));

    let mut out = BTreeMap::new();
    for (address, decl) in &decls {
        let fields = obj(decl, "fields");
        let members = obj(decl, "members");

        let mut inherited = Vec::new();
        if let Some(project) = project {
            let project_fields = obj(project, "fields");
            if str_field(project, "address")? != address {
                if let Some(prompt) = project_fields.get("prompt") {
                    inherited.push(json!({ "source": "project", "hash": digest(prompt)? }));
                }
            }
        }
        let module = str_field(decl, "module")?;
        let parts: Vec<&str> = if module.is_empty() {
            vec![]
        } else {
            module.split('.').collect()
        };
        for i in 1..=parts.len() {
            let path = parts[..i].join(".");
            if let Some(prompt) = modules
                .get(path.as_str())
                .and_then(|m| m.get("header"))
                .and_then(|h| h.get("prompt"))
            {
                inherited
                    .push(json!({ "source": format!("module:{path}"), "hash": digest(prompt)? }));
            }
        }
        for rule_address in decl
            .get("applies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(rule_address) = rule_address.as_str() else {
                return malformed(format!("{address}: non-string entry in 'applies'"));
            };
            let Some(rule) = decls.get(rule_address) else {
                return malformed(format!("{address}: applies unknown rule {rule_address}"));
            };
            let rule_input = json!({
                "prompt": get_or_null(obj(rule, "fields"), "prompt"),
                "expect": expect_texts(obj(rule, "members"), "expect", "text"),
            });
            inherited.push(
                json!({ "source": format!("rule:{rule_address}"), "hash": digest(&rule_input)? }),
            );
        }

        let behaviour = json!({
            "prompt": get_or_null(fields, "prompt"),
            "expect": expect_texts(members, "expect", "text"),
            "errors": expect_texts(members, "errors", "summary"),
        });
        let context = json!({ "normative": inherited });

        let kind = str_field(decl, "kind")?;
        let interface = interface_fields(kind);
        let mut body_fields = Map::new();
        for (name, value) in fields {
            let name = name.as_str();
            if interface.contains(&name)
                || BEHAVIOUR_FIELDS.contains(&name)
                || GUIDANCE_FIELDS.contains(&name)
                || name == "summary"
            {
                continue;
            }
            body_fields.insert(name.to_string(), field_value(name, value)?);
        }
        let policies: Map<String, Value> = members
            .get("expect")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| Some((k.clone(), v.get("verify")?.clone())))
            .collect();
        let Some(lifecycle) = decl.get("lifecycle") else {
            return malformed(format!("{address}: missing 'lifecycle'"));
        };
        let body = json!({ "fields": body_fields, "policies": policies, "lifecycle": lifecycle });

        let mut guidance = Map::new();
        for &name in GUIDANCE_FIELDS {
            if let Some(v) = fields.get(name) {
                guidance.insert(name.to_string(), field_value(name, v)?);
            }
        }

        let Some(public_hash) = public.get(address) else {
            return malformed(format!("{address}: no public hash"));
        };
        out.insert(
            address.clone(),
            Projections {
                public: public_hash.clone(),
                behaviour: digest(&behaviour)?,
                body: digest(&body)?,
                guidance: digest(&Value::Object(guidance))?,
                context: digest(&context)?,
            },
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(address: &str, kind: &str, fields: Value) -> Value {
        json!({
            "address": address, "module": "m", "kind": kind, "sort": "managed",
            "fields": fields, "members": {}, "lifecycle": "managed", "edges": []
        })
    }

    #[test]
    fn set_fields_are_sorted() {
        let a = json!({ "modules": [], "declarations": [decl("m.a", "core:lib", json!({ "uses": ["b", "a"] }))] });
        let b = json!({ "modules": [], "declarations": [decl("m.a", "core:lib", json!({ "uses": ["a", "b"] }))] });
        assert_eq!(projections(&a).unwrap(), projections(&b).unwrap());
    }

    #[test]
    fn type_refs_carry_the_public_hash() {
        let typed = |summary: &str| {
            json!({ "modules": [], "declarations": [
                decl("m.t", "core:type", json!({ "summary": summary })),
                decl("m.f", "core:action", json!({ "input": { "$type": "named", "name": "m.t" } })),
            ]})
        };
        let one = public_hashes(&typed("one")).unwrap();
        let two = public_hashes(&typed("two")).unwrap();
        assert_ne!(
            one["m.f"], two["m.f"],
            "changing the type changes its user's public hash"
        );
    }

    #[test]
    fn recursive_types_hash_as_a_component() {
        let g = json!({ "modules": [], "declarations": [
            decl("m.a", "core:action", json!({ "input": { "$type": "named", "name": "m.b" } })),
            decl("m.b", "core:action", json!({ "input": { "$type": "named", "name": "m.a" } })),
        ]});
        let h = public_hashes(&g).unwrap();
        assert_ne!(h["m.a"], h["m.b"]);
        assert!(h["m.a"].starts_with("sha256:"));
    }

    #[test]
    fn inherited_context_order() {
        let g = json!({
            "modules": [
                { "path": "m", "header": { "prompt": "outer" } },
                { "path": "m.n", "header": { "prompt": "inner" } }
            ],
            "declarations": [
                { "address": "p", "module": "", "kind": "core:project", "sort": "context",
                  "fields": { "prompt": "project" }, "members": {}, "lifecycle": "managed", "edges": [] },
                { "address": "m.n.x", "module": "m.n", "kind": "core:lib", "sort": "managed",
                  "fields": {}, "members": {}, "lifecycle": "managed", "edges": [] }
            ]
        });
        let expected = json!({ "normative": [
            { "source": "project", "hash": digest(&json!("project")).unwrap() },
            { "source": "module:m", "hash": digest(&json!("outer")).unwrap() },
            { "source": "module:m.n", "hash": digest(&json!("inner")).unwrap() },
        ]});
        assert_eq!(
            projections(&g).unwrap()["m.n.x"].context,
            digest(&expected).unwrap()
        );
    }
}
