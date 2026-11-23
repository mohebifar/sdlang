//! Per-module binding tables and path resolution (docs/spec.md, "Scoping and resolution").
//!
//! A module's bindings are its declarations, types and lets, its imports, the project's
//! capabilities and the core prelude. Two bindings with one name are an error; nothing
//! shadows anything. A path walks modules, packages, capabilities and their exported
//! names; walking into a declaration is an error.

use std::collections::BTreeMap;
use std::path::Path as FsPath;

use sdlang_syntax::ast::{Exportable, ImportSource, Path, Span, Statement};
use sdlang_syntax::strings::{Part, StringLit};

use crate::error::{GResult, GraphError};
use crate::loader::{Project, SourceFile};

pub const CORE_KINDS: &[&str] = &[
    "entity",
    "action",
    "lib",
    "rule",
    "environment",
    "check",
    "generator",
    "observer",
    "binding",
];
pub const BUILTIN_TYPES: &[&str] = &[
    "string",
    "number",
    "integer",
    "boolean",
    "uuid",
    "email",
    "url",
    "timestamp",
    "date",
    "json",
    "route_path",
    "template",
    "secret",
    "catalog",
];
pub const CORE_ANNOTATIONS: &[&str] = &[
    "id",
    "unique",
    "max_length",
    "interface",
    "relation",
    "deprecated",
];

/// What a name or path denotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Module(String),
    Package(usize),
    /// A capability bound by `use`, by name.
    Capability(String),
    /// A project declaration, type or let: index into [`Scope::decls`].
    Decl(usize),
    /// A top-level name in a package's sources (a kind, type, extern, binding, ...).
    PackageItem {
        package: usize,
        name: String,
    },
    /// A skill named in a package manifest.
    Skill {
        package: usize,
        name: String,
    },
    Builtin(String),
    CoreKind(String),
    CoreAnnotation(String),
    Now,
}

/// The form of a project declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclForm {
    Named,
    Type,
    Let,
}

#[derive(Clone, Debug)]
pub struct DeclInfo {
    pub address: String,
    pub module: String,
    pub name: String,
    pub exported: bool,
    pub form: DeclForm,
    /// Index of the file in its module, and of the statement in the file.
    pub file: usize,
    pub statement: usize,
}

/// Where a path is being resolved from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    Module(String),
    Package(usize),
}

#[derive(Clone, Debug)]
pub struct PackageItem {
    pub exported: bool,
    /// Which statement in the package's (single) file defines it.
    pub statement: usize,
}

#[derive(Clone, Debug)]
pub struct Capability {
    /// The library name as `use` writes it.
    pub library: String,
    /// The provider's spec package, if the lock has one.
    pub provider: Option<usize>,
}

pub struct Scope<'p> {
    pub project: &'p Project,
    pub decls: Vec<DeclInfo>,
    pub capabilities: BTreeMap<String, Capability>,
    pub package_items: Vec<BTreeMap<String, PackageItem>>,
    module_decls: BTreeMap<String, BTreeMap<String, usize>>,
    bindings: BTreeMap<String, BTreeMap<String, Target>>,
    /// Each module's imports in authored order, coalesced: (alias, target).
    pub imports: BTreeMap<String, Vec<(String, Target)>>,
}

pub fn lit_text(lit: &StringLit) -> String {
    lit.parts
        .iter()
        .map(|p| match p {
            Part::Text(t) => t.as_str(),
            _ => "",
        })
        .collect()
}

fn prelude(name: &str) -> Option<Target> {
    if CORE_KINDS.contains(&name) {
        Some(Target::CoreKind(name.into()))
    } else if BUILTIN_TYPES.contains(&name) {
        Some(Target::Builtin(name.into()))
    } else if name == "now" {
        Some(Target::Now)
    } else if CORE_ANNOTATIONS.contains(&name) {
        Some(Target::CoreAnnotation(name.into()))
    } else {
        None
    }
}

fn top_level_name(statement: &Statement) -> Option<(&str, bool, Span)> {
    let Statement::Decl(decl) = statement else {
        return None;
    };
    let name = match &decl.item {
        Exportable::Let { name, .. }
        | Exportable::Type { name, .. }
        | Exportable::Kind { name, .. }
        | Exportable::Annotation { name, .. } => name,
        Exportable::Named(n) => &n.name,
        Exportable::Implements { .. } => return None,
    };
    Some((name.text.as_str(), decl.export, name.span))
}

