//! One project's analysis: its files, every definition and reference in them, member
//! names, diagnostics and what is in scope where.
//!
//! When the whole project loads, names resolve with the compiler's own scope
//! (`sdlang_graph::scope`), so navigation follows docs/spec.md's resolution rules exactly, and
//! the first compile error from lowering is reported. When it does not load (a file does
//! not parse, a name is bound twice), each file is parsed on its own and names resolve by
//! a simpler rule: the file's module first, then a declaration with that name anywhere.
//!
//! An open buffer that does not parse is analysed from its last text that did, so
//! navigation keeps working while it is being edited; such a file is marked stale.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use sdlang_graph::loader::{self, Overlay, Project};
use sdlang_graph::schema;
use sdlang_graph::scope::{self, Owner, Scope, Target};
use sdlang_graph::GraphError;
use sdlang_syntax::ast::{
    Block, Exportable, Expr, File, ImportSource, MemberValue, Path as SynPath, Predicate, Record,
    SelectSource, Span, Statement,
};
use sdlang_syntax::strings::StringLit;
use sdlang_syntax::{lexer, parse};

use crate::text::LineIndex;

/// What a name denotes, independent of indices into any one load.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sym {
    /// A project declaration, type or let, by address (`shop.add_to_cart`).
    Decl(String),
    /// A top-level name in a package: a kind, annotation, extern, type, ...
    PackageItem {
        package: String,
        name: String,
    },
    Module(String),
    Package(String),
    Capability(String),
    Skill {
        package: String,
        name: String,
    },
    Builtin(String),
    CoreKind(String),
    CoreAnnotation(String),
}

/// An editor buffer.
#[derive(Clone, Debug, Default)]
pub struct Buffer {
    pub text: String,
    /// The last text of this buffer that parsed.
    pub last_good: Option<String>,
}

pub struct FileData {
    pub path: PathBuf,
    /// The normalised text the syntax tree's spans refer to.
    pub text: String,
    pub index: LineIndex,
    pub ast: Option<File>,
    /// The module the file belongs to (`""` for `spec/` itself), if it is a spec file.
    pub module: Option<String>,
    /// The package id, if it is a package's source.
    pub package: Option<String>,
    /// The editor's text differs from `text` (it did not parse); positions are not reliable.
    pub stale: bool,
}

/// A name in the source that denotes `sym`: one path segment, a declaration's name, or a
/// segment of a mention.
#[derive(Clone, Debug)]
pub struct Occ {
    pub file: usize,
    pub span: Span,
    pub sym: Sym,
    /// The defining name of a declaration.
    pub def: bool,
    /// Inside a `moved` statement: a canonical address, which a rename leaves alone.
    pub canonical: bool,
}

/// A field or section name inside a body.
#[derive(Clone, Debug)]
pub struct MemberOcc {
    pub file: usize,
    pub span: Span,
    /// The declaration the body belongs to, and its kind.
    pub decl: Option<Sym>,
    pub kind: Option<Sym>,
    /// Section path from the declaration's body down to this member.
    pub path: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub file: Option<usize>,
    /// For definitions outside any analysed file (a skill's Markdown).
    pub path: PathBuf,
    pub name_span: Span,
    pub full_span: Span,
    /// `action`, `nextjs.page`, `type`, `let`, `kind`, `module`, ...
    pub label: String,
    pub summary: Option<String>,
    pub kind: Option<Sym>,
    pub exported: bool,
}

#[derive(Clone, Debug)]
pub struct Diag {
    pub span: Span,
    pub message: String,
}

pub struct Analysis {
    pub root: PathBuf,
    pub files: Vec<FileData>,
    pub by_path: HashMap<PathBuf, usize>,
    pub occs: Vec<Occ>,
    pub members: Vec<MemberOcc>,
    pub defs: HashMap<Sym, Def>,
    /// Diagnostics by file; paths not in `files` are dropped.
    pub diags: HashMap<PathBuf, Vec<Diag>>,
    /// Names bound in each module.
    pub bindings: BTreeMap<String, Vec<(String, Sym)>>,
    /// What `name.` can be followed by.
    pub children: HashMap<Sym, Vec<(String, Sym)>>,
    /// Field and section names each kind accepts.
    pub kind_fields: HashMap<Sym, Vec<String>>,
    /// Names resolved with the compiler's scope (the project loaded).
    pub semantic: bool,
}

impl Analysis {
    pub fn file(&self, path: &Path) -> Option<usize> {
        self.by_path.get(path).copied()
    }

