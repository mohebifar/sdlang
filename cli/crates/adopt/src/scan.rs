//! The project's files as `spec import` sees them: which are code, what each exports and
//! imports, and who imports each file. Static reading only; no package code runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use intlang_apply::imports::{aliases, exports, resolve_local, specifiers, CODE_EXTENSIONS};

use crate::error::{AdResult, AdoptError};

/// Directories never scanned: tool state, dependencies and the spec tree itself.
const SKIP: &[&str] = &[".git", ".spec", "node_modules", "spec", "spec-packages"];

/// One code file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceFile {
    pub exports: Vec<String>,
    /// Project files it imports, resolved.
    pub imports: BTreeSet<String>,
    /// Specifiers that point into the repository but resolve to no file.
    pub broken: Vec<(String, &'static str)>,
    /// `[case:<tag>]` test tags, in order of first appearance.
    pub cases: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Scanned {
    /// Every project file, repository-relative, sorted.
    pub files: BTreeSet<String>,
    /// Code files by path.
    pub code: BTreeMap<String, SourceFile>,
    /// File -> the code files that import it.
    pub importers: BTreeMap<String, BTreeSet<String>>,
}

pub fn is_code(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| CODE_EXTENSIONS.contains(&ext))
}

fn skipped(rel: &str) -> bool {
    rel.split('/').next().is_some_and(|top| SKIP.contains(&top))
}

/// Every file under `dir` (repository-relative), skipping tool and dependency
/// directories and hidden directories such as `.next`.
fn walk(root: &Path, dir: &Path) -> AdResult<Vec<String>> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(dir).sort_by_file_name().into_iter();
    for entry in walker.filter_entry(|e| {
        let rel = e.path().strip_prefix(root).unwrap_or(e.path());
        let rel = rel.to_string_lossy();
        let hidden = e.depth() > 0
            && e.file_name().to_string_lossy().starts_with('.')
            && e.file_type().is_dir();
        rel.is_empty() || !(skipped(&rel) || hidden)
    }) {
        let entry = entry.map_err(|e| AdoptError::new(e.to_string()))?;
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(out)
}

/// `[case:<tag>]` and `[case:<tag>:<key>]` tags in test text.
pub fn case_tags(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, _) in text.match_indices("[case:") {
        let rest = &text[i + "[case:".len()..];
        let tag: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        let after = rest[tag.len()..].chars().next();
        let shaped = tag.chars().next().is_some_and(|c| !c.is_ascii_digit());
        if shaped && matches!(after, Some(']' | ':')) && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}

/// Reads the whole project.
pub fn scan(root: &Path) -> AdResult<Scanned> {
    let files: BTreeSet<String> = walk(root, root)?.into_iter().collect();
    let alias = aliases(root);
    let mut code = BTreeMap::new();
    let mut importers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files.iter().filter(|p| is_code(p)) {
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        let mut f = SourceFile {
            exports: exports(&text),
            cases: case_tags(&text),
            ..SourceFile::default()
        };
        for spec in specifiers(&text) {
            match resolve_local(root, &alias, path, &spec) {
                None => {}
                Some(Ok(target)) => {
                    importers
                        .entry(target.clone())
                        .or_default()
                        .insert(path.clone());
                    f.imports.insert(target);
                }
                Some(Err(why)) => f.broken.push((spec, why)),
            }
        }
        code.insert(path.clone(), f);
    }
    Ok(Scanned {
        files,
        code,
        importers,
    })
}

/// The pointed-at paths as files: each path is a file or a directory, relative to the
/// project root.
pub fn expand(root: &Path, scanned: &Scanned, paths: &[String]) -> AdResult<Vec<String>> {
    let mut out = BTreeSet::new();
    for p in paths {
        let rel = p.trim_start_matches("./").trim_end_matches('/');
        let full = root.join(rel);
        if rel.is_empty() || rel == "." {
            out.extend(scanned.files.iter().cloned());
        } else if full.is_file() {
            if skipped(rel) {
                return Err(AdoptError::new(format!(
                    "{rel}: spec import never claims files under {}",
                    SKIP.join(", ")
                )));
            }
            out.insert(rel.to_string());
        } else if full.is_dir() {
            let prefix = format!("{rel}/");
            out.extend(
                scanned
                    .files
                    .iter()
                    .filter(|f| f.starts_with(&prefix))
                    .cloned(),
            );
        } else {
            return Err(AdoptError::new(format!("{rel}: no such file or directory")));
        }
    }
    Ok(out.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags() {
        let text = r#"it("[case:valid] signs in", ...); it("[case:generic_error:wrong] x");
            it("[case:valid] again"); it("[case:Bad]"); it("[case:9x]");"#;
        assert_eq!(case_tags(text), ["valid", "generic_error"]);
    }
}
