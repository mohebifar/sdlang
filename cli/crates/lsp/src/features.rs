//! The requests, answered from an [`Analysis`].

use std::collections::HashMap;

use lsp_types::{
    CompletionItem, CompletionItemKind, DocumentHighlight, DocumentHighlightKind, DocumentSymbol,
    Documentation, FoldingRange, FoldingRangeKind, Hover, HoverContents, InsertTextFormat,
    Location, MarkupContent, MarkupKind, Range, SemanticToken, SemanticTokenModifier,
    SemanticTokenType, SymbolInformation, SymbolKind, TextEdit, Url,
};
use sdlang_syntax::ast::{Block, Exportable, Expr, File, MemberValue, Record, Statement};
use sdlang_syntax::lexer::{self, TokenKind};
use sdlang_syntax::strings::StringKind;

use crate::analysis::{statement_parts, Analysis, Def, Sym};
use crate::docs;
use crate::text::path_to_url;

fn markdown(value: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

pub fn location(a: &Analysis, def: &Def) -> Option<Location> {
    let uri = path_to_url(&def.path)?;
    let range = match def.file {
        Some(f) => a.files[f].index.range(&a.files[f].text, def.name_span),
        None => Range::default(),
    };
    Some(Location { uri, range })
}

fn occ_location(a: &Analysis, file: usize, span: sdlang_syntax::ast::Span) -> Option<Location> {
    let f = &a.files[file];
    Some(Location {
        uri: path_to_url(&f.path)?,
        range: f.index.range(&f.text, span),
    })
}

fn relative(a: &Analysis, path: &std::path::Path) -> String {
    path.strip_prefix(&a.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// A declaration's source, cut to `max` lines.
fn excerpt(a: &Analysis, def: &Def, max: usize) -> Option<String> {
    let f = &a.files[def.file?];
    let text = f.text.get(def.full_span.start..def.full_span.end)?;
    let lines: Vec<&str> = text.lines().collect();
    let mut out = lines
        .iter()
        .take(max)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if lines.len() > max {
        out.push_str("\n  \u{2026}");
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------
// Hover.

pub fn hover(a: &Analysis, file: usize, offset: usize) -> Option<Hover> {
    if let Some(occ) = a.occ_at(file, offset) {
        return sym_hover(a, &occ.sym).map(markdown);
    }
    if let Some(m) = a.member_at(file, offset) {
        return member_hover(a, m).map(markdown);
    }
    keyword_hover(&a.files[file].text, offset).map(markdown)
}

pub fn sym_hover(a: &Analysis, sym: &Sym) -> Option<String> {
    let def = a.defs.get(sym);
    let head = |label: &str, name: &str| format!("**{label}** `{name}`");
    Some(match sym {
        Sym::Decl(addr) => {
            let def = def?;
            let mut s = head(&def.label, addr);
            if let Some(summary) = &def.summary {
                s.push_str(&format!("\n\n{summary}"));
            }
            if let Some(code) = excerpt(a, def, 24) {
                s.push_str(&format!("\n\n```sdlang\n{code}\n```"));
            }
            s.push_str(&format!("\n\n{}", relative(a, &def.path)));
            s
        }
        Sym::PackageItem { package, name } => {
            let def = def?;
            let mut s = format!("{} from package `{package}`", head(&def.label, name));
            if let Some(summary) = &def.summary {
                s.push_str(&format!("\n\n{summary}"));
            }
            if let Some(code) = excerpt(a, def, 24) {
                s.push_str(&format!("\n\n```sdlang\n{code}\n```"));
            }
            s
        }
        Sym::Module(m) => {
            let mut s = head("module", if m.is_empty() { "(root)" } else { m });
            if let Some(summary) = def.and_then(|d| d.summary.as_ref()) {
                s.push_str(&format!("\n\n{summary}"));
            }
            let files: Vec<String> = a
                .files
                .iter()
                .filter(|f| f.module.as_deref() == Some(m.as_str()))
                .map(|f| format!("- {}", relative(a, &f.path)))
                .collect();
            if !files.is_empty() {
                s.push_str(&format!("\n\n{}", files.join("\n")));
            }
            s
        }
        Sym::Package(id) => {
            let mut s = head("package", id);
            if let Some(d) = def {
                if let Some(summary) = &d.summary {
                    s.push_str(&format!("\n\n{summary}"));
                }
                s.push_str(&format!("\n\n{}", relative(a, &d.path)));
            }
            s
        }
        Sym::Capability(c) => {
            let mut s = head("capability", c);
            if let Some(summary) = def.and_then(|d| d.summary.as_ref()) {
                s.push_str(&format!(", {summary}"));
            }
            s
        }
        Sym::Skill { package, name } => {
            let mut s = format!("{} from package `{package}`", head("skill", name));
            if let Some(text) = def.and_then(|d| std::fs::read_to_string(&d.path).ok()) {
                let lines: Vec<&str> = text.lines().take(14).collect();
                s.push_str(&format!("\n\n---\n\n{}", lines.join("\n")));
            }
            s
        }
        Sym::Builtin(b) => format!(
            "{}\n\n{}",
            head("builtin", b),
            docs::builtin(b).unwrap_or("")
        ),
        Sym::CoreKind(k) => format!(
            "{}\n\n{}",
            head("core kind", k),
            docs::core_kind(k).unwrap_or("")
        ),
        Sym::CoreAnnotation(n) => format!(
            "{}\n\n{}",
            head("annotation", n),
            docs::core_annotation(n).unwrap_or("")
        ),
    })
}

fn member_hover(a: &Analysis, m: &crate::analysis::MemberOcc) -> Option<String> {
    let name = m.path.last()?;
    if m.path.len() == 1 {
        let mut s = format!("**field** `{name}`");
        if let Some(doc) = docs::field(name) {
            s.push_str(&format!("\n\n{doc}"));
        }
        // A package kind's own definition of the field.
        if let Some(def) = m.kind.as_ref().and_then(|k| a.defs.get(k)) {
            if let Some(src) = kind_field_source(a, def, name) {
                s.push_str(&format!("\n\n```sdlang\n{src}\n```"));
            }
        }
        return Some(s);
    }
    let Some(Sym::Decl(addr)) = &m.decl else {
        return Some(format!("`{}`", m.path.join(".")));
    };
    let member = format!("{addr}#{}", m.path.join("."));
    Some(match m.path[0].as_str() {
        "expect" => format!(
            "**requirement** `{member}`\n\nCovered by a test case tagged `[case:{name}]`, or by review."
        ),
        "errors" => format!("**error case** `{member}`"),
        _ => format!("`{member}`"),
    })
}

fn kind_field_source(a: &Analysis, def: &Def, field: &str) -> Option<String> {
    let f = &a.files[def.file?];
    let statement = f
        .ast
        .as_ref()?
        .statements
        .iter()
        .find(|s| s.span() == def.full_span)?;
    let Statement::Decl(decl) = statement else {
        return None;
    };
    let Exportable::Kind { body, .. } = &decl.item else {
        return None;
    };
    let fields = body.members.iter().find(|m| m.name.text == "fields")?;
    let MemberValue::Section(section) = &fields.value else {
        return None;
    };
    let member = section.members.iter().find(|m| m.name.text == field)?;
    f.text
        .get(member.span.start..member.span.end)
        .map(str::to_string)
}

fn keyword_hover(text: &str, offset: usize) -> Option<String> {
    let tokens = lexer::lex(text).ok()?;
    let token = tokens
        .iter()
        .find(|t| t.span.start <= offset && offset <= t.span.end)?;
    let TokenKind::Word(w) = &token.kind else {
        return None;
    };
    docs::keyword(w).map(|d| format!("**keyword** `{w}`\n\n{d}"))
}

// ---------------------------------------------------------------------------------------
// Navigation.

pub fn definition(a: &Analysis, file: usize, offset: usize) -> Option<Location> {
    let occ = a.occ_at(file, offset)?;
    location(a, a.defs.get(&occ.sym)?)
}

pub fn references(a: &Analysis, file: usize, offset: usize, include_decl: bool) -> Vec<Location> {
    let Some(occ) = a.occ_at(file, offset) else {
        return Vec::new();
    };
    let sym = occ.sym.clone();
    let mut out: Vec<Location> = a
        .occs
        .iter()
        .filter(|o| o.sym == sym && (include_decl || !o.def))
        .filter_map(|o| occ_location(a, o.file, o.span))
        .collect();
    if include_decl && !a.occs.iter().any(|o| o.sym == sym && o.def) {
        if let Some(l) = a.defs.get(&sym).and_then(|d| location(a, d)) {
            out.insert(0, l);
        }
    }
    out
}

pub fn highlights(a: &Analysis, file: usize, offset: usize) -> Vec<DocumentHighlight> {
    let Some(occ) = a.occ_at(file, offset) else {
        return Vec::new();
    };
    let f = &a.files[file];
    a.occs
        .iter()
        .filter(|o| o.file == file && o.sym == occ.sym)
        .map(|o| DocumentHighlight {
            range: f.index.range(&f.text, o.span),
            kind: Some(if o.def {
                DocumentHighlightKind::WRITE
            } else {
                DocumentHighlightKind::READ
            }),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Rename: a declaration and every reference to it, plus a `moved` statement so the
// declaration keeps its identity (docs/spec.md, Moves: a rename without `moved` is a delete
// plus a create).

pub fn prepare_rename(a: &Analysis, file: usize, offset: usize) -> Result<(Range, String), String> {
    if a.files.iter().any(|f| f.stale) {
        return Err("a file does not parse; fix it before renaming".into());
    }
    let occ = a.occ_at(file, offset).ok_or("nothing to rename here")?;
    let Sym::Decl(addr) = &occ.sym else {
        return Err("only declarations, types and lets of this project can be renamed".into());
    };
    let f = &a.files[file];
    let name = addr.rsplit('.').next().unwrap_or(addr);
    if f.text.get(occ.span.start..occ.span.end) != Some(name) {
        return Err("nothing to rename here".into());
    }
    Ok((f.index.range(&f.text, occ.span), name.to_string()))
}

pub fn rename(
    a: &Analysis,
    file: usize,
    offset: usize,
    new_name: &str,
) -> Result<HashMap<Url, Vec<TextEdit>>, String> {
    prepare_rename(a, file, offset)?;
    let valid = new_name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && new_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !lexer::is_keyword(new_name);
    if !valid {
        return Err(format!("'{new_name}' is not a valid name"));
    }
    let sym = a
        .occ_at(file, offset)
        .map(|o| o.sym.clone())
        .ok_or("nothing to rename here")?;
    let Sym::Decl(old) = &sym else { unreachable!() };
    let new_addr = match old.rsplit_once('.') {
        Some((module, _)) => format!("{module}.{new_name}"),
        None => new_name.to_string(),
    };
    if a.defs.contains_key(&Sym::Decl(new_addr.clone())) {
        return Err(format!("'{new_addr}' already exists"));
    }
    let mut edits: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for o in a.occs.iter().filter(|o| o.sym == sym && !o.canonical) {
        let f = &a.files[o.file];
        let Some(uri) = path_to_url(&f.path) else {
            continue;
        };
        edits.entry(uri).or_default().push(TextEdit {
            range: f.index.range(&f.text, o.span),
            new_text: new_name.to_string(),
        });
    }
    if let Some(def_file) = a.defs.get(&sym).and_then(|d| d.file) {
        let f = &a.files[def_file];
        if let Some(uri) = path_to_url(&f.path) {
            let end = f.index.position(&f.text, f.text.len());
            let sep = if f.text.ends_with('\n') || f.text.is_empty() {
                ""
            } else {
                "\n"
            };
            edits.entry(uri).or_default().push(TextEdit {
                range: Range::new(end, end),
                new_text: format!("{sep}\nmoved {old} -> {new_addr}\n"),
            });
        }
    }
    Ok(edits)
}

// ---------------------------------------------------------------------------------------
// Symbols.

fn symbol_kind(label: &str) -> SymbolKind {
    match label.trim_start_matches("extern ") {
        "action" => SymbolKind::FUNCTION,
        "entity" => SymbolKind::STRUCT,
        "lib" => SymbolKind::MODULE,
        "type" => SymbolKind::TYPE_PARAMETER,
        "let" => SymbolKind::CONSTANT,
        "kind" => SymbolKind::CLASS,
        "annotation" => SymbolKind::PROPERTY,
        "implements" => SymbolKind::INTERFACE,
        "rule" => SymbolKind::EVENT,
        "module" | "project" | "stack" => SymbolKind::NAMESPACE,
        _ => SymbolKind::OBJECT,
    }
}

#[allow(deprecated)]
fn doc_symbol(
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    range: Range,
    selection: Range,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: selection,
        children: (!children.is_empty()).then_some(children),
    }
}

pub fn document_symbols(a: &Analysis, file: usize) -> Vec<DocumentSymbol> {
    let f = &a.files[file];
    let Some(ast) = &f.ast else { return Vec::new() };
    let r = |s| f.index.range(&f.text, s);
    let members = |block: &Block| -> Vec<DocumentSymbol> { block_symbols(block, &r) };
    let mut out = Vec::new();
    for statement in &ast.statements {
        match statement {
            Statement::Project(p) => out.push(doc_symbol(
                p.name.text.clone(),
                Some("project".into()),
                SymbolKind::PACKAGE,
                r(p.span),
                r(p.name.span),
                members(&p.body),
            )),
            Statement::Package(p) => out.push(doc_symbol(
                p.name.text.clone(),
                Some("package".into()),
                SymbolKind::PACKAGE,
                r(p.span),
                r(p.name.span),
                members(&p.body),
            )),
            Statement::Stack(s) => out.push(doc_symbol(
                "stack".into(),
                None,
                SymbolKind::PACKAGE,
                r(s.span),
                r(s.span),
                members(&s.body),
            )),
            Statement::Module(m) => out.push(doc_symbol(
                "module".into(),
                None,
                SymbolKind::NAMESPACE,
                r(m.span),
                r(m.span),
                members(&m.body),
            )),
            Statement::Use(u) => out.push(doc_symbol(
                u.capability.text.clone(),
                Some(format!("use {}", u.library.to_source_text())),
                SymbolKind::INTERFACE,
                r(u.span),
                r(u.capability.span),
                Vec::new(),
            )),
            _ => {
                if let Some((name, label, body)) = statement_parts(statement) {
                    let children = body.map(members).unwrap_or_default();
                    out.push(doc_symbol(
                        name.text.clone(),
                        Some(label.clone()),
                        symbol_kind(&label),
                        r(statement.span()),
                        r(name.span),
                        children,
                    ));
                }
            }
        }
    }
    out
}

fn block_symbols(
    block: &Block,
    r: &dyn Fn(sdlang_syntax::ast::Span) -> Range,
) -> Vec<DocumentSymbol> {
    block
        .members
        .iter()
        .map(|m| match &m.value {
            MemberValue::Field(_) => doc_symbol(
                m.name.text.clone(),
                None,
                SymbolKind::FIELD,
                r(m.span),
                r(m.name.span),
                Vec::new(),
            ),
            MemberValue::Section(b) => {
                let kids = b
                    .members
                    .iter()
                    .map(|k| {
                        doc_symbol(
                            k.name.text.clone(),
                            None,
                            SymbolKind::KEY,
                            r(k.span),
                            r(k.name.span),
                            Vec::new(),
                        )
                    })
                    .collect();
                doc_symbol(
                    m.name.text.clone(),
                    None,
                    SymbolKind::NAMESPACE,
                    r(m.span),
                    r(m.name.span),
                    kids,
                )
            }
        })
        .collect()
}

#[allow(deprecated)]
pub fn workspace_symbols(analyses: &[&Analysis], query: &str) -> Vec<SymbolInformation> {
    let q = query.to_lowercase();
    let matches = |name: &str| {
        let mut it = name.to_lowercase().chars().collect::<Vec<_>>().into_iter();
        q.chars().all(|c| it.any(|n| n == c))
    };
    let mut out = Vec::new();
    for a in analyses {
        for (sym, def) in &a.defs {
            let (name, container) = match sym {
                Sym::Decl(addr) => match addr.rsplit_once('.') {
                    Some((m, n)) => (n.to_string(), Some(m.to_string())),
                    None => (addr.clone(), None),
                },
                Sym::PackageItem { package, name } => (name.clone(), Some(package.clone())),
                _ => continue,
            };
            if !matches(&name) {
                continue;
            }
            if let Some(location) = location(a, def) {
                out.push(SymbolInformation {
                    name,
                    kind: symbol_kind(&def.label),
                    tags: None,
                    deprecated: None,
                    location,
                    container_name: container,
                });
            }
        }
    }
    out.sort_by(|x, y| x.name.cmp(&y.name));
    out.truncate(200);
    out
}

// ---------------------------------------------------------------------------------------
// Folding.

pub fn folding(a: &Analysis, file: usize) -> Vec<FoldingRange> {
    let f = &a.files[file];
    let mut spans = Vec::new();
    if let Some(ast) = &f.ast {
        collect_folds(ast, &mut spans);
    }
    let mut out: Vec<FoldingRange> = spans
        .into_iter()
        .filter_map(|(s, kind)| {
            let (start, end) = (
                f.index.position(&f.text, s.start).line,
                f.index.position(&f.text, s.end).line,
            );
            (end > start).then(|| FoldingRange {
                start_line: start,
                start_character: None,
                end_line: end - 1,
                end_character: None,
                kind,
                collapsed_text: None,
            })
        })
        .collect();
    // Runs of comment lines.
    let lines: Vec<&str> = f.text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("//") {
            let start = i;
            while i + 1 < lines.len() && lines[i + 1].trim_start().starts_with("//") {
                i += 1;
            }
            if i > start {
                out.push(FoldingRange {
                    start_line: start as u32,
                    start_character: None,
                    end_line: i as u32,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Comment),
                    collapsed_text: None,
                });
            }
        }
        i += 1;
    }
    out
}

type Fold = (sdlang_syntax::ast::Span, Option<FoldingRangeKind>);

fn collect_folds(ast: &File, out: &mut Vec<Fold>) {
    let imports: Vec<_> = ast
        .statements
        .iter()
        .filter(|s| matches!(s, Statement::Import(_)))
        .collect();
    if let (Some(first), Some(last)) = (imports.first(), imports.last()) {
        // Imports fold to their first line; the range ends after the last one.
        out.push((
            first
                .span()
                .to(last.span())
                .to(sdlang_syntax::ast::Span::new(
                    last.span().end,
                    last.span().end + 1,
                )),
            Some(FoldingRangeKind::Imports),
        ));
    }
    for statement in &ast.statements {
        match statement {
            Statement::Project(p) => fold_block(&p.body, out),
            Statement::Package(p) => fold_block(&p.body, out),
            Statement::Stack(s) => fold_block(&s.body, out),
            Statement::Module(m) => fold_block(&m.body, out),
            Statement::Use(u) => {
                if let Some(r) = &u.options {
                    fold_record(r, out);
                }
            }
            Statement::Decl(d) => match &d.item {
                Exportable::Let { value, .. } | Exportable::Type { value, .. } => {
                    fold_expr(value, out)
                }
                Exportable::Kind { body, .. }
                | Exportable::Annotation { body, .. }
                | Exportable::Implements { body, .. } => fold_block(body, out),
                Exportable::Named(n) => fold_block(&n.body, out),
            },
            _ => {}
        }
    }
}

fn fold_block(b: &Block, out: &mut Vec<Fold>) {
    out.push((b.span, None));
    for m in &b.members {
        match &m.value {
            MemberValue::Field(f) => fold_expr(&f.value, out),
            MemberValue::Section(s) => fold_block(s, out),
        }
    }
}

fn fold_record(r: &Record, out: &mut Vec<Fold>) {
    out.push((r.span, None));
    for f in &r.fields {
        fold_expr(&f.field.value, out);
    }
}

fn fold_expr(e: &Expr, out: &mut Vec<Fold>) {
    match e {
        Expr::Record(r) => fold_record(r, out),
        Expr::List { items, span } => {
            out.push((*span, None));
            items.iter().for_each(|i| fold_expr(i, out));
        }
        Expr::String(s) if s.kind == StringKind::Prose => {
            // Keep the closing `"""` visible.
            out.push((s.span, None));
        }
        Expr::Path {
            record: Some(r), ..
        } => fold_record(r, out),
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------
// Semantic tokens: what TextMate cannot know, such as which names are kinds, types,
// declarations or modules.

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::DECORATOR,
    SemanticTokenType::ENUM_MEMBER,
];
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFAULT_LIBRARY,
];

fn token_of(a: &Analysis, sym: &Sym) -> (u32, u32) {
    const LIB: u32 = 2;
    match sym {
        Sym::Module(_) | Sym::Package(_) | Sym::Capability(_) => (0, 0),
        Sym::Builtin(b) if b == "now" => (4, LIB),
        Sym::Builtin(_) => (1, LIB),
        Sym::CoreKind(_) => (2, LIB),
        Sym::CoreAnnotation(_) => (6, LIB),
        Sym::Skill { .. } => (7, 0),
        Sym::Decl(_) | Sym::PackageItem { .. } => match a.defs.get(sym).map(|d| d.label.as_str()) {
            Some("type") => (1, 0),
            Some("let") => (4, 0),
            Some("kind") => (2, 0),
            Some("annotation") => (6, 0),
            Some("entity") => (1, 0),
            _ => (3, 0),
        },
    }
}

pub fn semantic_tokens(a: &Analysis, file: usize) -> Vec<SemanticToken> {
    let f = &a.files[file];
    let mut raw: Vec<(usize, usize, u32, u32)> = a
        .occs
        .iter()
        .filter(|o| o.file == file)
        .map(|o| {
            let (ty, mut mods) = token_of(a, &o.sym);
            if o.def {
                mods |= 1;
            }
            (o.span.start, o.span.end, ty, mods)
        })
        .collect();
    raw.extend(a.members.iter().filter(|m| m.file == file).map(|m| {
        let ty = if m.path.len() > 1 && matches!(m.path[0].as_str(), "expect" | "errors") {
            7
        } else {
            5
        };
        (m.span.start, m.span.end, ty, 0)
    }));
    raw.sort_by_key(|t| (t.0, t.1));
    raw.dedup_by_key(|t| t.0);
    let mut out = Vec::new();
    let (mut line, mut col) = (0u32, 0u32);
    for (start, end, ty, mods) in raw {
        let s = f.index.position(&f.text, start);
        let e = f.index.position(&f.text, end);
        if s.line != e.line {
            continue;
        }
        let delta_line = s.line - line;
        let delta_start = if delta_line == 0 {
            s.character - col
        } else {
            s.character
        };
        out.push(SemanticToken {
            delta_line,
            delta_start,
            length: e.character - s.character,
            token_type: ty,
            token_modifiers_bitset: mods,
        });
        line = s.line;
        col = s.character;
    }
    out
}

// ---------------------------------------------------------------------------------------
// Completion, from the text before the cursor (it need not parse).

const STATEMENTS: &[&str] = &[
    "import", "export", "extern", "use", "let", "type", "module", "project", "stack", "moved",
];

const SNIPPETS: &[(&str, &str, &str)] = &[
    ("action", "action ${1:name} {\n  summary: \"$2\"\n  input: { $3 }\n  output: { $4 }\n  expect {\n    ${5:key}: \"$6\"\n  }\n}", "An action with input, output and a requirement"),
    ("entity", "entity ${1:Name} {\n  summary: \"$2\"\n  fields {\n    id: uuid @id\n    $3\n  }\n}", "An entity with an id field"),
    ("lib", "lib ${1:name} {\n  summary: \"$2\"\n  exports: [\"$3\"]\n}", "A lib"),
    ("rule", "rule ${1:name} {\n  applies_to: select ${2:action}\n  prompt: \"\"\"\n    $3\n  \"\"\"\n}", "A rule"),
    ("module", "module {\n  summary: \"$1\"\n}", "A module header"),
];

fn completion(
    label: &str,
    kind: CompletionItemKind,
    detail: Option<String>,
    doc: Option<String>,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        documentation: doc.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        ..Default::default()
    }
}

fn sym_item(a: &Analysis, name: &str, sym: &Sym) -> CompletionItem {
    let def = a.defs.get(sym);
    let (kind, detail) = match sym {
        Sym::Module(_) => (CompletionItemKind::MODULE, "module".to_string()),
        Sym::Package(id) => (CompletionItemKind::MODULE, format!("package {id}")),
        Sym::Capability(_) => (CompletionItemKind::INTERFACE, "capability".into()),
        Sym::Skill { .. } => (CompletionItemKind::FILE, "skill".into()),
        Sym::Builtin(_) => (CompletionItemKind::TYPE_PARAMETER, "builtin".into()),
        Sym::CoreKind(_) => (CompletionItemKind::CLASS, "core kind".into()),
        Sym::CoreAnnotation(_) => (CompletionItemKind::PROPERTY, "annotation".into()),
        Sym::Decl(addr) => {
            let label = def.map(|d| d.label.clone()).unwrap_or_default();
            let kind = match label.as_str() {
                "type" | "entity" => CompletionItemKind::STRUCT,
                "let" => CompletionItemKind::CONSTANT,
                "action" => CompletionItemKind::FUNCTION,
                _ => CompletionItemKind::REFERENCE,
            };
            (kind, format!("{label} {addr}"))
        }
        Sym::PackageItem { package, .. } => {
            let label = def.map(|d| d.label.clone()).unwrap_or_default();
            let kind = if label == "kind" {
                CompletionItemKind::CLASS
            } else {
                CompletionItemKind::REFERENCE
            };
            (kind, format!("{label} from {package}"))
        }
    };
    let doc = def.and_then(|d| d.summary.clone()).or_else(|| match sym {
        Sym::Builtin(b) => docs::builtin(b).map(String::from),
        Sym::CoreKind(k) => docs::core_kind(k).map(String::from),
        Sym::CoreAnnotation(n) => docs::core_annotation(n).map(String::from),
        _ => None,
    });
    completion(name, kind, Some(detail), doc)
}

/// Brace depth at the end of `text`, and the text of each open block's first line,
/// skipping strings and comments.
fn open_blocks(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut stack = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' if text[i..].starts_with("\"\"\"") => {
                i += 3;
                match text[i..].find("\"\"\"") {
                    Some(k) => i += k + 2,
                    None => return stack,
                }
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'{' | b'[' => stack.push(i),
            b'}' | b']' => {
                stack.pop();
            }
            _ => {}
        }
        i += 1;
    }
    stack
}

fn line_of(text: &str, offset: usize) -> &str {
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = text[offset..].find('\n').map_or(text.len(), |i| offset + i);
    &text[start..end]
}

/// The kind of the declaration whose header is `header` (`export nextjs.page home {`).
fn header_kind(a: &Analysis, module: &str, header: &str) -> Option<Sym> {
    let words: Vec<&str> = header
        .split_whitespace()
        .filter(|w| !matches!(*w, "export" | "extern"))
        .collect();
    let (kind, name) = (words.first()?, words.get(1)?.trim_end_matches('{'));
    let addr = if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}.{name}")
    };
    if let Some(k) = a.defs.get(&Sym::Decl(addr)).and_then(|d| d.kind.clone()) {
        return Some(k);
    }
    match kind.split_once('.') {
        None => Some(Sym::CoreKind(kind.to_string())),
        Some((first, rest)) => {
            let bound = a
                .bindings
                .get(module)?
                .iter()
                .find(|(n, _)| n == first)?
                .1
                .clone();
            a.children
                .get(&bound)?
                .iter()
                .find(|(n, _)| n == rest)
                .map(|(_, s)| s.clone())
        }
    }
}

