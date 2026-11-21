//! Lowering a loaded, resolved project to the canonical graph
//! (`schemas/graph.schema.json`): fields by their schema, members with short forms
//! expanded, edges with the field or member path they come from, annotations, rule
//! attachment and binding membership.

use std::collections::{BTreeMap, BTreeSet};

use intlang_syntax::ast::{
    Address, Annotation, Block, Exportable, Expr, Field, MemberValue, NamedDecl, Operand,
    SelectSource, Selector, Span, Statement,
};
use intlang_syntax::diagnostics::line_col;
use intlang_syntax::strings::{Part, StringKind, StringLit};
use serde_json::{json, Map, Value};

use crate::error::{GResult, GraphError};
use crate::loader::{Project, SourceFile};
use crate::schema::{self, Collection, FieldSchema, FieldType, KindSchema, Sort};
use crate::scope::{lit_text, DeclForm, Owner, Scope, Target};
use crate::selectors::{self, Candidate};

/// A lowered body: fields, members and lifecycle.
type Body = (Map<String, Value>, Map<String, Value>, String);

/// One lowered declaration and what attachment needs to know about it.
struct Lowered {
    value: Map<String, Value>,
    module: String,
    name: String,
    kind: String,
    sort: Sort,
}

/// A rule's selector, kept for attachment after every declaration is lowered.
struct RuleSelector<'a> {
    address: String,
    module: String,
    selector: &'a Selector,
    file: &'a SourceFile,
}

/// A binding's `collects` selector, for the `use` declaration that owns it.
struct Collects<'a> {
    use_address: String,
    package: usize,
    selector: &'a Selector,
}

/// Per-declaration state while lowering.
struct Ctx<'a> {
    owner: Owner,
    file: &'a SourceFile,
    address: String,
    edges: BTreeSet<(String, String, String)>,
    annotations: Vec<Value>,
}

impl Ctx<'_> {
    fn edge(&mut self, relation: &str, target: &str, via: &str) {
        self.edges
            .insert((relation.into(), target.into(), via.into()));
    }

    fn err(&self, span: Span, message: impl Into<String>) -> GraphError {
        GraphError::at(self.file, span, message)
    }
}

struct Lowerer<'p> {
    project: &'p Project,
    scope: Scope<'p>,
    kinds: BTreeMap<String, KindSchema>,
}

fn source_of(file: &SourceFile, span: Span) -> Value {
    let (line, _) = line_col(&file.text, span.start);
    let path: Vec<String> = file
        .path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    json!({ "file": path.join("/"), "line": line })
}

fn address_text(a: &Address) -> String {
    let mut out = a
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(".");
    if let Some(member) = &a.member {
        out.push('#');
        out.push_str(
            &member
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("."),
        );
    }
    out
}

fn number(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn sort_by_canonical(items: &mut [Value]) {
    items.sort_by_cached_key(|v| intlang_canon::canonical(v).unwrap_or_default());
}

fn valid_route(route: &str) -> bool {
    route == "/"
        || (route.starts_with('/')
            && route[1..].split('/').all(|seg| {
                !seg.is_empty()
                    && seg.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)
                    })
            }))
}

impl<'p> Lowerer<'p> {
    // ------------------------------------------------------------ text

    /// A string literal's text with mentions resolved and rewritten to `@{address}`, each
    /// adding a `mentions` edge from `via`.
    fn text(&self, ctx: &mut Ctx, lit: &StringLit, via: &str) -> GResult<String> {
        let mut out = String::new();
        for part in &lit.parts {
            match part {
                Part::Text(t) => out.push_str(t),
                Part::Mention(m) => {
                    let target = if m == "self" {
                        ctx.address.clone()
                    } else {
                        let names: Vec<&str> = m.split('.').collect();
                        let target = self
                            .scope
                            .resolve_names(&ctx.owner, &names, ctx.file, lit.span)?;
                        self.scope.canonical(&target).ok_or_else(|| {
                            ctx.err(lit.span, format!("@{{{m}}} does not name a declaration"))
                        })?
                    };
                    ctx.edge("mentions", &target, via);
                    out.push_str("@{");
                    out.push_str(&target);
                    out.push('}');
                }
                Part::Interp(_) => {
                    return Err(ctx.err(
                        lit.span,
                        "interpolation is legal only in template fields of kind definitions",
                    ))
                }
            }
        }
        Ok(out)
    }

    /// Prose: the text with trailing spaces removed from every line.
    fn prose(&self, ctx: &mut Ctx, lit: &StringLit, via: &str) -> GResult<Value> {
        let text = self.text(ctx, lit, via)?;
        let trimmed: Vec<&str> = text
            .split('\n')
            .map(|l| l.trim_end_matches([' ', '\t']))
            .collect();
        Ok(json!({ "$prose": trimmed.join("\n") }))
    }

