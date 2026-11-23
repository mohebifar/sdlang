//! Finds a project's modules and files, reads its lock and loads its packages.
//!
//! A project root holds `spec/` (one module per directory, the root module named "")
//! and, optionally, `sd.lock`. Until the lock format is settled (open item O.5) the lock
//! is a stand-in: `{ "packages": [{ "id": "npm:<name>", "path": "<dir>" }] }`. Each package
//! directory holds a `package.sd` with one `package` header, which names the package and its
//! files. The lock, not the package, says where it came from. A package imported with
//! `import package "./dir"` is loaded from that directory directly.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use sdlang_syntax::ast::{Expr, File, ImportSource, MemberValue, Statement};
use sdlang_syntax::strings::Part;
use sdlang_syntax::{lexer, parse};
use serde_json::Value;

use crate::error::{GResult, GraphError};

/// One parsed `.sd` file.
#[derive(Debug)]
pub struct SourceFile {
    /// The path shown in diagnostics and recorded in the graph (relative to the project root).
    pub path: PathBuf,
    /// The CRLF-normalised text; spans refer to it.
    pub text: String,
    pub ast: File,
}

#[derive(Debug)]
pub struct Module {
    /// Dotted path from `spec/`; "" for the root module.
    pub path: String,
    pub files: Vec<SourceFile>,
}

#[derive(Debug)]
pub struct Package {
    /// Canonical id from the lock: `npm:<package name>`, or `local:<path>` for a local package.
    pub id: String,
    /// The id without its source scheme, as `use "<name>" for ...` writes it.
    pub source_name: String,
    /// The import name from the `package` header.
    pub import_name: String,
    /// Capability export name -> package declaration name, per capability.
    pub provides: BTreeMap<String, BTreeMap<String, String>>,
    /// Skill name -> package-relative path of its Markdown.
    pub skills: BTreeMap<String, String>,
    pub files: Vec<SourceFile>,
    /// The package's directory, where its checks and generators run from.
    pub root: PathBuf,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub modules: BTreeMap<String, Module>,
    pub packages: Vec<Package>,
    /// The lock document as read (`{}` without a lock file).
    pub lock: Value,
}

impl Project {
    pub fn package_by_id(&self, id: &str) -> Option<usize> {
        self.packages.iter().position(|p| p.id == id)
    }
}

/// Text to use instead of a file's contents on disk, keyed by path (an editor's unsaved
/// buffers). A path is looked up as the loader builds it, then canonicalised.
pub type Overlay = BTreeMap<PathBuf, String>;

fn read_text(path: &Path, overlay: &Overlay) -> std::io::Result<String> {
    if let Some(text) = overlay.get(path) {
        return Ok(text.clone());
    }
    if !overlay.is_empty() {
        if let Some(text) = fs::canonicalize(path).ok().and_then(|p| overlay.get(&p)) {
            return Ok(text.clone());
        }
    }
    fs::read_to_string(path)
}

fn read_source(root: &Path, path: &Path, overlay: &Overlay) -> GResult<SourceFile> {
    let raw = read_text(path, overlay)
        .map_err(|e| GraphError::plain(format!("cannot read {}: {e}", path.display())))?;
    let text = lexer::normalize(&raw).into_owned();
    let shown = relative(root, path);
    let ast = parse(&text).map_err(|e| GraphError {
        message: e.message.clone(),
        location: Some((shown.clone(), text.clone(), e.span)),
    })?;
    Ok(SourceFile {
        path: shown,
        text,
        ast,
    })
}

/// `path` relative to `root`, lexically, with `..` where needed.
fn relative(root: &Path, path: &Path) -> PathBuf {
    let root: Vec<Component> = root.components().collect();
    let path_c: Vec<Component> = path.components().collect();
    let common = root.iter().zip(&path_c).take_while(|(a, b)| a == b).count();
    let mut out = PathBuf::new();
    for _ in common..root.len() {
        out.push("..");
    }
    for c in &path_c[common..] {
        out.push(c.as_os_str());
    }
    out
}

fn sorted_entries(dir: &Path) -> GResult<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| GraphError::plain(format!("cannot read {}: {e}", dir.display())))?
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    Ok(entries)
}

fn load_modules(
    root: &Path,
    dir: &Path,
    path: &str,
    out: &mut BTreeMap<String, Module>,
    overlay: &Overlay,
) -> GResult<()> {
    let mut files = Vec::new();
    for entry in sorted_entries(dir)? {
        if entry.is_dir() {
            let name = entry
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let child = if path.is_empty() {
                name.to_string()
            } else {
                format!("{path}.{name}")
            };
            load_modules(root, &entry, &child, out, overlay)?;
        } else if entry.extension().is_some_and(|e| e == "sd") {
            files.push(read_source(root, &entry, overlay)?);
        }
    }
    out.insert(
        path.to_string(),
        Module {
            path: path.to_string(),
            files,
        },
    );
    Ok(())
}

/// The text of a string or a dotted name.
fn text_of(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String(s) => Some(crate::scope::lit_text(s)),
        Expr::Path { path, .. } => Some(path.dotted()),
        _ => None,
    }
}

