//! The check protocol (SPEC.md, Packages -> Check protocol): check definitions read from
//! packages, and build, test and conformance runs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use intlang_canon::digest;
use intlang_graph::loader::{Package, Project};
use intlang_graph::scope::lit_text;
use intlang_plan::materialise::{kind_body, kind_package};
use intlang_plan::state::{declarations, file_digest, ORDERING};
use intlang_syntax::ast::{Block, Exportable, Expr, MemberValue, Statement};
use serde_json::{json, Value};

use crate::error::AResult;

/// A check definition as its package declares it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckDef {
    /// `<package import name>.<check name>`.
    pub id: String,
    pub purpose: String,
    pub scope: String,
    pub run: Vec<String>,
    /// Rule -> the kinds (short names) it applies to.
    pub rules: BTreeMap<String, Vec<String>>,
    pub advisory: Vec<String>,
    pub package_root: PathBuf,
    pub definition_hash: String,
}

fn field<'b>(body: &'b Block, name: &str) -> Option<&'b Expr> {
    body.members.iter().find_map(|m| match &m.value {
        MemberValue::Field(f) if m.name.text == name => Some(&f.value),
        _ => None,
    })
}

fn names(expr: Option<&Expr>) -> Vec<String> {
    match expr {
        Some(Expr::List { items, .. }) => items
            .iter()
            .filter_map(|i| match i {
                Expr::Path { path, .. } => Some(path.dotted()),
                Expr::String(s) => Some(lit_text(s)),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn text(expr: Option<&Expr>) -> String {
    match expr {
        Some(Expr::String(s)) => lit_text(s),
        _ => String::new(),
    }
}

fn check_def(package: &Package, name: &str) -> Option<CheckDef> {
    let body = package
        .files
        .iter()
        .flat_map(|f| f.ast.statements.iter())
        .find_map(|s| match s {
            Statement::Decl(d) => match &d.item {
                Exportable::Named(n) if n.kind.dotted() == "check" && n.name.text == name => {
                    Some(&n.body)
                }
                _ => None,
            },
            _ => None,
        })?;
    let mut rules = BTreeMap::new();
    for m in &body.members {
        if let (true, MemberValue::Section(section)) = (m.name.text == "rules", &m.value) {
            for r in &section.members {
                if let MemberValue::Field(f) = &r.value {
                    rules.insert(r.name.text.clone(), names(Some(&f.value)));
                }
            }
        }
    }
    let purpose = text(field(body, "purpose"));
    let scope = text(field(body, "scope"));
    let run = strings(field(body, "run"));
    let advisory = names(field(body, "advisory"));
    let definition = json!({
        "package": package.id, "name": name, "purpose": purpose, "scope": scope,
        "run": run, "rules": rules, "advisory": advisory,
    });
    Some(CheckDef {
        id: format!("{}.{name}", package.import_name),
        definition_hash: digest(&definition).ok()?,
        purpose,
        scope,
        run,
        rules,
        advisory,
        package_root: package.root.clone(),
    })
}

/// The checks a declaration's kind requires, from its package.
pub fn resolve_checks(project: &Project, decl: &Value) -> Vec<CheckDef> {
    let (Some(body), Some(package)) = (kind_body(project, decl), kind_package(project, decl))
    else {
        return vec![];
    };
    names(field(body, "checks"))
        .iter()
        .filter_map(|n| check_def(package, n.rsplit('.').next().unwrap_or(n)))
        .collect()
}

/// What a run input closure is computed from (SPEC.md, Currency).
pub struct Closure<'a> {
    /// The tree the files are read from: a candidate, or the project for `spec check`.
    pub dir: &'a Path,
    pub project: &'a Project,
    pub graph: &'a Value,
    /// `(member, path)` of a declaration's outputs.
    pub outputs_of: &'a dyn Fn(&str) -> Vec<(String, String)>,
    pub profile: &'a str,
}

/// The digest of a run's input closure: for a declaration-scope run, the outputs of the
/// subject, of everything it reaches over ordering relations (`requires` aside) and of
/// the stack, plus the inputs its kind declares; for a project-scope run, every file git
/// sees except `.spec/` and the plan document. Both add the check definition and the
/// execution profile.
pub fn run_input_digest(
    c: &Closure,
    address: &str,
    definition_hash: &str,
    project_scope: bool,
) -> AResult<String> {
    let decls = declarations(c.graph);
    let mut files: BTreeMap<String, Value> = BTreeMap::new();
    let mut add = |path: &str| -> AResult<()> {
        files.insert(path.to_string(), json!(file_digest(c.dir, path)?));
        Ok(())
    };
    if project_scope {
        for path in crate::workspace::tree_files(c.dir)? {
            add(&path)?;
        }
    } else {
        let mut todo = vec![address.to_string()];
        if decls.contains_key("stack") {
            todo.push("stack".into());
        }
        if let Some(body) = decls.get(address).and_then(|d| kind_body(c.project, d)) {
            for path in strings(field(body, "inputs")) {
                add(&path)?;
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(a) = todo.pop() {
            if !seen.insert(a.clone()) {
                continue;
            }
            for (_, path) in (c.outputs_of)(&a) {
                add(&path)?;
            }
            for e in decls
                .get(a.as_str())
                .and_then(|d| d["edges"].as_array())
                .into_iter()
                .flatten()
            {
                let relation = e["relation"].as_str().unwrap_or_default();
                if ORDERING.contains(&relation) && relation != "requires" {
                    if let Some(t) = e["target"].as_str().filter(|t| decls.contains_key(*t)) {
                        todo.push(t.to_string());
                    }
                }
            }
        }
    }
    Ok(digest(
        &json!({ "files": files, "check": definition_hash, "profile": c.profile }),
    )?)
}

fn strings(expr: Option<&Expr>) -> Vec<String> {
    match expr {
        Some(Expr::List { items, .. }) => items
            .iter()
            .filter_map(|i| match i {
                Expr::String(s) => Some(lit_text(s)),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// A kind's short name, as conformance rules list kinds: `npm:x::page` -> `page`.
pub fn kind_short(kind: &str) -> &str {
    kind.rsplit_once("::")
        .map(|(_, k)| k)
        .or_else(|| kind.strip_prefix("core:"))
        .unwrap_or(kind)
}

/// How one case fared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseOutcome {
    pub oracle: String,
    pub passed: bool,
    pub keys: Vec<String>,
}

/// One check run's outcome.
#[derive(Clone, Debug)]
pub struct CheckRun {
    pub receipt: String,
    pub check: String,
    pub definition_hash: String,
    pub subject: Option<String>,
    /// The test output member, for test runs.
    pub output: Option<String>,
    pub inputs: String,
    pub passed: bool,
    /// `implementation`, `test` or `infrastructure` when it did not pass.
    pub classification: Option<&'static str>,
    pub excerpt: String,
    pub cases: BTreeMap<String, CaseOutcome>,
    pub rules: BTreeMap<String, String>,
    /// The run's full output, for get_diagnostic.
    pub log: String,
}

/// Everything a run needs besides the definition.
pub struct RunContext<'a> {
    pub receipt: String,
    pub workdir: &'a Path,
    pub scratch: &'a Path,
    pub graph_path: &'a Path,
    pub snapshot: &'a str,
    pub profile: &'a str,
    pub subject: Option<&'a str>,
    pub subject_kind: Option<&'a str>,
    /// `(member, path)` of the outputs the check sees.
    pub outputs: Vec<(String, String)>,
    /// The test output member, for test checks.
    pub test_output: Option<(String, String)>,
    /// Expected cases and instance keys (empty keys: any instances).
    pub expected_cases: BTreeMap<String, Vec<String>>,
    pub inputs: String,
    pub timeout: std::time::Duration,
}

fn excerpt(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    let tail = &lines[lines.len().saturating_sub(40)..];
    let mut out = tail.join("\n");
    out.truncate(4000);
    out
}

fn exec(
    argv: &[String],
    cwd: &Path,
    stdin: Option<&str>,
    package_root: &Path,
    timeout: std::time::Duration,
) -> AResult<(i32, Vec<u8>, Vec<u8>)> {
    let f = crate::process::run(
        crate::process::Program::Argv(argv),
        cwd,
        stdin.unwrap_or(""),
        &[(
            "SPEC_PACKAGE_ROOT",
            package_root.to_string_lossy().into_owned(),
        )],
        timeout,
        true,
    )?;
    if f.timed_out {
        let mut err = f.stderr;
        err.extend_from_slice(format!("\ntimed out after {}s", timeout.as_secs()).as_bytes());
        return Ok((124, f.stdout, err));
    }
    Ok((f.code, f.stdout, f.stderr))
}

/// Runs one check. Infrastructure problems are reported in the outcome, never as `Err`.
pub fn run_check(def: &CheckDef, ctx: &RunContext) -> AResult<CheckRun> {
    let mut run = CheckRun {
        receipt: ctx.receipt.clone(),
        check: def.id.clone(),
        definition_hash: def.definition_hash.clone(),
        subject: ctx.subject.map(String::from),
        output: ctx.test_output.as_ref().map(|(m, _)| m.clone()),
        inputs: ctx.inputs.clone(),
        passed: false,
        classification: None,
        excerpt: String::new(),
        cases: BTreeMap::new(),
        rules: BTreeMap::new(),
        log: String::new(),
    };
    // Paths relative to the package root resolve there; plain commands come from PATH.
    let mut argv = def.run.clone();
    if let Some(first) = argv.get_mut(1) {
        if first.starts_with("./") {
            *first = def
                .package_root
                .join(&first[2..])
                .to_string_lossy()
                .into_owned();
        }
    }
    if let Some(first) = argv.first_mut() {
        if first.starts_with("./") {
            *first = def
                .package_root
                .join(&first[2..])
                .to_string_lossy()
                .into_owned();
        }
    }
    if def.purpose == "build" {
        let (code, stdout, stderr) =
            exec(&argv, ctx.workdir, None, &def.package_root, ctx.timeout)?;
        run.log = String::from_utf8_lossy(&[stdout.clone(), stderr.clone()].concat()).into_owned();
        run.passed = code == 0;
        if !run.passed {
            // A missing program is infrastructure; a build that hangs is the code's fault.
            run.classification = Some(if code == 127 {
                "infrastructure"
            } else {
                "implementation"
            });
            run.excerpt = excerpt(&[stdout, stderr].concat());
        }
        return Ok(run);
    }
    let mut input = json!({
        "protocol": "intlang.check/1",
        "check": { "id": def.id, "definition_hash": def.definition_hash },
        "purpose": def.purpose,
        "scope": def.scope,
        "subject": ctx.subject,
        "outputs": ctx.outputs.iter().map(|(m, p)| json!({ "member": m, "path": p })).collect::<Vec<_>>(),
        "graph": ctx.graph_path,
        "snapshot": ctx.snapshot,
        "workdir": ctx.workdir,
        "scratch": ctx.scratch,
        "package_root": def.package_root,
        "profile": ctx.profile,
    });
    let applicable: Vec<String> = def
        .rules
        .iter()
        .filter(|(_, kinds)| {
            ctx.subject_kind
                .is_some_and(|k| kinds.iter().any(|x| x == kind_short(k)))
        })
        .map(|(r, _)| r.clone())
        .collect();
    if def.purpose == "conformance" {
        input["rules"] = json!(applicable);
    } else {
        input["expected_cases"] = Value::Array(
            ctx.expected_cases
                .iter()
                .map(|(c, keys)| json!({ "case": c, "keys": if keys.is_empty() { vec!["default".to_string()] } else { keys.clone() } }))
                .collect(),
        );
        input["inputs"] = json!(ctx.inputs);
        if let Some((member, path)) = &ctx.test_output {
            input["outputs"] = json!([{ "member": member, "path": path }]);
        }
    }
    let (code, stdout, stderr) = exec(
        &argv,
        &def.package_root,
        Some(&input.to_string()),
        &def.package_root,
        ctx.timeout,
    )?;
    run.log = String::from_utf8_lossy(&[stdout.clone(), stderr.clone()].concat()).into_owned();
    let infra = |run: &mut CheckRun, why: String| {
        run.classification = Some("infrastructure");
        run.excerpt = why;
    };
    if code != 0 {
        infra(
            &mut run,
            format!("exit {code}: {}", excerpt(&[stdout, stderr].concat())),
        );
        return Ok(run);
    }
    let result: Value = match serde_json::from_slice(&stdout) {
        Ok(v) => v,
        Err(e) => {
            infra(&mut run, format!("invalid result: {e}"));
            return Ok(run);
        }
    };
    if result["protocol"] != "intlang.check/1"
        || result["check"]["id"] != json!(def.id)
        || result["snapshot"] != json!(ctx.snapshot)
    {
        infra(
            &mut run,
            "result does not echo the check, protocol and snapshot".into(),
        );
        return Ok(run);
    }
    if def.purpose == "conformance" {
        let mut violated = Vec::new();
        for rule in &applicable {
            let status = result["results"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["rule"] == json!(rule) && r["subject"] == json!(ctx.subject))
                .and_then(|r| r["status"].as_str())
                .unwrap_or("error")
                .to_string();
            if status != "satisfied" && !def.advisory.contains(rule) {
                violated.push(format!("{rule}: {status}"));
            }
            run.rules.insert(rule.clone(), status);
        }
        run.passed = violated.is_empty();
        if !run.passed {
            let infra_only = run.rules.values().all(|s| s == "satisfied" || s == "error");
            run.classification = Some(if infra_only {
                "infrastructure"
            } else {
                "implementation"
            });
            run.excerpt = violated.join("; ");
        }
        return Ok(run);
    }
    // Test result: echo, inventory, cases.
    if result["inputs"] != json!(ctx.inputs)
        || result["profile"] != json!(ctx.profile)
        || result["output"] != json!(run.output)
    {
        infra(
            &mut run,
            "test result does not echo its inputs, profile and output".into(),
        );
        return Ok(run);
    }
    let mut pairs = BTreeSet::new();
    let mut duplicate = false;
    let mut problems = Vec::new();
    for case in result["cases"].as_array().into_iter().flatten() {
        let name = case["case"].as_str().unwrap_or_default().to_string();
        let mut keys = Vec::new();
        let mut passed = true;
        for instance in case["instances"].as_array().into_iter().flatten() {
            let key = instance["key"].as_str().unwrap_or_default().to_string();
            duplicate |= !pairs.insert((name.clone(), key.clone()));
            let attempts: Vec<&str> = instance["attempts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            // Under the default policy every attempt must pass: failed -> passed does not.
            passed &= !attempts.is_empty() && attempts.iter().all(|a| *a == "passed");
            keys.push(key);
        }
        passed &= !keys.is_empty();
        run.cases.insert(
            name,
            CaseOutcome {
                oracle: case["oracle"].as_str().unwrap_or_default().to_string(),
                passed,
                keys,
            },
        );
    }
    let inventory: Vec<Value> = pairs.iter().map(|(c, k)| json!([c, k])).collect();
    if result["selection"] == "complete"
        && digest(&Value::Array(inventory))? != result["inventory"].as_str().unwrap_or_default()
    {
        problems.push("inventory digest does not match the reported cases".to_string());
    }
    if duplicate {
        problems.push("duplicate case or instance key".to_string());
    }
    for (case, keys) in &ctx.expected_cases {
        match run.cases.get(case) {
            None => problems.push(format!("expected case {case} is missing")),
            Some(o) if !o.passed => problems.push(format!("case {case} did not pass")),
            Some(o) => {
                for k in keys {
                    if !o.keys.contains(k) {
                        problems.push(format!("case {case} lacks instance {k}"));
                    }
                }
            }
        }
    }
    let untagged_failed = result["untagged"]["failed"].as_u64().unwrap_or(0);
    if untagged_failed > 0 {
        problems.push(format!("{untagged_failed} untagged tests failed"));
    }
    let failed_cases = run
        .cases
        .iter()
        .filter(|(_, o)| !o.passed)
        .map(|(c, _)| c.clone())
        .collect::<Vec<_>>();
    for c in failed_cases {
        if !ctx.expected_cases.contains_key(&c) {
            problems.push(format!("case {c} did not pass"));
        }
    }
    run.passed = problems.is_empty();
    if !run.passed {
        run.classification = Some("test");
        run.excerpt = problems.join("; ");
    }
    Ok(run)
}
