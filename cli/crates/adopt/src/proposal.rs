//! The ownership mapping `spec import` proposes for the files it was pointed at. It claims
//! nothing else and never two owners for one file: a file two declarations could own is a
//! collision, claimed by neither.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use intlang_apply::evidence::requirements;
use intlang_graph::loader::Project;
use intlang_plan::materialise::{materialise, Materialisation};
use intlang_plan::state::{declarations, StateRecord, Store};
use intlang_plan::Acquisition;
use serde_json::{json, Value};

use crate::error::{AdResult, AdoptError};
use crate::scan::{expand, is_code, scan, Scanned};

/// What the import claims for one declaration the spec already has.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Claim {
    pub kind: String,
    /// Slot key -> path.
    pub slots: BTreeMap<String, String>,
    /// Created output key -> (root key, path relative to the root, path).
    pub created: BTreeMap<String, (String, String, String)>,
    /// Requirement member -> `(test output member, case)` from tags equal to its key.
    pub tests: BTreeMap<String, Vec<(String, String)>>,
    /// Requirement members no test tag names.
    pub untested: Vec<String>,
}

impl Claim {
    pub fn acquisition(&self) -> Acquisition {
        Acquisition {
            slots: self.slots.keys().cloned().collect(),
            created: self
                .created
                .iter()
                .map(|(k, (root, rel, _))| (k.clone(), (root.clone(), rel.clone())))
                .collect(),
            tests: self.tests.clone(),
        }
    }
}

/// A `lib` proposed for an existing code file that no declaration owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewLib {
    /// Dotted module path; empty for the root module.
    pub module: String,
    pub name: String,
    pub path: String,
    /// True when the lib's template does not give `path`, so the lib names it.
    pub with_path: bool,
    pub exports: Vec<String>,
    pub importers: Vec<String>,
    /// Declarations owning or claiming the importers.
    pub importing: Vec<String>,
}

