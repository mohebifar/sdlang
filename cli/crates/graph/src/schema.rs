//! Field schemas: core kinds are fixed here; package kinds are read from `kind`
//! definitions. A schema is closed: unknown fields and sections are errors.

use std::collections::BTreeMap;

use intlang_syntax::ast::{Block, Expr, MemberValue};

/// How a field's expression is read and lowered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldType {
    /// A line or prose string, lowered to a plain string.
    Str,
    /// Normative or advisory text, lowered to `{ "$prose": ... }`.
    Prose,
    /// A `route_path` string.
    RoutePath,
    Bool,
    Number,
    /// A type expression (`input`, `output`, a `type` body).
    Type,
    /// A list of references; `relation` names the edges it creates.
    Refs {
        relation: Option<String>,
    },
    /// One reference.
    Ref {
        relation: Option<String>,
    },
    /// A record value, lowered to `{ "$record": ... }`.
    Record,
    /// A list of strings.
    StringList,
    /// A selector.
    Selector,
    /// Anything, lowered by its shape.
    Any,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSchema {
    pub ty: FieldType,
    pub interface: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Managed,
    Context,
}

impl Sort {
    pub fn as_str(self) -> &'static str {
        match self {
            Sort::Managed => "managed",
            Sort::Context => "context",
        }
    }
}

/// Addressable member collections a kind may declare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Collection {
    Expect,
    Errors,
    EntityFields,
}

impl Collection {
    pub fn section(self) -> &'static str {
        match self {
            Collection::Expect => "expect",
            Collection::Errors => "errors",
            Collection::EntityFields => "fields",
        }
    }
}

#[derive(Clone, Debug)]
pub struct KindSchema {
    /// `core:<kind>` or `<package id>::<kind>`.
    pub canonical: String,
    pub sort: Sort,
    pub fields: BTreeMap<String, FieldSchema>,
    pub collections: Vec<Collection>,
    /// Any field name is allowed and read as a record value (the stack's tool slots).
    pub open_records: bool,
}

fn field(ty: FieldType) -> FieldSchema {
    FieldSchema {
        ty,
        interface: false,
    }
}

fn refs(relation: Option<&str>) -> FieldType {
    FieldType::Refs {
        relation: relation.map(String::from),
    }
}

/// Fields every declaration may carry (SPEC.md: core declaration fields).
fn common_fields() -> BTreeMap<String, FieldSchema> {
    let mut f = BTreeMap::new();
    f.insert("summary".into(), field(FieldType::Str));
    f.insert("prompt".into(), field(FieldType::Prose));
    f.insert("hint".into(), field(FieldType::Prose));
    f.insert("uses".into(), field(refs(Some("uses"))));
    f.insert("skills".into(), field(refs(None)));
    f.insert("lifecycle".into(), field(FieldType::Str));
    f
}

/// The schema of a core kind, by its prelude name.
pub fn core_kind(name: &str) -> Option<KindSchema> {
    let mut fields = common_fields();
    let mut collections = vec![Collection::Expect];
    let mut sort = Sort::Managed;
    let mut open_records = false;
    match name {
        "project" => {
            fields.remove("uses");
            fields.insert("verify".into(), field(FieldType::Str));
            collections.clear();
            sort = Sort::Context;
        }
        "stack" => {
            fields.clear();
            collections.clear();
            open_records = true;
        }
        "module" => {
            fields.remove("uses");
            fields.remove("lifecycle");
            collections.clear();
            sort = Sort::Context;
        }
        "action" => {
            for f in ["input", "output"] {
                fields.insert(
                    f.into(),
                    FieldSchema {
                        ty: FieldType::Type,
                        interface: true,
                    },
                );
            }
            fields.insert("requires".into(), field(refs(Some("requires"))));
            fields.insert("reads".into(), field(refs(Some("reads"))));
            fields.insert("writes".into(), field(refs(Some("writes"))));
            collections.push(Collection::Errors);
        }
        "lib" => {
            fields.insert(
                "exports".into(),
                FieldSchema {
                    ty: FieldType::StringList,
                    interface: true,
                },
            );
            fields.insert("path".into(), field(FieldType::Str));
        }
        "entity" => collections.push(Collection::EntityFields),
        "rule" => {
            fields.insert(
                "applies_to".into(),
                FieldSchema {
                    ty: FieldType::Selector,
                    interface: true,
                },
            );
            sort = Sort::Context;
        }
        "type" => {
            fields.clear();
            collections.clear();
            sort = Sort::Context;
        }
        _ => return None,
    }
    Some(KindSchema {
        canonical: format!("core:{name}"),
        sort,
        fields,
        collections,
        open_records,
    })
}