pub fn complete(a: &Analysis, module: &str, text: &str, offset: usize) -> Vec<CompletionItem> {
    let text = &text[..offset.min(text.len())];
    let line = &text[text.rfind('\n').map_or(0, |i| i + 1)..];
    let blocks = open_blocks(text);
    let bindings = a.bindings.get(module).cloned().unwrap_or_default();

    // Inside a string: only mentions.
    let quotes = line.matches('"').count() - line.matches("\\\"").count();
    let in_prose = text.matches("\"\"\"").count() % 2 == 1;
    if quotes % 2 == 1 || in_prose {
        let Some(at) = line.rfind("@{") else {
            return Vec::new();
        };
        let partial = &line[at + 2..];
        if partial.contains('}') {
            return Vec::new();
        }
        return path_items(a, &bindings, partial);
    }

    // `@annotation`
    if let Some(at) = line.rfind('@') {
        let word = &line[at + 1..];
        if word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            if word.contains('.') {
                return path_items(a, &bindings, word);
            }
            let mut out: Vec<CompletionItem> = sdlang_graph::scope::CORE_ANNOTATIONS
                .iter()
                .map(|n| sym_item(a, n, &Sym::CoreAnnotation(n.to_string())))
                .collect();
            out.extend(
                bindings
                    .iter()
                    .filter(|(_, s)| matches!(s, Sym::Package(_)))
                    .map(|(n, s)| sym_item(a, n, s)),
            );
            return out;
        }
    }

    // `name.partial`: what the prefix contains.
    let tail: String = line
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if tail.contains('.') {
        return path_items(a, &bindings, &tail);
    }

    let before_word = line[..line.len() - tail.len()].trim();
    let at_line_start = before_word.is_empty()
        || before_word == "export"
        || before_word == "extern"
        || before_word == "export extern";
    if blocks.is_empty() && at_line_start {
        let mut out: Vec<CompletionItem> = STATEMENTS
            .iter()
            .map(|k| {
                completion(
                    k,
                    CompletionItemKind::KEYWORD,
                    None,
                    docs::keyword(k).map(String::from),
                )
            })
            .collect();
        for (label, body, detail) in SNIPPETS {
            out.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some(detail.to_string()),
                insert_text: Some(body.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
        out.extend(
            sdlang_graph::scope::CORE_KINDS
                .iter()
                .map(|k| sym_item(a, k, &Sym::CoreKind(k.to_string()))),
        );
        out.extend(
            bindings
                .iter()
                .filter(|(_, s)| matches!(s, Sym::Package(_) | Sym::Module(_)))
                .map(|(n, s)| sym_item(a, n, s)),
        );
        return out;
    }
    if blocks.len() == 1 && at_line_start {
        // A member of the declaration's body: the fields and sections its kind accepts.
        let header = line_of(text, blocks[0]);
        let present: Vec<String> = text[blocks[0]..]
            .lines()
            .filter_map(|l| {
                let w = l.trim_start();
                let name: String = w
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                (!name.is_empty() && w[name.len()..].trim_start().starts_with([':', '{']))
                    .then_some(name)
            })
            .collect();
        let Some(kind) = header_kind(a, module, header) else {
            return Vec::new();
        };
        return a
            .kind_fields
            .get(&kind)
            .into_iter()
            .flatten()
            .filter(|n| !present.contains(n))
            .map(|n| {
                let mut item = completion(
                    n,
                    CompletionItemKind::FIELD,
                    None,
                    docs::field(n).map(String::from),
                );
                let section = matches!(n.as_str(), "expect" | "errors" | "fields");
                item.insert_text = Some(if section {
                    format!("{n} {{\n  $0\n}}")
                } else {
                    format!("{n}: $0")
                });
                item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                item
            })
            .collect();
    }
    if at_line_start {
        return Vec::new(); // a key in a section or record: free text
    }

    // A value: names in scope, builtins and constants.
    let mut out: Vec<CompletionItem> = bindings.iter().map(|(n, s)| sym_item(a, n, s)).collect();
    out.extend(
        sdlang_graph::scope::BUILTIN_TYPES
            .iter()
            .map(|b| sym_item(a, b, &Sym::Builtin(b.to_string()))),
    );
    out.extend(
        sdlang_graph::scope::CORE_KINDS
            .iter()
            .map(|k| sym_item(a, k, &Sym::CoreKind(k.to_string()))),
    );
    for k in ["ref", "select", "true", "false", "null", "now"] {
        out.push(completion(
            k,
            CompletionItemKind::KEYWORD,
            None,
            docs::keyword(k).or(docs::builtin(k)).map(String::from),
        ));
    }
    out
}

/// Completions after `a.b.` (`prefix` may end in a partial name).
fn path_items(a: &Analysis, bindings: &[(String, Sym)], prefix: &str) -> Vec<CompletionItem> {
    let parts: Vec<&str> = prefix.split('.').collect();
    if parts.len() == 1 {
        return bindings.iter().map(|(n, s)| sym_item(a, n, s)).collect();
    }
    // Unbound (the file may not parse yet): a module of that name.
    let module = Sym::Module(parts[0].to_string());
    let Some(mut sym) = bindings
        .iter()
        .find(|(n, _)| n == parts[0])
        .map(|(_, s)| s.clone())
        .or_else(|| a.children.contains_key(&module).then_some(module))
    else {
        return Vec::new();
    };
    for seg in &parts[1..parts.len() - 1] {
        let Some(next) = a
            .children
            .get(&sym)
            .and_then(|c| c.iter().find(|(n, _)| n == seg))
        else {
            return Vec::new();
        };
        sym = next.1.clone();
    }
    a.children
        .get(&sym)
        .into_iter()
        .flatten()
        .map(|(n, s)| sym_item(a, n, s))
        .collect()
}
