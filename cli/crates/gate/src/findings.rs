//! The findings of `spec check` (SPEC.md, Team workflow) and how they are gathered.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use intlang_apply::checks::{resolve_checks, run_input_digest, Closure};
use intlang_apply::evidence::requirements;
use intlang_graph::loader::Project;
use intlang_plan::materialise::{materialise, Materialisation};
use intlang_plan::state::{declarations, file_digest, StateRecord, Store};
use serde_json::{json, Value};

use crate::currency::{requirement_status, Status};
use crate::error::{GResult, GateError};
use crate::ownership::ownership_findings;

/// Finding kinds, in the order SPEC.md's table lists them and findings print.
pub const KINDS: [&str; 6] = [
    "unapplied",
    "drift",
    "ownership",
    "evidence",
    "reconcile",
    "unspecified",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    /// An address, a requirement member or a path.
    pub subject: String,
    pub detail: String,
    /// Reported without failing the check (an allowed kind, or unspecified without
    /// `--strict`).
    pub warning: bool,
}

impl Finding {
    pub fn new(kind: &'static str, subject: &str, detail: String) -> Finding {
        Finding {
            kind,
            subject: subject.to_string(),
            detail,
            warning: false,
        }
    }

    pub fn to_value(&self) -> Value {
        json!({ "kind": self.kind, "subject": self.subject, "detail": self.detail, "warning": self.warning })
    }
}

pub struct CheckOptions {
    pub root: PathBuf,
    /// The execution profile run input digests are computed for.
    pub profile: String,
    /// Kinds reported without failing.
    pub allow: Vec<String>,
    pub strict: bool,
    pub rerun: bool,
    pub check_timeout: Duration,
}

impl CheckOptions {
    pub fn new(root: PathBuf) -> CheckOptions {
        CheckOptions {
            root,
            profile: "local".into(),
            allow: vec![],
            strict: false,
            rerun: false,
            check_timeout: Duration::from_secs(900),
        }
    }
}

/// What every part of the check reads.
pub struct Loaded {
    pub project: Project,
    pub graph: Value,
    pub mats: BTreeMap<String, Materialisation>,
    pub records: BTreeMap<String, StateRecord>,
}

impl Loaded {
    /// `(member, path)` of a declaration's outputs: its slots and its recorded created
    /// outputs, as apply computes them.
    pub fn outputs_of(&self, address: &str) -> Vec<(String, String)> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        if let Some(m) = self.mats.get(address) {
            for (k, p) in &m.slots {
                out.insert(format!("{address}#outputs.{k}"), p.clone());
            }
        }
        if let Some(r) = self.records.get(address) {
            for (k, o) in r.outputs.iter().filter(|(_, o)| o.origin == "created") {
                out.insert(format!("{address}#outputs.{k}"), o.path.clone());
            }
        }
        out.into_iter().collect()
    }

    /// The input digest a run of `check` on `address` would have, read from `dir`.
    pub fn run_digest(
        &self,
        dir: &std::path::Path,
        profile: &str,
        check: &str,
        address: &str,
    ) -> GResult<Option<String>> {
        let Some(decl) = declarations(&self.graph).get(address).copied() else {
            return Ok(None);
        };
        let Some(def) = resolve_checks(&self.project, decl)
            .into_iter()
            .find(|d| d.id == check)
        else {
            return Ok(None);
        };
        let outputs_of = |a: &str| self.outputs_of(a);
        let closure = Closure {
            dir,
            project: &self.project,
            graph: &self.graph,
            outputs_of: &outputs_of,
            profile,
        };
        Ok(Some(run_input_digest(
            &closure,
            address,
            &def.definition_hash,
            def.scope == "project",
        )?))
    }
}

fn load(options: &CheckOptions) -> GResult<(Loaded, Vec<(String, StateRecord)>)> {
    let root = &options.root;
    let project = intlang_graph::loader::load(root).map_err(|e| GateError::new(e.render()))?;
    let graph =
        intlang_graph::lower::lower_project(&project).map_err(|e| GateError::new(e.render()))?;
    let mats = materialise(&project, &graph)?;
    let entries = Store::new(root).record_entries()?;
    let mut records = BTreeMap::new();
    for (_, r) in &entries {
        records
            .entry(r.address.clone())
            .or_insert_with(|| r.clone());
    }
    Ok((
        Loaded {
            project,
            graph,
            mats,
            records,
        },
        entries,
    ))
}

/// Obligations that say nothing about the obliged declaration's own spec: evidence
/// alone, or the impact of another declaration's change (that one is reported).
fn not_own(because: &str) -> bool {
    ["unverified:", "failed:", "impact:"]
        .iter()
        .any(|p| because.starts_with(p))
}

