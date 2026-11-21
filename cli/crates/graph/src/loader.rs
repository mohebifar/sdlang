//! Finds a project's modules and files, reads its lock and loads its packages.
//!
//! A project root holds `spec/` (one module per directory, the root module named "")
//! and, optionally, `spec.lock`. Until the lock format is settled (open item O.5) the lock
//! is a stand-in: `{ "packages": [{ "id": "npm:<name>", "path": "<dir>" }] }`, each
//! directory holding a `package.json` with an `intlang` entry. A package imported with
//! `import package "./dir"` is loaded from that directory directly.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use intlang_syntax::ast::{File, ImportSource, Statement};
use intlang_syntax::strings::Part;
use intlang_syntax::{lexer, parse};
use serde_json::Value;

use crate::error::{GResult, GraphError};

/// One parsed `.int` file.
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
    /// Canonical id: `npm:<package name>`, or `local:<path>` for a local package.
    pub id: String,
    /// The npm package name, as `use "<name>" for ...` writes it.
    pub npm_name: String,
    /// The import name from the manifest's `intlang.name`.
    pub import_name: String,
    /// Capability export name -> package declaration name, per capability.
    pub provides: BTreeMap<String, BTreeMap<String, String>>,
    /// Skill names declared in the manifest.
    pub skills: Vec<String>,
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

fn read_source(root: &Path, path: &Path) -> GResult<SourceFile> {
    let raw = fs::read_to_string(path)
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
            load_modules(root, &entry, &child, out)?;
        } else if entry.extension().is_some_and(|e| e == "int") {
            files.push(read_source(root, &entry)?);
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

fn load_package(root: &Path, id: String, dir: &Path) -> GResult<Package> {
    let manifest_path = dir.join("package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|e| GraphError::plain(format!("cannot read {}: {e}", manifest_path.display())))?;
    let manifest: Value = serde_json::from_str(&text).map_err(|e| {
        GraphError::plain(format!(
            "{} is not valid JSON: {e}",
            manifest_path.display()
        ))
    })?;
    let intlang = manifest.get("intlang").ok_or_else(|| {
        GraphError::plain(format!("{} has no intlang entry", manifest_path.display()))
    })?;
    let str_of = |v: Option<&Value>, what: &str| -> GResult<String> {
        v.and_then(Value::as_str).map(String::from).ok_or_else(|| {
            GraphError::plain(format!("{}: missing {what}", manifest_path.display()))
        })
    };
    let npm_name = str_of(manifest.get("name"), "name")?;
    let import_name = str_of(intlang.get("name"), "intlang.name")?;
    let entry = str_of(intlang.get("entry"), "intlang.entry")?;
    let provides = intlang
        .get("provides")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(cap, names)| {
            let names = names
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect();
            (cap.clone(), names)
        })
        .collect();
    let skills = intlang
        .get("skills")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let files = vec![read_source(root, &dir.join(entry))?];
    Ok(Package {
        id,
        npm_name,
        import_name,
        provides,
        skills,
        files,
        root: dir.to_path_buf(),
    })
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
    let spec = root.join("spec");
    if !spec.is_dir() {
        return Err(GraphError::plain(format!(
            "{} has no spec/ directory",
            root.display()
        )));
    }
    let mut modules = BTreeMap::new();
    load_modules(root, &spec, "", &mut modules)?;

    let lock_path = root.join("spec.lock");
    let lock: Value = if lock_path.is_file() {
        let text = fs::read_to_string(&lock_path)
            .map_err(|e| GraphError::plain(format!("cannot read spec.lock: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| GraphError::plain(format!("spec.lock is not valid JSON: {e}")))?
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
                "spec.lock: each package needs an id and a path",
            ));
        };
        packages.push(load_package(root, id.to_string(), &root.join(path))?);
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
                let id = format!("local:{}", clean(Path::new(&name)).display());
                if packages.iter().any(|p: &Package| p.id == id) {
                    continue;
                }
                packages.push(load_package(root, id, &root.join(&name))?);
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
            relative(Path::new("/a/b"), Path::new("/a/b/spec/x.int")),
            PathBuf::from("spec/x.int")
        );
        assert_eq!(
            relative(Path::new("/a/b"), Path::new("/a/p/x.int")),
            PathBuf::from("../p/x.int")
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