impl<'p> Scope<'p> {
    pub fn build(project: &'p Project) -> GResult<Scope<'p>> {
        let mut decls = Vec::new();
        let mut module_decls: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        for (path, module) in &project.modules {
            let table = module_decls.entry(path.clone()).or_default();
            for (fi, file) in module.files.iter().enumerate() {
                for (si, statement) in file.ast.statements.iter().enumerate() {
                    let Statement::Decl(decl) = statement else {
                        continue;
                    };
                    let (name, form) = match &decl.item {
                        Exportable::Named(n) => (&n.name, DeclForm::Named),
                        Exportable::Type { name, .. } => (name, DeclForm::Type),
                        Exportable::Let { name, .. } => (name, DeclForm::Let),
                        _ => return Err(GraphError::at(
                            file,
                            decl.span,
                            "kind, implements and annotation definitions belong in package sources",
                        )),
                    };
                    if table.contains_key(&name.text) {
                        return Err(GraphError::at(
                            file,
                            name.span,
                            format!("'{}' is declared twice in module '{path}'", name.text),
                        ));
                    }
                    let address = if path.is_empty() {
                        name.text.clone()
                    } else {
                        format!("{path}.{}", name.text)
                    };
                    table.insert(name.text.clone(), decls.len());
                    decls.push(DeclInfo {
                        address,
                        module: path.clone(),
                        name: name.text.clone(),
                        exported: decl.export,
                        form,
                        file: fi,
                        statement: si,
                    });
                }
            }
        }

        let mut package_items = Vec::new();
        for package in &project.packages {
            let mut items = BTreeMap::new();
            for (si, statement) in package.files[0].ast.statements.iter().enumerate() {
                if let Some((name, exported, _)) = top_level_name(statement) {
                    items.insert(
                        name.to_string(),
                        PackageItem {
                            exported,
                            statement: si,
                        },
                    );
                }
            }
            package_items.push(items);
        }

        let mut capabilities = BTreeMap::new();
        if let Some(root) = project.modules.get("") {
            for file in &root.files {
                for statement in &file.ast.statements {
                    if let Statement::Use(u) = statement {
                        let library = lit_text(&u.library);
                        let provider = project
                            .packages
                            .iter()
                            .position(|p| p.source_name == library);
                        if capabilities.contains_key(&u.capability.text) {
                            return Err(GraphError::at(
                                file,
                                u.capability.span,
                                format!("capability '{}' is bound twice", u.capability.text),
                            ));
                        }
                        capabilities
                            .insert(u.capability.text.clone(), Capability { library, provider });
                    }
                }
            }
        }
        for module in project.modules.values() {
            for file in &module.files {
                for statement in &file.ast.statements {
                    if let Statement::Use(u) = statement {
                        if !module.path.is_empty() {
                            return Err(GraphError::at(
                                file,
                                u.span,
                                "use belongs in the root module",
                            ));
                        }
                    }
                }
            }
        }

        let mut scope = Scope {
            project,
            decls,
            capabilities,
            package_items,
            module_decls,
            bindings: BTreeMap::new(),
            imports: BTreeMap::new(),
        };
        for path in project.modules.keys() {
            scope.build_bindings(path)?;
        }
        Ok(scope)
    }

    fn build_bindings(&mut self, path: &str) -> GResult<()> {
        let module = &self.project.modules[path];
        let mut table: BTreeMap<String, Target> = BTreeMap::new();
        for (name, &i) in &self.module_decls[path] {
            table.insert(name.clone(), Target::Decl(i));
        }
        let mut imports: Vec<(String, Target)> = Vec::new();
        for file in &module.files {
            for statement in &file.ast.statements {
                let Statement::Import(import) = statement else {
                    continue;
                };
                let (target, default_alias) =
                    self.import_target(file, &import.source, import.span)?;
                let alias = import
                    .alias
                    .as_ref()
                    .map(|a| a.text.clone())
                    .unwrap_or(default_alias);
                if let Some((_, existing)) = imports.iter().find(|(a, _)| *a == alias) {
                    if *existing == target {
                        continue; // identical repeated imports coalesce
                    }
                }
                if table.contains_key(&alias) || imports.iter().any(|(a, _)| *a == alias) {
                    return Err(GraphError::at(
                        file,
                        import.span,
                        format!("'{alias}' is already bound in module '{path}'"),
                    ));
                }
                imports.push((alias, target));
            }
        }
        for (alias, target) in &imports {
            table.insert(alias.clone(), target.clone());
        }
        for name in self.capabilities.keys() {
            if table.contains_key(name) {
                let (file, span) = self.binding_site(path, name);
                return Err(GraphError::at(
                    file,
                    span,
                    format!("'{name}' is already bound as a capability; use an alias"),
                ));
            }
            table.insert(name.clone(), Target::Capability(name.clone()));
        }
        for name in table.keys() {
            if prelude(name).is_some() {
                let (file, span) = self.binding_site(path, name);
                return Err(GraphError::at(
                    file,
                    span,
                    format!("'{name}' is a core name and cannot be bound again"),
                ));
            }
        }
        self.imports.insert(path.to_string(), imports);
        self.bindings.insert(path.to_string(), table);
        Ok(())
    }

    /// The file and span where `name` is bound in module `path`, for diagnostics.
    fn binding_site(&self, path: &str, name: &str) -> (&'p SourceFile, Span) {
        let module = &self.project.modules[path];
        for file in &module.files {
            for statement in &file.ast.statements {
                if let Some((n, _, span)) = top_level_name(statement) {
                    if n == name {
                        return (file, span);
                    }
                }
                if let Statement::Use(u) = statement {
                    if u.capability.text == name {
                        return (file, u.capability.span);
                    }
                }
                if let Statement::Import(i) = statement {
                    let alias = i.alias.as_ref().map(|a| a.text.as_str());
                    let last = match &i.source {
                        ImportSource::Path(p) | ImportSource::Module(p) => {
                            p.segments.last().map(|s| s.text.as_str())
                        }
                        ImportSource::Package(_) => None,
                    };
                    if alias.or(last) == Some(name) {
                        return (file, i.span);
                    }
                }
            }
        }
        if !path.is_empty() {
            let (file, span) = self.binding_site("", name);
            if span != Span::new(0, 0) {
                return (file, span);
            }
        }
        let file = module
            .files
            .first()
            .or_else(|| self.project.modules[""].files.first())
            .expect("a project has at least one file");
        (file, Span::new(0, 0))
    }

    fn import_target(
        &self,
        file: &SourceFile,
        source: &ImportSource,
        span: Span,
    ) -> GResult<(Target, String)> {
        let project = self.project;
        match source {
            ImportSource::Module(path) => {
                let dotted = path.dotted();
                if !project.modules.contains_key(&dotted) {
                    return Err(GraphError::at(
                        file,
                        path.span,
                        format!("no module '{dotted}'"),
                    ));
                }
                Ok((
                    Target::Module(dotted),
                    path.segments.last().expect("non-empty").text.clone(),
                ))
            }
            ImportSource::Path(path) => {
                let dotted = path.dotted();
                let module = project.modules.contains_key(&dotted);
                let packages: Vec<usize> = project
                    .packages
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.import_name == dotted)
                    .map(|(i, _)| i)
                    .collect();
                let alias = path.segments.last().expect("non-empty").text.clone();
                match (module, packages.as_slice()) {
                    (true, []) => Ok((Target::Module(dotted), alias)),
                    (false, [p]) => Ok((Target::Package(*p), alias)),
                    (false, []) => Err(GraphError::at(
                        file,
                        path.span,
                        format!("no local module or locked package is called '{dotted}'"),
                    )),
                    _ => Err(GraphError::at(
                        file,
                        path.span,
                        format!("'{dotted}' names more than one module or package; write 'import module' or 'import package'"),
                    )),
                }
            }
            ImportSource::Package(lit) => {
                let name = lit_text(lit);
                let found = if name.starts_with("./") || name.starts_with("../") {
                    let id = format!("local:{}", clean_display(&name));
                    project.package_by_id(&id)
                } else {
                    project.packages.iter().position(|p| p.source_name == name)
                };
                match found {
                    Some(p) => Ok((Target::Package(p), project.packages[p].import_name.clone())),
                    None => Err(GraphError::at(
                        file,
                        span,
                        format!("package '{name}' is not in sd.lock"),
                    )),
                }
            }
        }
    }