impl NewLib {
    pub fn address(&self) -> String {
        if self.module.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.module, self.name)
        }
    }

    /// Several declarations use several of its exports: one integration owner, with the
    /// other two choices offered.
    pub fn integration(&self) -> bool {
        self.exports.len() >= 2 && self.importing.len() >= 2
    }

    /// The `.int` source `--accept` writes.
    pub fn source(&self) -> String {
        let mut s = format!("export lib {} {{\n", self.name);
        if self.with_path {
            let _ = writeln!(s, "  path: \"{}\"", self.path);
        }
        let _ = writeln!(
            s,
            "  summary: \"Adopted as it was from {}; say what it does\"",
            self.path
        );
        let names: Vec<String> = self.exports.iter().map(|e| format!("\"{e}\"")).collect();
        let _ = writeln!(s, "  exports: [{}]", names.join(", "));
        s.push_str("}\n");
        s
    }

    /// Where `--accept` writes it.
    pub fn file(&self) -> String {
        let dir = if self.module.is_empty() {
            "spec".to_string()
        } else {
            format!("spec/{}", self.module.replace('.', "/"))
        };
        format!("{dir}/{}.int", self.name)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Proposal {
    /// The paths as given, for the accept hint.
    pub paths: Vec<String>,
    pub claims: BTreeMap<String, Claim>,
    pub new_libs: Vec<NewLib>,
    /// (path, owner) for files a state record already owns.
    pub owned: Vec<(String, String)>,
    /// (path, why) for files that cannot be claimed without a choice.
    pub collisions: Vec<(String, String)>,
    /// Pointed-at files nothing claims that are not code, or that a kind names as an
    /// input (a root layout): left unmanaged.
    pub unmanaged: Vec<String>,
    /// (address, slot, path) of slots of claimed declarations whose file exists but was
    /// not pointed at; nothing claims it.
    pub unclaimed: Vec<(String, String, String)>,
    /// (address, slot, path) of slots with no file; the next plan's update writes them.
    pub empty: Vec<(String, String, String)>,
    /// (declaration, lib) pairs: its code imports the lib's file and its spec needs
    /// `uses` on it.
    pub grants: Vec<(String, String)>,
    /// (file, specifier, why) for imports of project files nothing owns or claims.
    pub unresolved: Vec<(String, String, String)>,
}

impl Proposal {
    pub fn acceptable(&self) -> bool {
        self.collisions.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.claims.is_empty() && self.new_libs.is_empty()
    }
}

/// `snake_case` identifier for a file stem: `LoginForm` -> `login_form`.
pub fn ident(stem: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        if c.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_lower = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

fn identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn stem(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.split('.').next().unwrap_or(file)
}

/// The module and name of a lib for `path`: from the lib template when some module and
/// name reproduce `path`, otherwise a lib that names the file with `path`.
fn lib_name(project: &Project, path: &str) -> (String, String, bool) {
    let name = ident(stem(path));
    let dirs: Vec<&str> = path.split('/').collect();
    let dirs = &dirs[..dirs.len() - 1];
    // A template fits only with a real module; an empty one would collapse a segment.
    for i in 0..dirs.len() {
        let module = dirs[i..].join(".");
        if !module.split('.').all(identifier) || !identifier(&name) {
            continue;
        }
        let address = if module.is_empty() {
            name.clone()
        } else {
            format!("{module}.{name}")
        };
        let decl = json!({ "address": address, "module": module, "kind": "core:lib", "sort": "managed", "fields": {} });
        let graph = json!({ "declarations": [decl] });
        let fits = materialise(project, &graph)
            .ok()
            .and_then(|m| m.get(&address).and_then(|m| m.slots.get("main").cloned()))
            .is_some_and(|main| main == path);
        if fits {
            return (module, name, false);
        }
    }
    let parent = dirs.last().map(|d| ident(d)).unwrap_or_default();
    let generic = ["", "src", "lib", "app", "server", "utils", "components"];
    let module = if generic.contains(&parent.as_str()) {
        name.clone()
    } else {
        parent
    };
    let name = if name == module {
        format!("{name}_lib")
    } else {
        name
    };
    (module, name, true)
}

/// Who could own `path`: `(address, Some(slot) | None for a creation root, root key)`.
fn candidates(
    mats: &BTreeMap<String, Materialisation>,
    path: &str,
) -> Vec<(String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    for (address, m) in mats {
        for (slot, p) in &m.slots {
            if p == path {
                out.push((address.clone(), Some(slot.clone()), None));
            }
        }
        for (root, dir) in &m.roots {
            if path.starts_with(dir.as_str()) && path.len() > dir.len() {
                out.push((address.clone(), None, Some(root.clone())));
            }
        }
    }
    out
}

/// Proposes a mapping for `paths` (files or directories, relative to `root`).
pub fn propose(root: &Path, paths: &[String]) -> AdResult<Proposal> {
    let project = intlang_graph::loader::load(root).map_err(|e| AdoptError::new(e.render()))?;
    let graph =
        intlang_graph::lower::lower_project(&project).map_err(|e| AdoptError::new(e.render()))?;
    let mats = materialise(&project, &graph)?;
    let records = Store::new(root).records()?;
    let scanned = scan(root)?;
    let pointed = expand(root, &scanned, paths)?;
    let mut p = Proposal {
        paths: paths.to_vec(),
        ..Proposal::default()
    };
    let owner_of = owners(&records);
    let decls = declarations(&graph);
    // Framework files a kind names as inputs stay unmanaged unless a slot claims them.
    let kind_inputs: BTreeSet<String> = mats
        .keys()
        .filter_map(|a| decls.get(a.as_str()))
        .flat_map(|d| intlang_apply::checks::kind_inputs(&project, d))
        .collect();
    let mut lib_files: Vec<String> = Vec::new();
    for path in &pointed {
        if let Some(owner) = owner_of.get(path) {
            p.owned.push((path.clone(), owner.clone()));
            continue;
        }
        let found = candidates(&mats, path);
        match found.as_slice() {
            [] if kind_inputs.contains(path) => p.unmanaged.push(path.clone()),
            [] if is_code(path) => lib_files.push(path.clone()),
            [] => p.unmanaged.push(path.clone()),
            [(address, slot, root_key)] => {
                if records.contains_key(address) {
                    p.collisions.push((
                        path.clone(),
                        format!("{address} already has a state record; adding outputs to it is a planned operation of its own"),
                    ));
                    continue;
                }
                let claim = p.claims.entry(address.clone()).or_default();
                match (slot, root_key) {
                    (Some(slot), _) => {
                        claim.slots.insert(slot.clone(), path.clone());
                    }
                    (None, Some(root_key)) => {
                        let dir = &mats[address].roots[root_key];
                        let rel = path[dir.len()..].to_string();
                        let key = ident(stem(path));
                        if mats[address].slots.contains_key(&key)
                            || claim.created.contains_key(&key)
                            || !identifier(&key)
                        {
                            p.collisions.push((
                                path.clone(),
                                format!("{address}: created output key '{key}' is taken or not an identifier"),
                            ));
                            continue;
                        }
                        claim
                            .created
                            .insert(key, (root_key.clone(), rel, path.clone()));
                    }
                    _ => {}
                }
            }
            many => {
                let names: Vec<String> = many
                    .iter()
                    .map(|(a, s, r)| match (s, r) {
                        (Some(s), _) => format!("{a}#outputs.{s}"),
                        (_, Some(r)) => format!("{a} (creation root {r})"),
                        _ => a.clone(),
                    })
                    .collect();
                p.collisions.push((
                    path.clone(),
                    format!("could be owned by {}", names.join(" and ")),
                ));
            }
        }
    }
    // A claimed declaration whose collisions removed every claim is not claimed.
    p.claims
        .retain(|_, c| !c.slots.is_empty() || !c.created.is_empty());
    for (address, claim) in &mut p.claims {
        claim.kind = decls[address.as_str()]["kind"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        for (slot, path) in &mats[address].slots {
            if claim.slots.contains_key(slot) {
                continue;
            }
            if root.join(path).is_file() {
                p.unclaimed
                    .push((address.clone(), slot.clone(), path.clone()));
            } else {
                p.empty.push((address.clone(), slot.clone(), path.clone()));
            }
        }
        bind_tests(&graph, decls[address.as_str()], address, claim, &scanned)?;
    }
    // New libs for unowned code files.
    let taken: BTreeSet<String> = decls.keys().map(|a| a.to_string()).collect();
    let mut proposed: BTreeSet<String> = BTreeSet::new();
    for path in lib_files {
        let (module, name, with_path) = lib_name(&project, &path);
        let lib = NewLib {
            module,
            name,
            path: path.clone(),
            with_path,
            exports: scanned
                .code
                .get(&path)
                .map(|f| f.exports.clone())
                .unwrap_or_default(),
            importers: scanned
                .importers
                .get(&path)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default(),
            importing: vec![],
        };
        let address = lib.address();
        if taken.contains(&address) || !proposed.insert(address.clone()) {
            p.collisions.push((
                path,
                format!("the proposed lib {address} is already declared; name it by hand"),
            ));
            continue;
        }
        p.new_libs.push(lib);
    }
    // Who owns each file once the proposal is accepted.
    let mut will_own: BTreeMap<String, String> = owner_of.clone();
    for (address, claim) in &p.claims {
        for path in claim.slots.values() {
            will_own.insert(path.clone(), address.clone());
        }
        for (_, _, path) in claim.created.values() {
            will_own.insert(path.clone(), address.clone());
        }
    }
    for lib in &p.new_libs {
        will_own.insert(lib.path.clone(), lib.address());
    }
    let declared_owner = |path: &str| -> Option<String> {
        will_own
            .get(path)
            .cloned()
            .or_else(|| match candidates(&mats, path).as_slice() {
                [(a, _, _)] => Some(a.clone()),
                _ => None,
            })
    };
    for lib in &mut p.new_libs {
        let mut importing: BTreeSet<String> = BTreeSet::new();
        for file in &lib.importers {
            if let Some(owner) = declared_owner(file) {
                if owner != lib.address() {
                    importing.insert(owner);
                }
            }
        }
        lib.importing = importing.into_iter().collect();
        for d in &lib.importing {
            p.grants.push((d.clone(), lib.address()));
        }
    }
    // Imports of project files that nothing will own.
    let claimed_files: Vec<&String> = will_own
        .iter()
        .filter(|(_, a)| p.claims.contains_key(*a) || p.new_libs.iter().any(|l| l.address() == **a))
        .map(|(f, _)| f)
        .collect();
    for file in claimed_files {
        let Some(src) = scanned.code.get(file) else {
            continue;
        };
        for target in &src.imports {
            if !will_own.contains_key(target) {
                p.unresolved.push((
                    file.clone(),
                    target.clone(),
                    "which nothing owns or claims".into(),
                ));
            }
        }
        for (spec, why) in &src.broken {
            p.unresolved
                .push((file.clone(), spec.clone(), format!("which {why}")));
        }
    }
    p.grants.sort();
    p.grants.dedup();
    p.unresolved.sort();
    Ok(p)
}

fn owners(records: &BTreeMap<String, StateRecord>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for r in records.values() {
        for o in r.outputs.values() {
            out.insert(o.path.clone(), r.address.clone());
        }
    }
    out
}

/// Binds each requirement to the claimed test outputs' cases tagged with its key.
fn bind_tests(
    graph: &Value,
    decl: &Value,
    address: &str,
    claim: &mut Claim,
    scanned: &Scanned,
) -> AdResult<()> {
    let tests: Vec<(String, Vec<String>)> = claim
        .slots
        .iter()
        .filter(|(slot, _)| slot.contains("test"))
        .map(|(slot, path)| {
            let cases = scanned
                .code
                .get(path)
                .map(|f| f.cases.clone())
                .unwrap_or_default();
            (format!("{address}#outputs.{slot}"), cases)
        })
        .collect();
    for req in requirements(graph, decl)? {
        let key = req.member.rsplit('.').next().unwrap_or_default();
        let bound: Vec<(String, String)> = tests
            .iter()
            .filter(|(_, cases)| cases.iter().any(|c| c == key))
            .map(|(output, _)| (output.clone(), key.to_string()))
            .collect();
        if bound.is_empty() {
            if req.policy.tests {
                claim.untested.push(req.member.clone());
            }
        } else {
            claim.tests.insert(req.member.clone(), bound);
        }
    }
    Ok(())
}

/// The proposal as text for a reviewer, in the shape SPEC.md's walk-through shows.
pub fn render(p: &Proposal) -> String {
    let mut s = String::new();
    for lib in &p.new_libs {
        let _ = write!(s, "  {}  exports {}", lib.path, list(&lib.exports));
        if !lib.importers.is_empty() {
            let _ = write!(s, "; imported by {}", lib.importers.join(", "));
        }
        s.push('\n');
        if lib.integration() {
            let _ = writeln!(
                s,
                "  proposal: one integration owner (a file serving {} would-be declarations)",
                lib.importing.len()
            );
            let _ = writeln!(
                s,
                "    other choices: extern (reference its symbols, leave it unmanaged) or a planned extraction (declare the parts, then edit the spec)"
            );
        }
        let _ = writeln!(
            s,
            "    + lib  {}   outputs.main = {}   exports {}   strategy: acquire",
            lib.address(),
            lib.path,
            list(&lib.exports)
        );
    }
    for (address, c) in &p.claims {
        let _ = writeln!(
            s,
            "    + {}  {}   strategy: acquire",
            intlang_apply::checks::kind_short(&c.kind),
            address
        );
        for (slot, path) in &c.slots {
            let _ = writeln!(s, "        outputs.{slot} = {path}");
        }
        for (key, (root, _, path)) in &c.created {
            let _ = writeln!(s, "        outputs.{key} = {path}   (created under {root})");
        }
        for (member, tests) in &c.tests {
            let cases: Vec<String> = tests
                .iter()
                .map(|(o, case)| format!("{o} [case:{case}]"))
                .collect();
            let _ = writeln!(s, "        test {member}: {}", cases.join(", "));
        }
        for member in &c.untested {
            let _ = writeln!(s, "        no test tag names {member}");
        }
    }
    let mut by_lib: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (d, lib) in &p.grants {
        by_lib.entry(lib.as_str()).or_default().push(d.as_str());
    }
    for (lib, ds) in by_lib {
        let short = lib.rsplit('.').next().unwrap_or(lib);
        let _ = writeln!(
            s,
            "    ~ {}   need uses: [{short}]   (their imports need a grant; add it to their spec)",
            ds.join(", ")
        );
    }
    let section = |s: &mut String, title: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            let _ = writeln!(s, "  {title}");
            for l in lines {
                let _ = writeln!(s, "    {l}");
            }
        }
    };
    section(
        &mut s,
        "collisions (nothing is claimed for these):",
        p.collisions
            .iter()
            .map(|(path, why)| format!("{path}: {why}"))
            .collect(),
    );
    section(
        &mut s,
        "already owned:",
        p.owned
            .iter()
            .map(|(path, a)| format!("{path} by {a}"))
            .collect(),
    );
    section(
        &mut s,
        "unresolved dependencies:",
        p.unresolved
            .iter()
            .map(|(f, t, why)| format!("{f} imports {t}, {why}"))
            .collect(),
    );
    section(
        &mut s,
        "existing files not pointed at (not claimed):",
        p.unclaimed
            .iter()
            .map(|(a, slot, path)| format!("{a}#outputs.{slot} = {path}"))
            .collect(),
    );
    section(
        &mut s,
        "empty slots (the next plan's update writes them):",
        p.empty
            .iter()
            .map(|(a, slot, path)| format!("{a}#outputs.{slot} = {path}"))
            .collect(),
    );
    section(&mut s, "left unmanaged:", p.unmanaged.clone());
    if p.is_empty() {
        s.push_str("  nothing to claim\n");
    } else if p.acceptable() {
        let mut writes: Vec<String> = p.new_libs.iter().map(NewLib::file).collect();
        writes.push(intlang_plan::PENDING_IMPORTS.to_string());
        let _ = writeln!(
            s,
            "  accept with: spec import {} --accept  (writes {})",
            p.paths.join(" "),
            writes.join(" and ")
        );
    } else {
        s.push_str("  resolve the collisions before accepting\n");
    }
    s
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "nothing".into()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers() {
        assert_eq!(ident("LoginForm"), "login_form");
        assert_eq!(ident("task_row"), "task_row");
        assert_eq!(ident("forgot-password"), "forgot_password");
        assert_eq!(ident("2fa"), "_2fa");
    }
}