/// The fields of an `extern` declaration, whatever its kind.
pub fn extern_fields() -> BTreeMap<String, FieldSchema> {
    let mut f = BTreeMap::new();
    f.insert("summary".into(), field(FieldType::Str));
    f.insert("source".into(), field(FieldType::Str));
    f
}

const BUILTIN_SCALARS: &[(&str, FieldType)] = &[
    ("string", FieldType::Str),
    ("email", FieldType::Str),
    ("url", FieldType::Str),
    ("uuid", FieldType::Str),
    ("template", FieldType::Str),
    ("route_path", FieldType::RoutePath),
    ("boolean", FieldType::Bool),
    ("number", FieldType::Number),
    ("integer", FieldType::Number),
];

fn relation_of(annotations: &[intlang_syntax::ast::Annotation]) -> Option<String> {
    annotations.iter().find_map(|a| {
        if a.name.dotted() != "relation" {
            return None;
        }
        match a.args.as_deref() {
            Some([Expr::String(s)]) => Some(s.to_source_text()),
            _ => None,
        }
    })
}

/// The field type a kind definition's type expression describes.
fn field_type(expr: &Expr, relation: Option<String>) -> FieldType {
    match expr {
        Expr::Path {
            path, record: None, ..
        } if path.segments.len() == 1 => {
            let name = path.segments[0].text.as_str();
            BUILTIN_SCALARS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, t)| t.clone())
                .unwrap_or(FieldType::Any)
        }
        Expr::Ref { .. } => FieldType::Ref { relation },
        Expr::List { items, .. } if items.len() == 1 => match &items[0] {
            Expr::Ref { .. } => FieldType::Refs { relation },
            Expr::Path { path, .. } if path.dotted() == "string" => FieldType::StringList,
            _ => FieldType::Any,
        },
        _ => FieldType::Any,
    }
}

/// The schema of a package kind from its definition body.
pub fn package_kind(canonical: String, body: &Block) -> KindSchema {
    let mut fields = common_fields();
    for member in &body.members {
        if member.name.text != "fields" {
            continue;
        }
        let MemberValue::Section(section) = &member.value else {
            continue;
        };
        for f in &section.members {
            let MemberValue::Field(def) = &f.value else {
                continue;
            };
            let interface = def
                .annotations
                .iter()
                .any(|a| a.name.dotted() == "interface");
            let relation = relation_of(&def.annotations);
            fields.insert(
                f.name.text.clone(),
                FieldSchema {
                    ty: field_type(&def.value, relation),
                    interface,
                },
            );
        }
    }
    KindSchema {
        canonical,
        sort: Sort::Managed,
        fields,
        collections: vec![Collection::Expect],
        open_records: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intlang_syntax::ast::{Exportable, Statement};

    fn kind_body(src: &str) -> Block {
        let file = intlang_syntax::parse(src).unwrap();
        let Statement::Decl(d) = &file.statements[0] else {
            panic!()
        };
        let Exportable::Kind { body, .. } = &d.item else {
            panic!()
        };
        body.clone()
    }

    #[test]
    fn interface_and_relation_from_definitions() {
        let body = kind_body(
            "kind page {\n  fields {\n    route: route_path @interface\n    calls: [ref action] @relation(\"calls\")\n  }\n}\n",
        );
        let k = package_kind("npm:x::page".into(), &body);
        assert_eq!(
            k.fields["route"],
            FieldSchema {
                ty: FieldType::RoutePath,
                interface: true
            }
        );
        assert_eq!(
            k.fields["calls"].ty,
            FieldType::Refs {
                relation: Some("calls".into())
            }
        );
        assert!(
            k.fields.contains_key("summary"),
            "core fields are always present"
        );
    }

    #[test]
    fn core_kinds() {
        assert!(core_kind("action").unwrap().fields["input"].interface);
        assert_eq!(core_kind("rule").unwrap().sort, Sort::Context);
        assert!(core_kind("page").is_none());
    }
}