    pub fn module_bindings(&self, path: &str) -> &BTreeMap<String, Target> {
        &self.bindings[path]
    }

    /// Resolves the first segment of a path.
    fn lookup(&self, owner: &Owner, name: &str) -> Option<Target> {
        match owner {
            Owner::Module(m) => self.bindings[m]
                .get(name)
                .cloned()
                .or_else(|| prelude(name)),
            Owner::Package(p) => {
                if self.package_items[*p].contains_key(name) {
                    Some(Target::PackageItem {
                        package: *p,
                        name: name.into(),
                    })
                } else {
                    prelude(name)
                }
            }
        }
    }

    /// Resolves a dotted path from `owner`.
    pub fn resolve(&self, owner: &Owner, path: &Path, file: &SourceFile) -> GResult<Target> {
        let names: Vec<&str> = path.segments.iter().map(|s| s.text.as_str()).collect();
        self.resolve_names(owner, &names, file, path.span)
    }

    /// Resolves a dotted name (as in a mention) from `owner`; `span` is for diagnostics.
    pub fn resolve_names(
        &self,
        owner: &Owner,
        names: &[&str],
        file: &SourceFile,
        span: Span,
    ) -> GResult<Target> {
        let first = names[0];
        let mut target = self
            .lookup(owner, first)
            .ok_or_else(|| GraphError::at(file, span, format!("'{first}' is not bound here")))?;
        let current_module = match owner {
            Owner::Module(m) => Some(m.as_str()),
            Owner::Package(_) => None,
        };
        for seg in &names[1..] {
            target = self
                .walk(&target, seg, current_module)
                .map_err(|m| GraphError::at(file, span, m))?;
        }
        Ok(target)
    }

