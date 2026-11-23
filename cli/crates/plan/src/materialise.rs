//! Where each managed declaration's outputs live and who produces them (docs/spec.md, Kind
//! definitions and Outputs): slots and creation roots from the declaration's package kind,
//! from the one package that `implements` its core kind, or from its `use`'s binding.

use std::collections::BTreeMap;

use sdlang_graph::loader::{Package, Project};
use sdlang_graph::schema::{package_kind, FieldType};
use sdlang_graph::scope::lit_text;
use sdlang_syntax::ast::{Block, Exportable, Expr, MemberValue, Statement};
use sdlang_syntax::strings::Part;
use serde_json::Value;

use crate::error::{PResult, PlanError};

/// One declaration's outputs and producer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Materialisation {
    /// Slot key -> repository-relative path.
    pub slots: BTreeMap<String, String>,
    /// Creation root key -> repository-relative directory, ending in `/`.
    pub roots: BTreeMap<String, String>,
    /// The generator that exclusively produces the outputs, if any.
    pub generator: Option<String>,
}

/// The source of a declaration's templates.
struct Source<'p> {
    body: &'p Block,
    /// The kind's `route_path` fields (package kinds only).
    route_fields: Vec<String>,
}

fn package_statements(p: &Package) -> impl Iterator<Item = &Exportable> {
    p.files
        .iter()
        .flat_map(|f| f.ast.statements.iter())
        .filter_map(|s| match s {
            Statement::Decl(d) => Some(&d.item),
            _ => None,
        })
}

/// The package implementing each core kind; two implementers of one kind is an error.
fn implementers(project: &Project) -> PResult<BTreeMap<String, (&Package, &Block)>> {
    let mut out: BTreeMap<String, (&Package, &Block)> = BTreeMap::new();
    for p in &project.packages {
        for item in package_statements(p) {
            if let Exportable::Implements { kind, body } = item {
                if let Some((other, _)) = out.get(&kind.text) {
                    return Err(PlanError::new(format!(
                        "core kind {} is implemented by both {} and {}",
                        kind.text, other.id, p.id
                    )));
                }
                out.insert(kind.text.clone(), (p, body));
            }
        }
    }
    Ok(out)
}

fn source_for<'p>(
    project: &'p Project,
    implementers: &BTreeMap<String, (&'p Package, &'p Block)>,
    decl: &Value,
) -> Option<Source<'p>> {
    let kind = decl["kind"].as_str()?;
    if kind == "core:use" {
        let package = decl["fields"]["package"].as_str()?;
        let capability = decl["fields"]["capability"].as_str()?;
        let p = project.packages.iter().find(|p| p.npm_name == package)?;
        return package_statements(p).find_map(|item| match item {
            Exportable::Named(n) if n.kind.dotted() == "binding" && n.name.text == capability => {
                Some(Source {
                    body: &n.body,
                    route_fields: vec![],
                })
            }
            _ => None,
        });
    }
    if let Some(core) = kind.strip_prefix("core:") {
        let (_, body) = implementers.get(core)?;
        return Some(Source {
            body,
            route_fields: vec![],
        });
    }
    let (package_id, name) = kind.rsplit_once("::")?;
    let p = project.packages.iter().find(|p| p.id == package_id)?;
    package_statements(p).find_map(|item| match item {
        Exportable::Kind { name: n, body } if n.text == name => {
            let schema = package_kind(kind.to_string(), body);
            Some(Source {
                body,
                route_fields: schema
                    .fields
                    .iter()
                    .filter(|(_, f)| f.ty == FieldType::RoutePath)
                    .map(|(n, _)| n.clone())
                    .collect(),
            })
        }
        _ => None,
    })
}

fn section<'b>(body: &'b Block, name: &str) -> Option<&'b Block> {
    body.members.iter().find_map(|m| match &m.value {
        MemberValue::Section(b) if m.name.text == name => Some(b),
        _ => None,
    })
}

/// The declaration's name within its module, and its module as a path.
fn meta(decl: &Value) -> (String, String) {
    let address = decl["address"].as_str().unwrap_or_default();
    let module = decl["module"].as_str().unwrap_or_default();
    let name = if module.is_empty() {
        address
    } else {
        address
            .strip_prefix(module)
            .and_then(|r| r.strip_prefix('.'))
            .unwrap_or(address)
    };
    (name.to_string(), module.replace('.', "/"))
}

/// Interpolates and normalises one template (docs/spec.md, Kind definitions: substitution).
pub fn expand(
    template: &[Part],
    decl: &Value,
    route_fields: &[String],
    root: bool,
) -> PResult<String> {
    let address = decl["address"].as_str().unwrap_or_default();
    let (name, module) = meta(decl);
    let mut raw = String::new();
    for part in template {
        match part {
            Part::Text(t) => raw.push_str(t),
            Part::Mention(m) => {
                return Err(PlanError::new(format!(
                    "{address}: output template mentions @{{{m}}}"
                )))
            }
            Part::Interp(field) => match field.as_str() {
                "meta.name" => raw.push_str(&name),
                "meta.module" => raw.push_str(&module),
                f => {
                    let value = &decl["fields"][f];
                    let text = match value {
                        Value::String(s) if route_fields.iter().any(|r| r == f) => {
                            s.strip_prefix('/').unwrap_or(s).to_string()
                        }
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => {
                            return Err(PlanError::new(format!(
                                "{address}: output template needs scalar field '{f}'"
                            )))
                        }
                    };
                    raw.push_str(&text);
                }
            },
        }
    }
    normalise(&raw, root)
        .map_err(|why| PlanError::new(format!("{address}: output path '{raw}' {why}")))
}