    // ------------------------------------------------------------ references and types

    fn reference(&self, ctx: &Ctx, expr: &Expr) -> GResult<String> {
        let Expr::Path {
            path, record: None, ..
        } = expr
        else {
            return Err(ctx.err(expr.span(), "expected a reference"));
        };
        let target = self.scope.resolve(&ctx.owner, path, ctx.file)?;
        if let Target::Decl(i) = target {
            if self.scope.decls[i].form == DeclForm::Let {
                return Err(ctx.err(path.span, "a let is a value, not a reference"));
            }
        }
        self.scope.canonical(&target).ok_or_else(|| {
            ctx.err(
                path.span,
                format!("'{}' cannot be referenced here", path.dotted()),
            )
        })
    }

    fn lower_type(&self, ctx: &mut Ctx, expr: &Expr, via: &str) -> GResult<Value> {
        Ok(match expr {
            Expr::Path {
                path, record: None, ..
            } => match self.scope.resolve(&ctx.owner, path, ctx.file)? {
                Target::Builtin(name) => json!({ "$type": "builtin", "name": name }),
                target @ (Target::Decl(_) | Target::PackageItem { .. }) => {
                    if let Target::Decl(i) = target {
                        if self.scope.decls[i].form == DeclForm::Let {
                            return Err(ctx.err(path.span, "a let is a value, not a type"));
                        }
                    }
                    let name = self.scope.canonical(&target).expect("referenceable");
                    ctx.edge("types", &name, via);
                    json!({ "$type": "named", "name": name })
                }
                _ => return Err(ctx.err(path.span, format!("'{}' is not a type", path.dotted()))),
            },
            Expr::Record(record) => {
                let mut fields = Map::new();
                for f in &record.fields {
                    if f.field.default.is_some() || !f.field.annotations.is_empty() {
                        return Err(ctx.err(
                            f.span,
                            "defaults and annotations in signature records are not lowered yet",
                        ));
                    }
                    fields.insert(
                        f.name.text.clone(),
                        self.lower_type(ctx, &f.field.value, via)?,
                    );
                }
                json!({ "$type": "record", "fields": fields })
            }
            Expr::List { items, span } => match items.as_slice() {
                [one] => json!({ "$type": "list", "of": self.lower_type(ctx, one, via)? }),
                _ => return Err(ctx.err(*span, "a list type has exactly one element type")),
            },
            Expr::Optional { inner, .. } => {
                json!({ "$type": "optional", "of": self.lower_type(ctx, inner, via)? })
            }
            Expr::Ref { target, .. } => {
                json!({ "$type": "ref", "of": self.lower_type(ctx, target, via)? })
            }
            Expr::Union { variants, .. } => {
                let options = variants
                    .iter()
                    .map(|v| self.lower_type(ctx, v, via))
                    .collect::<GResult<Vec<_>>>()?;
                json!({ "$type": "union", "options": options })
            }
            Expr::String(s) => json!({ "$type": "literal", "value": lit_text(s) }),
            Expr::Number { text, .. } => json!({ "$type": "literal", "value": number(text) }),
            Expr::Bool { value, .. } => json!({ "$type": "literal", "value": value }),
            Expr::Null { .. } => json!({ "$type": "literal", "value": null }),
            other => return Err(ctx.err(other.span(), "expected a type")),
        })
    }

    // ------------------------------------------------------------ values