/// Reads the package in `dir` from its `package.sd`:
///
/// ```text
/// package nextjs {
///   entry: "./spec/index.sd"
///   skills { app_router: "./skills/app_router.md" }
///   provides { ui { button: Button } }
/// }
/// ```
///
/// `entry` defaults to `./spec/index.sd`. `options` and `migrations` are reserved.
fn load_package(root: &Path, id: String, dir: &Path, overlay: &Overlay) -> GResult<Package> {
    let manifest = read_source(root, &dir.join("package.sd"), overlay)?;
    let mut header = None;
    for statement in &manifest.ast.statements {
        match statement {
            Statement::Package(h) if header.is_none() => header = Some(h),
            other => {
                return Err(GraphError::at(
                    &manifest,
                    other.span(),
                    "package.sd holds one package header and nothing else",
                ))
            }
        }
    }
    let Some(header) = header else {
        return Err(GraphError::plain(format!(
            "{}: no package header",
            manifest.path.display()
        )));
    };
    let mut entry = "./spec/index.sd".to_string();
    let mut skills = BTreeMap::new();
    let mut provides = BTreeMap::new();
    for member in &header.body.members {
        let bad = |what: &str| GraphError::at(&manifest, member.name.span, what.to_string());
        match (member.name.text.as_str(), &member.value) {
            ("entry", MemberValue::Field(f)) => match &f.value {
                Expr::String(s) => entry = crate::scope::lit_text(s),
                _ => return Err(bad("entry is a path in quotes")),
            },
            ("skills", MemberValue::Section(section)) => {
                for skill in &section.members {
                    let MemberValue::Field(f) = &skill.value else {
                        return Err(bad("each skill is `name: \"path\"`"));
                    };
                    let Expr::String(path) = &f.value else {
                        return Err(bad("each skill is `name: \"path\"`"));
                    };
                    skills.insert(skill.name.text.clone(), crate::scope::lit_text(path));
                }
            }
            ("provides", MemberValue::Section(section)) => {
                for capability in &section.members {
                    let MemberValue::Section(names) = &capability.value else {
                        return Err(bad("provides lists `capability { export: declaration }`"));
                    };
                    let mut map = BTreeMap::new();
                    for n in &names.members {
                        let target = match &n.value {
                            MemberValue::Field(f) => text_of(&f.value),
                            _ => None,
                        };
                        let Some(target) = target else {
                            return Err(bad("provides lists `capability { export: declaration }`"));
                        };
                        map.insert(n.name.text.clone(), target);
                    }
                    provides.insert(capability.name.text.clone(), map);
                }
            }
            ("options" | "migrations", MemberValue::Field(_)) => {}
            (other, _) => {
                return Err(bad(&format!(
                    "unknown package field `{other}`; a package header has entry, skills, provides, options and migrations"
                )))
            }
        }
    }
    let source_name = id
        .split_once(':')
        .map_or(id.as_str(), |(_, n)| n)
        .to_string();
    let files = vec![read_source(root, &dir.join(entry), overlay)?];
    Ok(Package {
        id,
        source_name,
        import_name: header.name.text.clone(),
        provides,
        skills,
        files,
        root: dir.to_path_buf(),
    })
}

/// `path` with `/` between its components on every platform, for ids that get hashed.
fn slashed(path: &Path) -> String {
    let parts: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    parts.join("/")
}

/// Normalises `./a/../b` style paths lexically.
fn clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Loads the project rooted at `root`.
pub fn load(root: &Path) -> GResult<Project> {
    load_with(root, &Overlay::new())
}

/// Loads the project rooted at `root`, reading the files in `overlay` from it.
pub fn load_with(root: &Path, overlay: &Overlay) -> GResult<Project> {
    let spec = root.join("spec");
    if !spec.is_dir() {
        return Err(GraphError::plain(format!(
            "{} has no spec/ directory",
            root.display()
        )));
    }
    let mut modules = BTreeMap::new();
    load_modules(root, &spec, "", &mut modules, overlay)?;

    let lock_path = root.join("sd.lock");
    let lock: Value = if lock_path.is_file() {
        let text = fs::read_to_string(&lock_path)
            .map_err(|e| GraphError::plain(format!("cannot read sd.lock: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| GraphError::plain(format!("sd.lock is not valid JSON: {e}")))?
    } else {
        Value::Object(Default::default())
    };
    let mut packages = Vec::new();
    for entry in lock
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = entry.get("id").and_then(Value::as_str);
        let path = entry.get("path").and_then(Value::as_str);
        let (Some(id), Some(path)) = (id, path) else {
            return Err(GraphError::plain(
                "sd.lock: each package needs an id and a path",
            ));
        };
        packages.push(load_package(
            root,
            id.to_string(),
            &root.join(path),
            overlay,
        )?);
    }
    // Local packages named by path in `import package "./dir"`.
    for module in modules.values() {
        for file in &module.files {
            for statement in &file.ast.statements {
                let Statement::Import(import) = statement else {
                    continue;
                };
                let ImportSource::Package(lit) = &import.source else {
                    continue;
                };
                let name: String = lit
                    .parts
                    .iter()
                    .map(|p| match p {
                        Part::Text(t) => t.as_str(),
                        _ => "",
                    })
                    .collect();
                if !(name.starts_with("./") || name.starts_with("../")) {
                    continue;
                }
                let id = format!("local:{}", slashed(&clean(Path::new(&name))));
                if packages.iter().any(|p: &Package| p.id == id) {
                    continue;
                }
                packages.push(load_package(root, id, &root.join(&name), overlay)?);
            }
        }
    }
    Ok(Project {
        root: root.to_path_buf(),
        modules,
        packages,
        lock,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative(Path::new("/a/b"), Path::new("/a/b/spec/x.sd")),
            PathBuf::from("spec/x.sd")
        );
        assert_eq!(
            relative(Path::new("/a/b"), Path::new("/a/p/x.sd")),
            PathBuf::from("../p/x.sd")
        );
    }

    #[test]
    fn clean_paths() {
        assert_eq!(
            clean(Path::new("./spec-packages/rust")),
            PathBuf::from("spec-packages/rust")
        );
        assert_eq!(clean(Path::new("./a/../b")), PathBuf::from("b"));
    }
}