fn normalise(raw: &str, root: bool) -> Result<String, &'static str> {
    if raw.starts_with('/') || raw.contains('\\') {
        return Err("is not repository-relative");
    }
    let segments: Vec<&str> = raw.split('/').filter(|s| !s.is_empty()).collect();
    if segments.iter().any(|s| *s == ".." || *s == ".") {
        return Err("escapes or aliases with '.' or '..'");
    }
    if segments.is_empty() {
        return Err("is empty");
    }
    let mut out = segments.join("/");
    if root {
        out.push('/');
    }
    Ok(out)
}

fn template_of(expr: &Expr) -> Option<&[Part]> {
    match expr {
        Expr::String(s) => Some(&s.parts),
        _ => None,
    }
}

fn templates(
    body: &Block,
    name: &str,
    decl: &Value,
    route_fields: &[String],
    root: bool,
) -> PResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let Some(section) = section(body, name) else {
        return Ok(out);
    };
    for m in &section.members {
        let MemberValue::Field(f) = &m.value else {
            continue;
        };
        if let Some(parts) = template_of(&f.value) {
            out.insert(
                m.name.text.clone(),
                expand(parts, decl, route_fields, root)?,
            );
        }
    }
    Ok(out)
}

fn generator(body: &Block) -> Option<String> {
    body.members.iter().find_map(|m| match &m.value {
        MemberValue::Field(f) if m.name.text == "generator" => Some(match &f.value {
            Expr::Path { path, .. } => path.dotted(),
            Expr::String(s) => lit_text(s),
            _ => "generator".to_string(),
        }),
        _ => None,
    })
}

/// The body that materialises `decl`: its package kind, the `implements` of its core
/// kind, or its `use`'s binding (for sections such as `checks`, `ambient` and `retains`).
pub fn kind_body<'p>(project: &'p Project, decl: &Value) -> Option<&'p Block> {
    let implementers = implementers(project).ok()?;
    source_for(project, &implementers, decl).map(|s| s.body)
}

/// The package that owns `decl`'s materialising body.
pub fn kind_package<'p>(project: &'p Project, decl: &Value) -> Option<&'p Package> {
    let kind = decl["kind"].as_str()?;
    if kind == "core:use" {
        let package = decl["fields"]["package"].as_str()?;
        return project.packages.iter().find(|p| p.npm_name == package);
    }
    if let Some(core) = kind.strip_prefix("core:") {
        return implementers(project).ok()?.get(core).map(|(p, _)| *p);
    }
    let (package_id, _) = kind.rsplit_once("::")?;
    project.packages.iter().find(|p| p.id == package_id)
}

/// Every managed declaration's materialisation, keyed by address. Two declarations
/// claiming one path is an error.
pub fn materialise(project: &Project, graph: &Value) -> PResult<BTreeMap<String, Materialisation>> {
    let implementers = implementers(project)?;
    let mut out = BTreeMap::new();
    let mut claims: BTreeMap<String, String> = BTreeMap::new();
    for decl in graph["declarations"].as_array().into_iter().flatten() {
        if decl["sort"] != "managed" {
            continue;
        }
        let address = decl["address"].as_str().unwrap_or_default().to_string();
        let mut m = Materialisation::default();
        if let Some(source) = source_for(project, &implementers, decl) {
            m.slots = templates(source.body, "outputs", decl, &source.route_fields, false)?;
            m.roots = templates(source.body, "creates", decl, &source.route_fields, true)?;
            m.generator = generator(source.body);
        }
        if decl["kind"] == "core:lib" {
            if let Some(path) = decl["fields"]["path"].as_str() {
                let path = normalise(path, false)
                    .map_err(|why| PlanError::new(format!("{address}: path '{path}' {why}")))?;
                m.slots.insert("main".into(), path);
            }
        }
        for path in m.slots.values() {
            if let Some(other) = claims.insert(path.clone(), address.clone()) {
                return Err(PlanError::new(format!(
                    "{path} is claimed by both {other} and {address}"
                )));
            }
        }
        out.insert(address, m);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parts(src: &str) -> Vec<Part> {
        let file = sdlang_syntax::parse(&format!("let t = \"{src}\"\n")).unwrap();
        let Statement::Decl(d) = &file.statements[0] else {
            panic!()
        };
        let Exportable::Let { value, .. } = &d.item else {
            panic!()
        };
        template_of(value).unwrap().to_vec()
    }

    fn page(route: &str) -> Value {
        json!({ "address": "auth.login", "module": "auth", "fields": { "route": route } })
    }

    #[test]
    fn route_paths_lose_their_slash() {
        let route = vec!["route".to_string()];
        let t = parts("src/app/${route}/page.tsx");
        assert_eq!(
            expand(&t, &page("/login"), &route, false).unwrap(),
            "src/app/login/page.tsx"
        );
        assert_eq!(
            expand(&t, &page("/"), &route, false).unwrap(),
            "src/app/page.tsx"
        );
        let r = parts("src/app/${route}/_parts/");
        assert_eq!(
            expand(&r, &page("/app"), &route, true).unwrap(),
            "src/app/app/_parts/"
        );
    }

    #[test]
    fn meta_fields() {
        let t = parts("src/server/${meta.module}/${meta.name}.ts");
        let d = json!({ "address": "a.b.sign_in", "module": "a.b", "fields": {} });
        assert_eq!(
            expand(&t, &d, &[], false).unwrap(),
            "src/server/a/b/sign_in.ts"
        );
    }

    #[test]
    fn escapes_are_errors() {
        let d = json!({ "address": "x", "module": "", "fields": { "p": "../etc" } });
        assert!(expand(&parts("${p}/f"), &d, &[], false).is_err());
        assert!(normalise("/abs", false).is_err());
    }
}