    fn lower_value(&self, ctx: &mut Ctx, ty: &FieldType, expr: &Expr, via: &str) -> GResult<Value> {
        match ty {
            FieldType::Str | FieldType::RoutePath => {
                let Expr::String(lit) = expr else {
                    return Err(ctx.err(expr.span(), "expected a string"));
                };
                let text = self.text(ctx, lit, via)?;
                if *ty == FieldType::RoutePath && !valid_route(&text) {
                    return Err(ctx.err(lit.span, format!("'{text}' is not a route_path: static segments of [a-z0-9._-] after '/'")));
                }
                Ok(Value::String(text))
            }
            FieldType::Prose => match expr {
                Expr::String(lit) => self.prose(ctx, lit, via),
                _ => Err(ctx.err(expr.span(), "expected text")),
            },
            FieldType::Bool => match expr {
                Expr::Bool { value, .. } => Ok(Value::Bool(*value)),
                _ => Err(ctx.err(expr.span(), "expected true or false")),
            },
            FieldType::Number => match expr {
                Expr::Number { text, .. } => Ok(number(text)),
                _ => Err(ctx.err(expr.span(), "expected a number")),
            },
            FieldType::Type => self.lower_type(ctx, expr, via),
            FieldType::Refs { relation } => {
                let Expr::List { items, .. } = expr else {
                    return Err(ctx.err(expr.span(), "expected a list of references"));
                };
                let mut out = Vec::new();
                for item in items {
                    let target = self.reference(ctx, item)?;
                    if let Some(r) = relation {
                        ctx.edge(r, &target, via);
                    }
                    out.push(json!({ "$ref": target }));
                }
                sort_by_canonical(&mut out);
                Ok(Value::Array(out))
            }
            FieldType::Ref { relation } => {
                let target = self.reference(ctx, expr)?;
                if let Some(r) = relation {
                    ctx.edge(r, &target, via);
                }
                Ok(json!({ "$ref": target }))
            }
            FieldType::Record => match expr {
                Expr::Record(_) => self.generic(ctx, expr, via),
                _ => Err(ctx.err(expr.span(), "expected a record")),
            },
            FieldType::StringList => {
                let Expr::List { items, .. } = expr else {
                    return Err(ctx.err(expr.span(), "expected a list of strings"));
                };
                items
                    .iter()
                    .map(|i| match i {
                        Expr::String(lit) => self.text(ctx, lit, via).map(Value::String),
                        other => Err(ctx.err(other.span(), "expected a string")),
                    })
                    .collect::<GResult<Vec<_>>>()
                    .map(Value::Array)
            }
            FieldType::Selector => match expr {
                Expr::Selector(s) => self.selector(ctx, s, &ctx.owner.clone()),
                _ => Err(ctx.err(expr.span(), "expected a selector")),
            },
            FieldType::Any => self.generic(ctx, expr, via),
        }
    }

    /// Lowers a value by its shape.
    fn generic(&self, ctx: &mut Ctx, expr: &Expr, via: &str) -> GResult<Value> {
        Ok(match expr {
            Expr::String(lit) if lit.kind == StringKind::Prose => self.prose(ctx, lit, via)?,
            Expr::String(lit) => Value::String(self.text(ctx, lit, via)?),
            Expr::Number { text, .. } => number(text),
            Expr::Bool { value, .. } => Value::Bool(*value),
            Expr::Null { .. } => Value::Null,
            Expr::List { items, .. } => Value::Array(
                items
                    .iter()
                    .map(|i| self.generic(ctx, i, via))
                    .collect::<GResult<Vec<_>>>()?,
            ),
            Expr::Record(record) => {
                let mut fields = Map::new();
                for f in &record.fields {
                    fields.insert(f.name.text.clone(), self.generic(ctx, &f.field.value, via)?);
                }
                json!({ "$record": fields })
            }
            Expr::Path {
                path, record: None, ..
            } => match self.scope.resolve(&ctx.owner, path, ctx.file)? {
                Target::Decl(i) if self.scope.decls[i].form == DeclForm::Let => {
                    self.let_value(i)?
                }
                Target::Now => json!({ "$init": "now" }),
                target => match self.scope.canonical(&target) {
                    Some(t) => json!({ "$ref": t }),
                    None => {
                        return Err(
                            ctx.err(path.span, format!("'{}' is not a value", path.dotted()))
                        )
                    }
                },
            },
            other => return Err(ctx.err(other.span(), "a type expression is not a value here")),
        })
    }

    fn let_value(&self, i: usize) -> GResult<Value> {
        let info = &self.scope.decls[i];
        let file = &self.project.modules[&info.module].files[info.file];
        let Statement::Decl(decl) = &file.ast.statements[info.statement] else {
            unreachable!()
        };
        let Exportable::Let { value, .. } = &decl.item else {
            unreachable!()
        };
        let mut ctx = Ctx {
            owner: Owner::Module(info.module.clone()),
            file,
            address: info.address.clone(),
            edges: BTreeSet::new(),
            annotations: Vec::new(),
        };
        self.generic(&mut ctx, value, "value")
    }

    fn selector(&self, ctx: &Ctx, s: &Selector, owner: &Owner) -> GResult<Value> {
        let kinds = self.selector_kinds(ctx.file, s, owner)?;
        let where_ = s.predicate.as_ref().map(selectors::canonical_where);
        Ok(json!({ "$select": { "kinds": kinds, "where": where_ } }))
    }

    fn selector_kinds(
        &self,
        file: &SourceFile,
        s: &Selector,
        owner: &Owner,
    ) -> GResult<Vec<String>> {
        let paths = match &s.source {
            SelectSource::All => return Ok(vec!["*".into()]),
            SelectSource::Kind(p) => vec![p],
            SelectSource::Kinds(ps) => ps.iter().collect(),
        };
        let mut kinds = Vec::new();
        for p in paths {
            let target = self.scope.resolve(owner, p, file)?;
            kinds.push(self.kind_id(&target).ok_or_else(|| {
                GraphError::at(file, p.span, format!("'{}' is not a kind", p.dotted()))
            })?);
        }
        kinds.sort();
        Ok(kinds)
    }