/// Declarations whose spec differs from what was applied, as the planner sees them.
fn unapplied(options: &CheckOptions, loaded: &Loaded) -> GResult<Vec<Finding>> {
    let plan = intlang_plan::plan(&intlang_plan::PlanInputs {
        root: &options.root,
        project: &loaded.project,
        graph: &loaded.graph,
        plan_id: "check",
        baseline_commit: "git:check",
        acquire: None,
    })?;
    let mut out = Vec::new();
    for e in plan["entries"].as_array().into_iter().flatten() {
        let address = e["address"].as_str().unwrap_or_default();
        let change = e["change"].as_str().unwrap_or("none");
        let causes: Vec<&str> = e["causes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if change != "none" {
            let detail = if causes.is_empty() {
                change.to_string()
            } else {
                format!("{change} ({})", causes.join(", "))
            };
            out.push(Finding::new("unapplied", address, detail));
            continue;
        }
        if e["identity"] == "moved" {
            out.push(Finding::new(
                "unapplied",
                address,
                format!("moved from {}", e["moved_from"].as_str().unwrap_or("?")),
            ));
            continue;
        }
        let because: BTreeSet<&str> = e["obligations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|o| o["because"].as_str())
            .filter(|b| !not_own(b))
            .collect();
        if !because.is_empty() {
            let list: Vec<&str> = because.into_iter().collect();
            out.push(Finding::new(
                "unapplied",
                address,
                format!("needs verify or review ({})", list.join(", ")),
            ));
        }
    }
    for b in plan["blocked"].as_array().into_iter().flatten() {
        if b["reason"] == "frozen" {
            out.push(Finding::new(
                "unapplied",
                b["address"].as_str().unwrap_or_default(),
                "frozen, and its spec needs an update".into(),
            ));
        }
    }
    Ok(out)
}

fn drift(options: &CheckOptions, loaded: &Loaded) -> GResult<Vec<Finding>> {
    let mut out = Vec::new();
    for r in loaded.records.values() {
        for (key, o) in &r.outputs {
            let actual = file_digest(&options.root, &o.path)?;
            if actual.as_deref() == Some(o.bytes.as_str()) {
                continue;
            }
            out.push(Finding::new(
                "drift",
                &o.path,
                format!(
                    "{}#outputs.{key}: expected {}, actual {}",
                    r.address,
                    o.bytes,
                    actual.as_deref().unwrap_or("absent")
                ),
            ));
        }
    }
    Ok(out)
}

fn evidence(
    options: &CheckOptions,
    loaded: &Loaded,
    skip: &BTreeSet<String>,
) -> GResult<Vec<Finding>> {
    let decls = declarations(&loaded.graph);
    let mut out = Vec::new();
    for (address, record) in &loaded.records {
        if skip.contains(address) {
            continue;
        }
        let Some(decl) = decls.get(address.as_str()) else {
            continue;
        };
        let run_digest =
            |check: &str, a: &str| loaded.run_digest(&options.root, &options.profile, check, a);
        for req in requirements(&loaded.graph, decl)? {
            let detail = match requirement_status(&req, record, &run_digest)? {
                Status::Met => continue,
                Status::Missing => "missing".to_string(),
                Status::Failed => "failed".to_string(),
                Status::Stale(why) => format!("stale: {why}"),
            };
            out.push(Finding::new(
                "evidence",
                &req.member,
                format!("{detail} (policy {})", req.policy.label()),
            ));
        }
    }
    Ok(out)
}

/// Managed declarations with no requirements of their own or from rules (SPEC.md,
/// Verification), except those whose outputs a generator produces.
pub fn unspecified_addresses(
    graph: &Value,
    mats: &BTreeMap<String, Materialisation>,
) -> GResult<Vec<String>> {
    let decls = declarations(graph);
    let mut out = Vec::new();
    for (address, m) in mats {
        if m.generator.is_some() {
            continue;
        }
        let Some(decl) = decls.get(address.as_str()) else {
            continue;
        };
        if requirements(graph, decl)?.is_empty() {
            out.push(address.clone());
        }
    }
    Ok(out)
}

fn unspecified(loaded: &Loaded) -> GResult<Vec<Finding>> {
    Ok(unspecified_addresses(&loaded.graph, &loaded.mats)?
        .iter()
        .map(|a| {
            Finding::new(
                "unspecified",
                a,
                "no requirements: nothing about its behaviour is tested or reviewed".into(),
            )
        })
        .collect())
}

/// Every finding for the project at `options.root`, sorted by kind, then subject.
pub fn check(options: &CheckOptions) -> GResult<Vec<Finding>> {
    let (loaded, entries) = load(options)?;
    let mut out = ownership_findings(&entries);
    let misnamed = out.iter().any(|f| f.subject.starts_with(".spec/state/"));
    let mut skip: BTreeSet<String> = BTreeSet::new();
    if !misnamed {
        let u = unapplied(options, &loaded)?;
        skip.extend(u.iter().map(|f| f.subject.clone()));
        out.extend(u);
    }
    out.extend(drift(options, &loaded)?);
    out.extend(evidence(options, &loaded, &skip)?);
    if options.rerun {
        out.extend(crate::rerun::rerun(options, &loaded, &skip)?);
    }
    if options.root.join(".spec/reconcile").is_dir() {
        out.push(Finding::new(
            "reconcile",
            ".spec/reconcile",
            "a merge left state files to reconcile: run spec plan --reconcile".into(),
        ));
    }
    let mut u = unspecified(&loaded)?;
    for f in &mut u {
        f.warning = !options.strict;
    }
    out.extend(u);
    for f in &mut out {
        if options.allow.iter().any(|a| a == f.kind) {
            f.warning = true;
        }
    }
    let rank = |k: &str| KINDS.iter().position(|x| *x == k).unwrap_or(KINDS.len());
    out.sort_by(|a, b| {
        (rank(a.kind), &a.subject, &a.detail).cmp(&(rank(b.kind), &b.subject, &b.detail))
    });
    out.dedup();
    Ok(out)
}