    fn walk(&self, target: &Target, seg: &str, current: Option<&str>) -> Result<Target, String> {
        let project = self.project;
        match target {
            Target::Module(m) => {
                let child = if m.is_empty() {
                    seg.to_string()
                } else {
                    format!("{m}.{seg}")
                };
                if project.modules.contains_key(&child) {
                    return Ok(Target::Module(child));
                }
                match self.module_decls[m].get(seg) {
                    Some(&i) if self.decls[i].exported || current == Some(m.as_str()) => {
                        Ok(Target::Decl(i))
                    }
                    Some(_) => Err(format!("'{seg}' is not exported from module '{m}'")),
                    None => Err(format!("module '{m}' has no '{seg}'")),
                }
            }
            Target::Package(p) => self.package_name(*p, seg),
            Target::Capability(c) => {
                let cap = &self.capabilities[c];
                let Some(p) = cap.provider else {
                    return Err(format!(
                        "capability '{c}' is bound to '{}', which has no spec package, so it exposes only its own name",
                        cap.library
                    ));
                };
                let mapped = project.packages[p]
                    .provides
                    .get(c)
                    .and_then(|names| names.get(seg))
                    .ok_or_else(|| format!("capability '{c}' does not provide '{seg}'"))?;
                self.package_name(p, mapped)
            }
            Target::Decl(i) => Err(format!(
                "'{}' is a declaration, and declarations are not records",
                self.decls[*i].address
            )),
            _ => Err(format!("cannot look up '{seg}' here")),
        }
    }

    fn package_name(&self, p: usize, name: &str) -> Result<Target, String> {
        let package = &self.project.packages[p];
        match self.package_items[p].get(name) {
            Some(item) if item.exported => Ok(Target::PackageItem {
                package: p,
                name: name.into(),
            }),
            Some(_) => Err(format!(
                "'{name}' is not exported from package {}",
                package.id
            )),
            None if package.skills.contains_key(name) => Ok(Target::Skill {
                package: p,
                name: name.into(),
            }),
            None => Err(format!("package {} has no '{name}'", package.id)),
        }
    }

    /// The canonical string a reference to `target` lowers to, if it can be referenced.
    pub fn canonical(&self, target: &Target) -> Option<String> {
        match target {
            Target::Decl(i) => Some(self.decls[*i].address.clone()),
            Target::PackageItem { package, name } | Target::Skill { package, name } => {
                Some(format!("{}::{name}", self.project.packages[*package].id))
            }
            Target::Capability(c) => Some(c.clone()),
            _ => None,
        }
    }
}

fn clean_display(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in FsPath::new(path).components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if out.pop().is_none() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str().to_str().unwrap_or_default()),
        }
    }
    out.join("/")
}