    fn kind_id(&self, target: &Target) -> Option<String> {
        match target {
            Target::CoreKind(n) => Some(format!("core:{n}")),
            Target::PackageItem { package, name } => {
                let item = &self.scope.package_items[*package][name];
                let statement =
                    &self.project.packages[*package].files[0].ast.statements[item.statement];
                match statement {
                    Statement::Decl(d) if matches!(d.item, Exportable::Kind { .. }) => {
                        Some(format!("{}::{name}", self.project.packages[*package].id))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn kind_schema(&mut self, target: &Target) -> Option<KindSchema> {
        match target {
            Target::CoreKind(n) => schema::core_kind(n),
            Target::PackageItem { package, name } => {
                let id = self.kind_id(target)?;
                if let Some(k) = self.kinds.get(&id) {
                    return Some(k.clone());
                }
                let item = &self.scope.package_items[*package][name];
                let statement =
                    &self.project.packages[*package].files[0].ast.statements[item.statement];
                let Statement::Decl(d) = statement else {
                    return None;
                };
                let Exportable::Kind { body, .. } = &d.item else {
                    return None;
                };
                let k = schema::package_kind(id.clone(), body);
                self.kinds.insert(id, k.clone());
                Some(k)
            }
            _ => None,
        }
    }

    // ------------------------------------------------------------ annotations and members

    fn annotation(&self, ctx: &mut Ctx, a: &Annotation, on: &str) -> GResult<Value> {
        let name = match self.scope.resolve(&ctx.owner, &a.name, ctx.file)? {
            Target::CoreAnnotation(n) => format!("core:{n}"),
            target @ Target::PackageItem { .. } => {
                self.scope.canonical(&target).expect("referenceable")
            }
            _ => {
                return Err(ctx.err(
                    a.name.span,
                    format!("'{}' is not an annotation", a.name.dotted()),
                ))
            }
        };
        let args = a
            .args
            .iter()
            .flatten()
            .map(|e| self.generic(ctx, e, on))
            .collect::<GResult<Vec<_>>>()?;
        Ok(json!({ "name": name, "on": on, "args": args }))
    }

    fn members(&self, ctx: &mut Ctx, collection: Collection, block: &Block) -> GResult<Value> {
        let mut out = Map::new();
        for m in &block.members {
            let key = m.name.text.clone();
            let value = match collection {
                Collection::Expect => {
                    let via = format!("expect.{key}.text");
                    match &m.value {
                        MemberValue::Field(Field {
                            value: Expr::String(lit),
                            default: None,
                            annotations,
                        }) if annotations.is_empty() => {
                            json!({ "text": self.prose(ctx, lit, &via)? })
                        }
                        MemberValue::Section(long) => {
                            let mut fields = Map::new();
                            for f in &long.members {
                                let ty = match f.name.text.as_str() {
                                    "text" => FieldType::Prose,
                                    "verify" => FieldType::Str,
                                    other => {
                                        return Err(ctx.err(
                                            f.name.span,
                                            format!("an expectation has no field '{other}'"),
                                        ))
                                    }
                                };
                                let MemberValue::Field(field) = &f.value else {
                                    return Err(ctx.err(f.span, "expected a field"));
                                };
                                let via = format!("expect.{key}.{}", f.name.text);
                                fields.insert(
                                    f.name.text.clone(),
                                    self.lower_value(ctx, &ty, &field.value, &via)?,
                                );
                            }
                            if !fields.contains_key("text") {
                                return Err(ctx.err(m.span, "an expectation needs text"));
                            }
                            Value::Object(fields)
                        }
                        _ => return Err(ctx.err(m.span, "an expectation is a string or a section")),
                    }
                }
                Collection::Errors => {
                    let via = format!("errors.{key}.summary");
                    match &m.value {
                        MemberValue::Field(Field {
                            value: Expr::String(lit),
                            default: None,
                            annotations,
                        }) if annotations.is_empty() => {
                            json!({ "summary": self.text(ctx, lit, &via)?, "code": key })
                        }
                        MemberValue::Section(long) => {
                            let mut fields = Map::new();
                            for f in &long.members {
                                if !matches!(f.name.text.as_str(), "summary" | "code") {
                                    return Err(ctx.err(
                                        f.name.span,
                                        format!("an error has no field '{}'", f.name.text),
                                    ));
                                }
                                let MemberValue::Field(field) = &f.value else {
                                    return Err(ctx.err(f.span, "expected a field"));
                                };
                                let via = format!("errors.{key}.{}", f.name.text);
                                fields.insert(
                                    f.name.text.clone(),
                                    self.lower_value(ctx, &FieldType::Str, &field.value, &via)?,
                                );
                            }
                            fields.entry("code").or_insert(Value::String(key.clone()));
                            Value::Object(fields)
                        }
                        _ => return Err(ctx.err(m.span, "an error is a string or a section")),
                    }
                }
                Collection::EntityFields => {
                    let via = format!("fields.{key}");
                    let MemberValue::Field(field) = &m.value else {
                        return Err(ctx.err(m.span, "an entity field is 'name: type'"));
                    };
                    let mut member = Map::new();
                    member.insert("type".into(), self.lower_type(ctx, &field.value, &via)?);
                    if let Some(default) = &field.default {
                        member.insert("default".into(), self.generic(ctx, default, &via)?);
                    }
                    for a in &field.annotations {
                        let lowered = self.annotation(ctx, a, &via)?;
                        ctx.annotations.push(lowered);
                    }
                    Value::Object(member)
                }
            };
            if out.contains_key(&key) {
                return Err(ctx.err(m.name.span, format!("'{key}' appears twice")));
            }
            out.insert(key, value);
        }
        Ok(Value::Object(out))
    }

    /// Lowers a body against a schema into (fields, members, lifecycle).
    fn body(
        &self,
        ctx: &mut Ctx,
        schema: &KindSchema,
        fields_schema: &BTreeMap<String, FieldSchema>,
        body: &Block,
    ) -> GResult<Body> {
        let mut fields = Map::new();
        let mut members = Map::new();
        let mut lifecycle = "managed".to_string();
        for m in &body.members {
            let name = m.name.text.as_str();
            match &m.value {
                MemberValue::Field(f) => {
                    if f.default.is_some() || !f.annotations.is_empty() {
                        return Err(ctx.err(
                            m.span,
                            "defaults and annotations are not allowed on declaration fields",
                        ));
                    }
                    if fields.contains_key(name) {
                        return Err(ctx.err(m.name.span, format!("'{name}' appears twice")));
                    }
                    let ty = if schema.open_records {
                        FieldType::Record
                    } else {
                        match fields_schema.get(name) {
                            Some(s) => s.ty.clone(),
                            None => {
                                return Err(ctx.err(
                                    m.name.span,
                                    format!("{} has no field '{name}'", schema.canonical),
                                ))
                            }
                        }
                    };
                    let value = self.lower_value(ctx, &ty, &f.value, name)?;
                    if name == "lifecycle" {
                        let l = value.as_str().unwrap_or_default();
                        if !matches!(l, "managed" | "frozen" | "protected") {
                            return Err(ctx
                                .err(f.value.span(), "lifecycle is managed, frozen or protected"));
                        }
                        lifecycle = l.to_string();
                    } else {
                        fields.insert(name.to_string(), value);
                    }
                }
                MemberValue::Section(block) => {
                    let Some(collection) = schema.collections.iter().find(|c| c.section() == name)
                    else {
                        return Err(ctx.err(
                            m.name.span,
                            format!("{} has no section '{name}'", schema.canonical),
                        ));
                    };
                    if members.contains_key(name) {
                        return Err(ctx.err(m.name.span, format!("'{name}' appears twice")));
                    }
                    members.insert(name.to_string(), self.members(ctx, *collection, block)?);
                }
            }
        }
        Ok((fields, members, lifecycle))
    }

    fn finish(ctx: Ctx, mut value: Map<String, Value>) -> Map<String, Value> {
        let mut edges: Vec<Value> = ctx
            .edges
            .into_iter()
            .map(|(relation, target, via)| json!({ "relation": relation, "target": target, "via": via }))
            .collect();
        sort_by_canonical(&mut edges);
        value.insert("edges".into(), Value::Array(edges));
        if !ctx.annotations.is_empty() {
            value.insert("annotations".into(), Value::Array(ctx.annotations));
        }
        value
    }

    fn named(
        &mut self,
        file: &'p SourceFile,
        module: &str,
        address: &str,
        exported: bool,
        n: &NamedDecl,
        span: Span,
    ) -> GResult<Lowered> {
        let owner = Owner::Module(module.to_string());
        let kind_target = self.scope.resolve(&owner, &n.kind, file)?;
        let schema = self.kind_schema(&kind_target).ok_or_else(|| {
            GraphError::at(
                file,
                n.kind.span,
                format!("'{}' is not a kind", n.kind.dotted()),
            )
        })?;
        let mut ctx = Ctx {
            owner,
            file,
            address: address.to_string(),
            edges: BTreeSet::new(),
            annotations: Vec::new(),
        };
        for a in &n.annotations {
            let lowered = self.annotation(&mut ctx, a, "")?;
            ctx.annotations.push(lowered);
        }
        let (fields_schema, sort) = if n.is_extern {
            (schema::extern_fields(), Sort::Context)
        } else {
            (schema.fields.clone(), schema.sort)
        };
        let (fields, members, lifecycle) = self.body(&mut ctx, &schema, &fields_schema, &n.body)?;
        let mut value = Map::new();
        value.insert("address".into(), json!(address));
        value.insert("module".into(), json!(module));
        value.insert("kind".into(), json!(schema.canonical));
        value.insert("sort".into(), json!(sort.as_str()));
        value.insert("exported".into(), json!(exported));
        if n.is_extern {
            value.insert("extern".into(), json!(true));
        }
        value.insert("fields".into(), Value::Object(fields));
        value.insert("members".into(), Value::Object(members));
        value.insert("lifecycle".into(), json!(lifecycle));
        value.insert("source".into(), source_of(file, span));
        Ok(Lowered {
            value: Self::finish(ctx, value),
            module: module.into(),
            name: n.name.text.clone(),
            kind: schema.canonical.clone(),
            sort,
        })
    }

    fn simple(
        file: &SourceFile,
        address: &str,
        kind: &str,
        sort: Sort,
        fields: Map<String, Value>,
        ctx: Ctx,
        span: Span,
    ) -> Lowered {
        let mut value = Map::new();
        value.insert("address".into(), json!(address));
        value.insert("module".into(), json!(""));
        value.insert("kind".into(), json!(kind));
        value.insert("sort".into(), json!(sort.as_str()));
        value.insert("exported".into(), json!(false));
        value.insert("fields".into(), Value::Object(fields));
        value.insert("members".into(), json!({}));
        value.insert("lifecycle".into(), json!("managed"));
        value.insert("source".into(), source_of(file, span));
        Lowered {
            value: Self::finish(ctx, value),
            module: String::new(),
            name: address.into(),
            kind: kind.into(),
            sort,
        }
    }

    fn new_ctx(&self, file: &'p SourceFile, module: &str, address: &str) -> Ctx<'p> {
        Ctx {
            owner: Owner::Module(module.into()),
            file,
            address: address.into(),
            edges: BTreeSet::new(),
            annotations: Vec::new(),
        }
    }

    /// The package's binding for a capability, if it defines one.
    fn binding_body(&self, package: usize, capability: &str) -> Option<&'p Block> {
        let file = &self.project.packages[package].files[0];
        file.ast.statements.iter().find_map(|s| match s {
            Statement::Decl(d) => match &d.item {
                Exportable::Named(n)
                    if n.kind.dotted() == "binding" && n.name.text == capability =>
                {
                    Some(&n.body)
                }
                _ => None,
            },
            _ => None,
        })
    }

    fn run(mut self) -> GResult<Value> {
        let project = self.project;
        let mut decls: BTreeMap<String, Lowered> = BTreeMap::new();
        let mut modules_out = Vec::new();
        let mut moves = Vec::new();
        let mut rules: Vec<RuleSelector> = Vec::new();
        let mut collects: Vec<Collects> = Vec::new();
        let insert = |decls: &mut BTreeMap<String, Lowered>,
                      l: Lowered,
                      file: &SourceFile,
                      span: Span|
         -> GResult<()> {
            let address = l.value["address"].as_str().unwrap_or_default().to_string();
            if decls.contains_key(&address) {
                return Err(GraphError::at(
                    file,
                    span,
                    format!("'{address}' is declared twice"),
                ));
            }
            decls.insert(address, l);
            Ok(())
        };

        for (path, module) in &project.modules {
            let mut header: Option<Value> = None;
            for (fi, file) in module.files.iter().enumerate() {
                for (si, statement) in file.ast.statements.iter().enumerate() {
                    match statement {
                        Statement::Import(_) => {}
                        Statement::Moved(m) => {
                            let scope = if m.module {
                                "module"
                            } else if m.from.member.is_some() {
                                "member"
                            } else {
                                "declaration"
                            };
                            moves.push(json!({ "from": address_text(&m.from), "to": address_text(&m.to), "scope": scope }));
                        }
                        Statement::Module(h) => {
                            if header.is_some() {
                                return Err(GraphError::at(
                                    file,
                                    h.span,
                                    "a module has one header",
                                ));
                            }
                            let schema = schema::core_kind("module").expect("core");
                            let mut ctx = self.new_ctx(file, path, path);
                            let (fields, _, _) =
                                self.body(&mut ctx, &schema, &schema.fields.clone(), &h.body)?;
                            header = Some(Value::Object(fields));
                        }
                        Statement::Project(p) => {
                            if !path.is_empty() {
                                return Err(GraphError::at(
                                    file,
                                    p.span,
                                    "project belongs in the root module",
                                ));
                            }
                            let schema = schema::core_kind("project").expect("core");
                            let mut ctx = self.new_ctx(file, path, &p.name.text);
                            let (fields, _, _) =
                                self.body(&mut ctx, &schema, &schema.fields.clone(), &p.body)?;
                            let l = Self::simple(
                                file,
                                &p.name.text,
                                "core:project",
                                Sort::Context,
                                fields,
                                ctx,
                                p.span,
                            );
                            insert(&mut decls, l, file, p.span)?;
                        }
                        Statement::Stack(s) => {
                            if !path.is_empty() {
                                return Err(GraphError::at(
                                    file,
                                    s.span,
                                    "stack belongs in the root module",
                                ));
                            }
                            let schema = schema::core_kind("stack").expect("core");
                            let mut ctx = self.new_ctx(file, path, "stack");
                            let (fields, _, _) =
                                self.body(&mut ctx, &schema, &schema.fields.clone(), &s.body)?;
                            let l = Self::simple(
                                file,
                                "stack",
                                "core:stack",
                                Sort::Managed,
                                fields,
                                ctx,
                                s.span,
                            );
                            insert(&mut decls, l, file, s.span)?;
                        }
                        Statement::Use(u) => {
                            let capability = u.capability.text.clone();
                            let mut ctx = self.new_ctx(file, path, &capability);
                            let mut fields = Map::new();
                            fields.insert("package".into(), json!(lit_text(&u.library)));
                            fields.insert("capability".into(), json!(capability));
                            if let Some(options) = &u.options {
                                fields.insert(
                                    "options".into(),
                                    self.generic(
                                        &mut ctx,
                                        &Expr::Record(options.clone()),
                                        "options",
                                    )?,
                                );
                            }
                            let provider = self.scope.capabilities[&capability].provider;
                            let binding = provider
                                .and_then(|p| self.binding_body(p, &capability).map(|b| (p, b)));
                            let managed = binding.is_some_and(|(_, b)| {
                                b.members.iter().any(|m| m.name.text == "outputs")
                            });
                            if let Some((p, b)) = binding {
                                for m in &b.members {
                                    if let (
                                        true,
                                        MemberValue::Field(Field {
                                            value: Expr::Selector(sel),
                                            ..
                                        }),
                                    ) = (m.name.text == "collects", &m.value)
                                    {
                                        collects.push(Collects {
                                            use_address: capability.clone(),
                                            package: p,
                                            selector: sel,
                                        });
                                    }
                                }
                            }
                            let sort = if managed {
                                Sort::Managed
                            } else {
                                Sort::Context
                            };
                            let l = Self::simple(
                                file,
                                &capability,
                                "core:use",
                                sort,
                                fields,
                                ctx,
                                u.span,
                            );
                            insert(&mut decls, l, file, u.span)?;
                        }
                        Statement::Decl(d) => {
                            let info = self
                                .scope
                                .decls
                                .iter()
                                .find(|i| i.module == *path && i.file == fi && i.statement == si)
                                .cloned()
                                .expect("indexed by scope");
                            match &d.item {
                                Exportable::Named(n) => {
                                    let l =
                                        self.named(file, path, &info.address, d.export, n, d.span)?;
                                    if l.kind == "core:rule" {
                                        if let Some(MemberValue::Field(Field {
                                            value: Expr::Selector(sel),
                                            ..
                                        })) = n
                                            .body
                                            .members
                                            .iter()
                                            .find(|m| m.name.text == "applies_to")
                                            .map(|m| &m.value)
                                        {
                                            rules.push(RuleSelector {
                                                address: info.address.clone(),
                                                module: path.clone(),
                                                selector: sel,
                                                file,
                                            });
                                        }
                                    }
                                    insert(&mut decls, l, file, d.span)?;
                                }
                                Exportable::Type { value, .. } => {
                                    let mut ctx = self.new_ctx(file, path, &info.address);
                                    let ty = self.lower_type(&mut ctx, value, "type")?;
                                    let mut fields = Map::new();
                                    fields.insert("type".into(), ty);
                                    let mut l = Self::simple(
                                        file,
                                        &info.address,
                                        "core:type",
                                        Sort::Context,
                                        fields,
                                        ctx,
                                        d.span,
                                    );
                                    l.value.insert("module".into(), json!(path));
                                    l.value.insert("exported".into(), json!(d.export));
                                    l.module = path.clone();
                                    l.name = info.name.clone();
                                    insert(&mut decls, l, file, d.span)?;
                                }
                                Exportable::Let { .. } => {} // inlined where used
                                _ => unreachable!("rejected by scope"),
                            }
                        }
                    }
                }
            }
            let imports: Vec<Value> = self.scope.imports[path]
                .iter()
                .map(|(alias, target)| {
                    let source = match target {
                        Target::Module(m) => format!("module:{m}"),
                        Target::Package(p) => format!("package:{}", project.packages[*p].id),
                        _ => unreachable!("imports name modules or packages"),
                    };
                    json!({ "alias": alias, "source": source })
                })
                .collect();
            let mut m = Map::new();
            m.insert("path".into(), json!(path));
            if let Some(h) = header {
                m.insert("header".into(), h);
            }
            m.insert("imports".into(), Value::Array(imports));
            modules_out.push(Value::Object(m));
        }

        // Rule attachment: managed declarations of the rule's kinds, in its module and below.
        let mut applies: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for rule in &rules {
            let owner = Owner::Module(rule.module.clone());
            let kinds = self.selector_kinds(rule.file, rule.selector, &owner)?;
            for (address, l) in &decls {
                let in_scope = rule.module.is_empty()
                    || l.module == rule.module
                    || l.module.starts_with(&format!("{}.", rule.module));
                if l.sort != Sort::Managed || l.kind == "core:rule" || !in_scope {
                    continue;
                }
                if self.selected(&kinds, rule.selector, l, &owner, rule.file) {
                    applies
                        .entry(address.clone())
                        .or_default()
                        .push(rule.address.clone());
                }
            }
        }
        for (address, mut rules) in applies {
            rules.sort();
            decls
                .get_mut(&address)
                .expect("present")
                .value
                .insert("applies".into(), json!(rules));
        }

        // Binding membership: `collects` edges from the use declaration.
        for c in &collects {
            let owner = Owner::Package(c.package);
            let file = &project.packages[c.package].files[0];
            let kinds = self.selector_kinds(file, c.selector, &owner)?;
            let members: Vec<String> = decls
                .iter()
                .filter(|(_, l)| {
                    l.sort == Sort::Managed && self.selected(&kinds, c.selector, l, &owner, file)
                })
                .map(|(a, _)| a.clone())
                .collect();
            let use_decl = decls.get_mut(&c.use_address).expect("present");
            let mut edges = use_decl.value["edges"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for m in members {
                edges.push(json!({ "relation": "collects", "target": m, "via": "collects" }));
            }
            sort_by_canonical(&mut edges);
            use_decl.value.insert("edges".into(), Value::Array(edges));
        }

        let lock = intlang_canon::digest(&project.lock)
            .map_err(|e| GraphError::plain(format!("spec.lock: {e}")))?;
        let mut kinds = Map::new();
        for l in decls.values() {
            if kinds.contains_key(&l.kind) {
                continue;
            }
            let entry = match l.kind.strip_prefix("core:") {
                Some("use") => schema::use_interface_entry(),
                Some(core) => schema::core_kind(core)
                    .ok_or_else(|| GraphError::plain(format!("unknown core kind {core}")))?
                    .interface_entry(),
                None => self
                    .kinds
                    .get(&l.kind)
                    .ok_or_else(|| GraphError::plain(format!("no schema for kind {}", l.kind)))?
                    .interface_entry(),
            };
            kinds.insert(l.kind.clone(), entry);
        }
        Ok(json!({
            "schema": "intlang.graph/1",
            "lock": lock,
            "kinds": kinds,
            "modules": modules_out,
            "declarations": decls.into_values().map(|l| Value::Object(l.value)).collect::<Vec<_>>(),
            "moves": moves,
        }))
    }

    fn selected(
        &self,
        kinds: &[String],
        selector: &Selector,
        l: &Lowered,
        owner: &Owner,
        file: &SourceFile,
    ) -> bool {
        if !(kinds.iter().any(|k| k == "*") || kinds.contains(&l.kind)) {
            return false;
        }
        let Some(predicate) = &selector.predicate else {
            return true;
        };
        let empty = Map::new();
        let fields = l
            .value
            .get("fields")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let candidate = Candidate {
            module: &l.module,
            name: &l.name,
            kind: &l.kind,
            fields,
        };
        let operand = |o: &Operand| self.operand_value(o, owner, file);
        selectors::matches(predicate, &candidate, &operand)
    }

    fn operand_value(&self, o: &Operand, owner: &Owner, file: &SourceFile) -> Value {
        match o {
            Operand::String(s) => Value::String(lit_text(s)),
            Operand::Number { text } => number(text),
            Operand::Bool { value } => Value::Bool(*value),
            Operand::Null => Value::Null,
            Operand::Path(p) => self
                .scope
                .resolve(owner, p, file)
                .ok()
                .and_then(|t| self.scope.canonical(&t))
                .map(Value::String)
                .unwrap_or(Value::Null),
            Operand::List { items } => Value::Array(
                items
                    .iter()
                    .map(|i| self.operand_value(i, owner, file))
                    .collect(),
            ),
        }
    }
}

/// Lowers a loaded project to its canonical graph.
pub fn lower_project(project: &Project) -> GResult<Value> {
    let scope = Scope::build(project)?;
    Lowerer {
        project,
        scope,
        kinds: BTreeMap::new(),
    }
    .run()
}