    /// The occurrence at `offset` in `file` (a span's end counts as inside it).
    pub fn occ_at(&self, file: usize, offset: usize) -> Option<&Occ> {
        self.occs
            .iter()
            .filter(|o| o.file == file && o.span.start <= offset && offset <= o.span.end)
            .min_by_key(|o| o.span.end - o.span.start)
    }

    pub fn member_at(&self, file: usize, offset: usize) -> Option<&MemberOcc> {
        self.members
            .iter()
            .find(|m| m.file == file && m.span.start <= offset && offset <= m.span.end)
    }

    pub fn module_of(&self, path: &Path) -> Option<String> {
        if let Some(m) = self.file(path).and_then(|f| self.files[f].module.clone()) {
            return Some(m);
        }
        module_for(&self.root, path)
    }
}

/// The module a path under `root/spec` belongs to.
pub fn module_for(root: &Path, path: &Path) -> Option<String> {
    let rel = path.parent()?.strip_prefix(root.join("spec")).ok()?;
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("."),
    )
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The defining name, a label and the body of a statement.
pub fn statement_parts(
    statement: &Statement,
) -> Option<(&sdlang_syntax::ast::Name, String, Option<&Block>)> {
    let Statement::Decl(decl) = statement else {
        return None;
    };
    Some(match &decl.item {
        Exportable::Let { name, .. } => (name, "let".into(), None),
        Exportable::Type { name, .. } => (name, "type".into(), None),
        Exportable::Kind { name, body } => (name, "kind".into(), Some(body)),
        Exportable::Annotation { name, body } => (name, "annotation".into(), Some(body)),
        Exportable::Implements { kind, body } => (kind, "implements".into(), Some(body)),
        Exportable::Named(n) => {
            let label = if n.is_extern {
                format!("extern {}", n.kind.dotted())
            } else {
                n.kind.dotted()
            };
            (&n.name, label, Some(&n.body))
        }
    })
}

pub fn summary_of(block: &Block) -> Option<String> {
    block.members.iter().find_map(|m| match &m.value {
        MemberValue::Field(f) if m.name.text == "summary" => match &f.value {
            Expr::String(s) => Some(s.to_source_text()),
            _ => None,
        },
        _ => None,
    })
}

/// Builds the analysis of the project at `root`, reading open buffers instead of disk.
pub fn build(root: &Path, buffers: &HashMap<PathBuf, Buffer>) -> Analysis {
    let mut overlay = Overlay::new();
    let mut stale = Vec::new();
    let mut diags: HashMap<PathBuf, Vec<Diag>> = HashMap::new();
    for (path, buffer) in buffers {
        let text = lexer::normalize(&buffer.text).into_owned();
        match parse(&text) {
            Ok(_) => {
                overlay.insert(path.clone(), text);
            }
            Err(e) => {
                diags.entry(path.clone()).or_default().push(Diag {
                    span: e.span,
                    message: e.message,
                });
                if let Some(good) = &buffer.last_good {
                    overlay.insert(path.clone(), lexer::normalize(good).into_owned());
                    stale.push(path.clone());
                } else {
                    overlay.insert(path.clone(), text);
                }
            }
        }
    }
    let mut out = match loader::load_with(root, &overlay) {
        Ok(project) => match Scope::build(&project) {
            Ok(scope) => {
                let mut a = semantic(root, &project, &scope);
                if let Err(e) = sdlang_graph::lower::lower_project(&project) {
                    push_error(&mut a.diags, root, &e);
                }
                a
            }
            Err(e) => {
                let mut a = syntactic(root, spec_files(root, &overlay));
                push_error(&mut a.diags, root, &e);
                a
            }
        },
        Err(e) => {
            let mut a = syntactic(root, spec_files(root, &overlay));
            push_error(&mut a.diags, root, &e);
            a
        }
    };
    for path in &stale {
        if let Some(&f) = out.by_path.get(path) {
            out.files[f].stale = true;
            // Errors located by the stale text would point at the wrong place.
            out.diags.remove(path);
        }
    }
    for (path, list) in diags {
        out.diags.insert(path, list);
    }
    out
}

/// A file outside any project: parsed on its own.
pub fn standalone(path: &Path, buffer: Option<&Buffer>) -> Analysis {
    let text = buffer
        .map(|b| b.text.clone())
        .or_else(|| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let root = path.parent().unwrap_or(Path::new("/")).to_path_buf();
    let mut a = syntactic(
        &root,
        vec![(
            path.to_path_buf(),
            String::new(),
            lexer::normalize(&text).into_owned(),
        )],
    );
    a.files[0].module = None;
    a
}

fn push_error(diags: &mut HashMap<PathBuf, Vec<Diag>>, root: &Path, e: &GraphError) {
    if let Some((path, _, span)) = &e.location {
        let path = canonical(&root.join(path));
        let list = diags.entry(path).or_default();
        if !list
            .iter()
            .any(|d| d.span == *span && d.message == e.message)
        {
            list.push(Diag {
                span: *span,
                message: e.message.clone(),
            });
        }
    }
}

/// Every `.sd` file under `root/spec`, with its module and text.
fn spec_files(root: &Path, overlay: &Overlay) -> Vec<(PathBuf, String, String)> {
    fn walk(dir: &Path, module: &str, overlay: &Overlay, out: &mut Vec<(PathBuf, String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for entry in entries {
            let name = entry
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if entry.is_dir() {
                let child = if module.is_empty() {
                    name
                } else {
                    format!("{module}.{name}")
                };
                walk(&entry, &child, overlay, out);
            } else if entry.extension().is_some_and(|e| e == "sd") {
                let path = canonical(&entry);
                let text = overlay
                    .get(&path)
                    .cloned()
                    .or_else(|| std::fs::read_to_string(&path).ok())
                    .unwrap_or_default();
                out.push((
                    path,
                    module.to_string(),
                    lexer::normalize(&text).into_owned(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("spec"), "", overlay, &mut out);
    out
}

fn empty(root: &Path, semantic: bool) -> Analysis {
    Analysis {
        root: root.to_path_buf(),
        files: Vec::new(),
        by_path: HashMap::new(),
        occs: Vec::new(),
        members: Vec::new(),
        defs: HashMap::new(),
        diags: HashMap::new(),
        bindings: BTreeMap::new(),
        children: HashMap::new(),
        kind_fields: HashMap::new(),
        semantic,
    }
}

fn add_file(
    a: &mut Analysis,
    path: PathBuf,
    text: String,
    ast: Option<File>,
    module: Option<String>,
    package: Option<String>,
) -> usize {
    let id = a.files.len();
    a.by_path.insert(path.clone(), id);
    a.files.push(FileData {
        index: LineIndex::new(&text),
        path,
        text,
        ast,
        module,
        package,
        stale: false,
    });
    id
}

fn core_kind_fields(name: &str) -> Option<Vec<String>> {
    let s = schema::core_kind(name)?;
    let mut names: Vec<String> = s.fields.keys().cloned().collect();
    names.extend(s.collections.iter().map(|c| c.section().to_string()));
    Some(names)
}

// ---------------------------------------------------------------------------------------
// With the compiler's scope.

fn target_sym(project: &Project, target: &Target) -> Sym {
    match target {
        Target::Module(m) => Sym::Module(m.clone()),
        Target::Package(p) => Sym::Package(project.packages[*p].id.clone()),
        Target::Capability(c) => Sym::Capability(c.clone()),
        Target::Decl(_) => unreachable!("handled by the caller"),
        Target::PackageItem { package, name } => Sym::PackageItem {
            package: project.packages[*package].id.clone(),
            name: name.clone(),
        },
        Target::Skill { package, name } => Sym::Skill {
            package: project.packages[*package].id.clone(),
            name: name.clone(),
        },
        Target::Builtin(b) => Sym::Builtin(b.clone()),
        Target::CoreKind(k) => Sym::CoreKind(k.clone()),
        Target::CoreAnnotation(a) => Sym::CoreAnnotation(a.clone()),
        Target::Now => Sym::Builtin("now".into()),
    }
}

fn okey(owner: &Owner) -> String {
    match owner {
        Owner::Module(m) => format!("m:{m}"),
        Owner::Package(p) => format!("p:{p}"),
    }
}

fn semantic(root: &Path, project: &Project, scope: &Scope) -> Analysis {
    let mut a = empty(root, true);
    // Files, and where each project file sits in the load.
    let mut owners: Vec<(Owner, usize)> = Vec::new();
    let mut file_of: HashMap<(String, usize), usize> = HashMap::new();
    for (m, module) in &project.modules {
        for (i, f) in module.files.iter().enumerate() {
            let id = add_file(
                &mut a,
                canonical(&root.join(&f.path)),
                f.text.clone(),
                Some(f.ast.clone()),
                Some(m.clone()),
                None,
            );
            owners.push((Owner::Module(m.clone()), i));
            file_of.insert((okey(&Owner::Module(m.clone())), i), id);
        }
    }
    for (p, package) in project.packages.iter().enumerate() {
        for (i, f) in package.files.iter().enumerate() {
            let id = add_file(
                &mut a,
                canonical(&root.join(&f.path)),
                f.text.clone(),
                Some(f.ast.clone()),
                None,
                Some(package.id.clone()),
            );
            owners.push((Owner::Package(p), i));
            file_of.insert((okey(&Owner::Package(p)), i), id);
        }
    }
    let source = |file: usize| -> &loader::SourceFile {
        match &owners[file] {
            (Owner::Module(m), i) => &project.modules[m].files[*i],
            (Owner::Package(p), i) => &project.packages[*p].files[*i],
        }
    };
    let to_sym = |t: &Target| -> Sym {
        match t {
            Target::Decl(i) => Sym::Decl(scope.decls[*i].address.clone()),
            other => target_sym(project, other),
        }
    };
    let resolve = |file: usize, names: &[&str]| -> Option<Sym> {
        let owner = &owners[file].0;
        scope
            .resolve_names(owner, names, source(file), Span::default())
            .ok()
            .map(|t| to_sym(&t))
    };

    // Definitions.
    let mut stmt_sym: HashMap<(usize, usize), Sym> = HashMap::new();
    for d in &scope.decls {
        let Some(&file) = file_of.get(&(okey(&Owner::Module(d.module.clone())), d.file)) else {
            continue;
        };
        let statement = &a.files[file].ast.as_ref().unwrap().statements[d.statement];
        let Some((name, label, body)) = statement_parts(statement) else {
            continue;
        };
        let kind = match statement {
            Statement::Decl(decl) => match &decl.item {
                Exportable::Named(n) => {
                    let names: Vec<&str> =
                        n.kind.segments.iter().map(|s| s.text.as_str()).collect();
                    resolve(file, &names)
                }
                _ => None,
            },
            _ => None,
        };
        let sym = Sym::Decl(d.address.clone());
        stmt_sym.insert((file, d.statement), sym.clone());
        a.defs.insert(
            sym,
            Def {
                file: Some(file),
                path: a.files[file].path.clone(),
                name_span: name.span,
                full_span: statement.span(),
                label,
                summary: body.and_then(summary_of),
                kind,
                exported: d.exported,
            },
        );
    }
    for (p, items) in scope.package_items.iter().enumerate() {
        let package = &project.packages[p];
        let Some(&file) = file_of.get(&(okey(&Owner::Package(p)), 0)) else {
            continue;
        };
        for (name, item) in items {
            let statement = &a.files[file].ast.as_ref().unwrap().statements[item.statement];
            let Some((name_node, label, body)) = statement_parts(statement) else {
                continue;
            };
            let sym = Sym::PackageItem {
                package: package.id.clone(),
                name: name.clone(),
            };
            if let (Statement::Decl(decl), "kind") = (statement, label.as_str()) {
                if let Exportable::Kind { body, .. } = &decl.item {
                    let s = schema::package_kind(format!("{}::{name}", package.id), body);
                    let mut names: Vec<String> = s.fields.keys().cloned().collect();
                    names.extend(s.collections.iter().map(|c| c.section().to_string()));
                    a.kind_fields.insert(sym.clone(), names);
                }
            }
            stmt_sym.insert((file, item.statement), sym.clone());
            a.defs.insert(
                sym,
                Def {
                    file: Some(file),
                    path: a.files[file].path.clone(),
                    name_span: name_node.span,
                    full_span: statement.span(),
                    label,
                    summary: body.and_then(summary_of),
                    kind: None,
                    exported: item.exported,
                },
            );
        }
        let entry = a.files[file].path.clone();
        a.defs.insert(
            Sym::Package(package.id.clone()),
            Def {
                file: Some(file),
                path: entry,
                name_span: Span::default(),
                full_span: Span::default(),
                label: "package".into(),
                summary: Some(format!(
                    "npm `{}`, imported as `{}`",
                    package.npm_name, package.import_name
                )),
                kind: None,
                exported: true,
            },
        );
        for skill in &package.skills {
            let path = package.root.join("skills").join(format!("{skill}.md"));
            a.defs.insert(
                Sym::Skill {
                    package: package.id.clone(),
                    name: skill.clone(),
                },
                Def {
                    file: None,
                    path: canonical(&path),
                    name_span: Span::default(),
                    full_span: Span::default(),
                    label: "skill".into(),
                    summary: None,
                    kind: None,
                    exported: true,
                },
            );
        }
        let mut kids: Vec<(String, Sym)> = items
            .iter()
            .filter(|(_, i)| i.exported)
            .map(|(n, _)| {
                (
                    n.clone(),
                    Sym::PackageItem {
                        package: package.id.clone(),
                        name: n.clone(),
                    },
                )
            })
            .collect();
        kids.extend(package.skills.iter().map(|s| {
            (
                s.clone(),
                Sym::Skill {
                    package: package.id.clone(),
                    name: s.clone(),
                },
            )
        }));
        a.children.insert(Sym::Package(package.id.clone()), kids);
    }
    for (m, module) in &project.modules {
        let header = module.files.iter().enumerate().find_map(|(i, f)| {
            f.ast.statements.iter().find_map(|s| match s {
                Statement::Module(h) => Some((i, h)),
                _ => None,
            })
        });
        let first = header.map(|(i, _)| i).unwrap_or(0);
        let Some(&file) = file_of.get(&(okey(&Owner::Module(m.clone())), first)) else {
            continue;
        };
        let (name_span, full_span) = header.map_or((Span::default(), Span::default()), |(_, h)| {
            (Span::new(h.span.start, h.span.start + 6), h.span)
        });
        a.defs.insert(
            Sym::Module(m.clone()),
            Def {
                file: Some(file),
                path: a.files[file].path.clone(),
                name_span,
                full_span,
                label: "module".into(),
                summary: header.and_then(|(_, h)| summary_of(&h.body)),
                kind: None,
                exported: true,
            },
        );
        let mut kids: Vec<(String, Sym)> = project
            .modules
            .keys()
            .filter_map(|c| {
                let rest = if m.is_empty() {
                    Some(c.as_str())
                } else {
                    c.strip_prefix(&format!("{m}."))
                }?;
                (!rest.is_empty() && !rest.contains('.'))
                    .then(|| (rest.to_string(), Sym::Module(c.clone())))
            })
            .collect();
        kids.extend(
            scope
                .decls
                .iter()
                .filter(|d| &d.module == m)
                .map(|d| (d.name.clone(), Sym::Decl(d.address.clone()))),
        );
        a.children.insert(Sym::Module(m.clone()), kids);
    }
    for (c, cap) in &scope.capabilities {
        let mut def = None;
        for (fid, f) in a.files.iter().enumerate() {
            for s in &f.ast.as_ref().unwrap().statements {
                if let Statement::Use(u) = s {
                    if &u.capability.text == c {
                        def = Some((fid, u.capability.span, u.span));
                    }
                }
            }
        }
        if let Some((file, name_span, full_span)) = def {
            a.defs.insert(
                Sym::Capability(c.clone()),
                Def {
                    file: Some(file),
                    path: a.files[file].path.clone(),
                    name_span,
                    full_span,
                    label: "capability".into(),
                    summary: Some(format!("bound to `{}`", cap.library)),
                    kind: None,
                    exported: true,
                },
            );
        }
        if let Some(p) = cap.provider {
            let package = &project.packages[p];
            let kids = package
                .provides
                .get(c)
                .into_iter()
                .flatten()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        Sym::PackageItem {
                            package: package.id.clone(),
                            name: v.clone(),
                        },
                    )
                })
                .collect();
            a.children.insert(Sym::Capability(c.clone()), kids);
        }
    }
    for m in project.modules.keys() {
        let list = scope
            .module_bindings(m)
            .iter()
            .map(|(n, t)| (n.clone(), to_sym(t)))
            .collect();
        a.bindings.insert(m.clone(), list);
    }
    for k in scope::CORE_KINDS
        .iter()
        .chain(["project", "stack", "module"].iter())
    {
        if let Some(names) = core_kind_fields(k) {
            a.kind_fields.insert(Sym::CoreKind(k.to_string()), names);
        }
    }

    // References.
    let canonical_addr = |addr: &str| -> Option<Sym> {
        let sym = Sym::Decl(addr.to_string());
        a.defs.contains_key(&sym).then_some(sym)
    };
    let mut occs = Vec::new();
    let mut members = Vec::new();
    for file in 0..a.files.len() {
        let mut w = Walker {
            file,
            text: &a.files[file].text,
            resolve: &|names: &[&str]| resolve(file, names),
            canonical: &canonical_addr,
            stmt_sym: &stmt_sym,
            occs: &mut occs,
            members: &mut members,
        };
        w.file(a.files[file].ast.as_ref().unwrap());
    }
    a.occs = occs;
    a.members = members;
    a
}

// ---------------------------------------------------------------------------------------
// Without it: each file on its own, names by a simple rule.

fn syntactic(root: &Path, files: Vec<(PathBuf, String, String)>) -> Analysis {
    let mut a = empty(root, false);
    for (path, module, text) in files {
        let ast = match parse(&text) {
            Ok(ast) => Some(ast),
            Err(e) => {
                a.diags.entry(path.clone()).or_default().push(Diag {
                    span: e.span,
                    message: e.message,
                });
                None
            }
        };
        add_file(&mut a, path, text, ast, Some(module), None);
    }
    let mut stmt_sym = HashMap::new();
    let mut by_name: HashMap<String, Vec<Sym>> = HashMap::new();
    for (file, f) in a.files.iter().enumerate() {
        let Some(ast) = &f.ast else { continue };
        let module = f.module.clone().unwrap_or_default();
        for (si, statement) in ast.statements.iter().enumerate() {
            if let Statement::Import(import) = statement {
                if let ImportSource::Module(p) = &import.source {
                    let alias = import.alias.as_ref().map_or_else(
                        || {
                            p.segments
                                .last()
                                .map(|s| s.text.clone())
                                .unwrap_or_default()
                        },
                        |a| a.text.clone(),
                    );
                    a.bindings
                        .entry(module.clone())
                        .or_default()
                        .push((alias, Sym::Module(p.dotted())));
                }
            }
            let Some((name, label, body)) = statement_parts(statement) else {
                continue;
            };
            if matches!(label.as_str(), "kind" | "annotation" | "implements") {
                continue;
            }
            let address = if module.is_empty() {
                name.text.clone()
            } else {
                format!("{module}.{}", name.text)
            };
            let sym = Sym::Decl(address);
            let exported = matches!(statement, Statement::Decl(d) if d.export);
            let kind = match statement {
                Statement::Decl(decl) => match &decl.item {
                    Exportable::Named(n)
                        if n.kind.segments.len() == 1
                            && scope::CORE_KINDS.contains(&n.kind.dotted().as_str()) =>
                    {
                        Some(Sym::CoreKind(n.kind.dotted()))
                    }
                    _ => None,
                },
                _ => None,
            };
            stmt_sym.insert((file, si), sym.clone());
            by_name
                .entry(name.text.clone())
                .or_default()
                .push(sym.clone());
            a.bindings
                .entry(module.clone())
                .or_default()
                .push((name.text.clone(), sym.clone()));
            a.children
                .entry(Sym::Module(module.clone()))
                .or_default()
                .push((name.text.clone(), sym.clone()));
            a.defs.insert(
                sym,
                Def {
                    file: Some(file),
                    path: f.path.clone(),
                    name_span: name.span,
                    full_span: statement.span(),
                    label,
                    summary: body.and_then(summary_of),
                    kind,
                    exported,
                },
            );
        }
    }
    for k in scope::CORE_KINDS
        .iter()
        .chain(["project", "stack", "module"].iter())
    {
        if let Some(names) = core_kind_fields(k) {
            a.kind_fields.insert(Sym::CoreKind(k.to_string()), names);
        }
    }
    let modules: Vec<String> = a.files.iter().filter_map(|f| f.module.clone()).collect();
    let resolve = |file: usize, names: &[&str]| -> Option<Sym> {
        let module = a.files[file].module.clone().unwrap_or_default();
        if let [one] = names {
            if let Some(sym) = prelude(one) {
                return Some(sym);
            }
            let local = Sym::Decl(if module.is_empty() {
                one.to_string()
            } else {
                format!("{module}.{one}")
            });
            if a.defs.contains_key(&local) {
                return Some(local);
            }
            if modules.iter().any(|m| m == one) {
                return Some(Sym::Module(one.to_string()));
            }
            return match by_name.get(*one).map(Vec::as_slice) {
                Some([only]) => Some(only.clone()),
                _ => None,
            };
        }
        let dotted = names.join(".");
        if modules.contains(&dotted) {
            return Some(Sym::Module(dotted));
        }
        let sym = Sym::Decl(dotted);
        a.defs.contains_key(&sym).then_some(sym)
    };
    let canonical_addr = |addr: &str| -> Option<Sym> {
        let sym = Sym::Decl(addr.to_string());
        a.defs.contains_key(&sym).then_some(sym)
    };
    let mut occs = Vec::new();
    let mut members = Vec::new();
    for file in 0..a.files.len() {
        let Some(ast) = &a.files[file].ast else {
            continue;
        };
        let mut w = Walker {
            file,
            text: &a.files[file].text,
            resolve: &|names: &[&str]| resolve(file, names),
            canonical: &canonical_addr,
            stmt_sym: &stmt_sym,
            occs: &mut occs,
            members: &mut members,
        };
        w.file(ast);
    }
    a.occs = occs;
    a.members = members;
    a
}

fn prelude(name: &str) -> Option<Sym> {
    if scope::CORE_KINDS.contains(&name) {
        Some(Sym::CoreKind(name.into()))
    } else if scope::BUILTIN_TYPES.contains(&name) || name == "now" {
        Some(Sym::Builtin(name.into()))
    } else if scope::CORE_ANNOTATIONS.contains(&name) {
        Some(Sym::CoreAnnotation(name.into()))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------------------
// The walk that finds every name in a file.

struct Walker<'a> {
    file: usize,
    text: &'a str,
    resolve: &'a dyn Fn(&[&str]) -> Option<Sym>,
    canonical: &'a dyn Fn(&str) -> Option<Sym>,
    stmt_sym: &'a HashMap<(usize, usize), Sym>,
    occs: &'a mut Vec<Occ>,
    members: &'a mut Vec<MemberOcc>,
}

impl Walker<'_> {
    fn push(&mut self, span: Span, sym: Sym, def: bool) {
        self.occs.push(Occ {
            file: self.file,
            span,
            sym,
            def,
            canonical: false,
        });
    }

    fn file(&mut self, ast: &File) {
        for (si, statement) in ast.statements.iter().enumerate() {
            self.statement(si, statement);
        }
    }

    fn statement(&mut self, si: usize, statement: &Statement) {
        match statement {
            Statement::Import(import) => {
                let alias = import.alias.as_ref().map(|a| a.text.clone());
                let (span, name) = match &import.source {
                    ImportSource::Path(p) | ImportSource::Module(p) => (
                        p.span,
                        alias.clone().unwrap_or_else(|| {
                            p.segments
                                .last()
                                .map(|s| s.text.clone())
                                .unwrap_or_default()
                        }),
                    ),
                    ImportSource::Package(lit) => (lit.span, alias.clone().unwrap_or_default()),
                };
                if let Some(sym) = (self.resolve)(&[name.as_str()]) {
                    self.push(span, sym.clone(), false);
                    if let Some(a) = &import.alias {
                        self.push(a.span, sym, true);
                    }
                }
            }
            Statement::Use(u) => {
                self.push(
                    u.capability.span,
                    Sym::Capability(u.capability.text.clone()),
                    true,
                );
                if let Some(r) = &u.options {
                    self.record(r);
                }
            }
            Statement::Moved(m) => {
                for addr in [&m.from, &m.to] {
                    let dotted: Vec<&str> = addr.segments.iter().map(|s| s.text.as_str()).collect();
                    let sym = if m.module {
                        Some(Sym::Module(dotted.join(".")))
                    } else {
                        (self.canonical)(&dotted.join("."))
                    };
                    if let (Some(sym), Some(last)) = (sym, addr.segments.last()) {
                        self.occs.push(Occ {
                            file: self.file,
                            span: last.span,
                            sym,
                            def: false,
                            canonical: true,
                        });
                    }
                }
            }
            Statement::Project(p) => self.block(
                &p.body,
                None,
                Some(Sym::CoreKind("project".into())),
                &mut Vec::new(),
            ),
            Statement::Stack(s) => self.block(
                &s.body,
                None,
                Some(Sym::CoreKind("stack".into())),
                &mut Vec::new(),
            ),
            Statement::Module(m) => self.block(
                &m.body,
                None,
                Some(Sym::CoreKind("module".into())),
                &mut Vec::new(),
            ),
            Statement::Decl(decl) => {
                let own = self.stmt_sym.get(&(self.file, si)).cloned();
                match &decl.item {
                    Exportable::Let { name, value } | Exportable::Type { name, value } => {
                        if let Some(sym) = own {
                            self.push(name.span, sym, true);
                        }
                        self.expr(value);
                    }
                    Exportable::Kind { name, body } | Exportable::Annotation { name, body } => {
                        if let Some(sym) = own {
                            self.push(name.span, sym.clone(), true);
                        }
                        self.block(body, None, None, &mut Vec::new());
                    }
                    Exportable::Implements { kind, body } => {
                        if let Some(sym) = (self.resolve)(&[kind.text.as_str()]) {
                            self.push(kind.span, sym, false);
                        }
                        self.block(body, None, None, &mut Vec::new());
                    }
                    Exportable::Named(n) => {
                        let kind = self.path(&n.kind);
                        if let Some(sym) = &own {
                            self.push(n.name.span, sym.clone(), true);
                        }
                        for a in &n.annotations {
                            self.path(&a.name);
                            for e in a.args.iter().flatten() {
                                self.expr(e);
                            }
                        }
                        self.block(&n.body, own, kind, &mut Vec::new());
                    }
                }
            }
        }
    }

    fn block(
        &mut self,
        block: &Block,
        decl: Option<Sym>,
        kind: Option<Sym>,
        path: &mut Vec<String>,
    ) {
        for member in &block.members {
            path.push(member.name.text.clone());
            self.members.push(MemberOcc {
                file: self.file,
                span: member.name.span,
                decl: decl.clone(),
                kind: kind.clone(),
                path: path.clone(),
            });
            match &member.value {
                MemberValue::Field(f) => {
                    self.expr(&f.value);
                    if let Some(d) = &f.default {
                        self.expr(d);
                    }
                    for a in &f.annotations {
                        self.path(&a.name);
                        for e in a.args.iter().flatten() {
                            self.expr(e);
                        }
                    }
                }
                MemberValue::Section(b) => self.block(b, decl.clone(), kind.clone(), path),
            }
            path.pop();
        }
    }

    fn record(&mut self, record: &Record) {
        for f in &record.fields {
            self.expr(&f.field.value);
            if let Some(d) = &f.field.default {
                self.expr(d);
            }
            for a in &f.field.annotations {
                self.path(&a.name);
                for e in a.args.iter().flatten() {
                    self.expr(e);
                }
            }
        }
    }

    /// Records each segment of `path` that resolves; returns what the whole path denotes.
    fn path(&mut self, path: &SynPath) -> Option<Sym> {
        let names: Vec<&str> = path.segments.iter().map(|s| s.text.as_str()).collect();
        let mut last = None;
        for i in 0..names.len() {
            let sym = (self.resolve)(&names[..=i])?;
            self.push(path.segments[i].span, sym.clone(), false);
            last = Some(sym);
        }
        last
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Path { path, record, .. } => {
                self.path(path);
                if let Some(r) = record {
                    self.record(r);
                }
            }
            Expr::Union { variants, .. } => variants.iter().for_each(|v| self.expr(v)),
            Expr::List { items, .. } => items.iter().for_each(|v| self.expr(v)),
            Expr::Optional { inner, .. } => self.expr(inner),
            Expr::Ref { target, .. } => self.expr(target),
            Expr::Record(r) => self.record(r),
            Expr::String(s) => self.mentions(s),
            Expr::Selector(sel) => {
                match &sel.source {
                    SelectSource::All => {}
                    SelectSource::Kind(p) => {
                        self.path(p);
                    }
                    SelectSource::Kinds(ps) => {
                        for p in ps {
                            self.path(p);
                        }
                    }
                }
                if let Some(p) = &sel.predicate {
                    self.predicate(p);
                }
            }
            Expr::Number { .. } | Expr::Bool { .. } | Expr::Null { .. } | Expr::SelfRef { .. } => {}
        }
    }

    fn predicate(&mut self, p: &Predicate) {
        match p {
            Predicate::Or { terms } | Predicate::And { terms } => {
                terms.iter().for_each(|t| self.predicate(t))
            }
            Predicate::Not { inner } => self.predicate(inner),
            Predicate::Exists { .. } => {}
            Predicate::Compare { operand, .. } => {
                if let sdlang_syntax::ast::Operand::Path(path) = operand {
                    self.path(path);
                }
            }
        }
    }

    /// `@{path}` mentions inside a string, found in its source text.
    fn mentions(&mut self, lit: &StringLit) {
        let Some(raw) = self.text.get(lit.span.start..lit.span.end) else {
            return;
        };
        let bytes = raw.as_bytes();
        let mut i = 0;
        while let Some(at) = raw[i..].find("@{").map(|k| k + i) {
            i = at + 2;
            if at > 0 && bytes[at - 1] == b'@' {
                continue; // `@@{` is a literal
            }
            let Some(close) = raw[i..].find('}').map(|k| k + i) else {
                break;
            };
            let inner = &raw[i..close];
            if inner == "self"
                || inner.is_empty()
                || !inner
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            {
                continue;
            }
            let names: Vec<&str> = inner.split('.').collect();
            let mut start = lit.span.start + i;
            for k in 0..names.len() {
                let Some(sym) = (self.resolve)(&names[..=k]) else {
                    break;
                };
                let span = Span::new(start, start + names[k].len());
                self.push(span, sym, false);
                start = span.end + 1;
            }
            i = close + 1;
        }
    }
}
