//! `sdc check` on the toy project apply's tests use: applied end to end, then edited.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sdlang_apply::{apply, ApplyOptions};
use sdlang_gate::{check, CheckOptions, Finding};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../apply/tests/fixture")
        .join(rel)
}

fn copy_dir(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.unwrap();
        let target = dst.join(entry.path().strip_prefix(src).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap();
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The toy project, planned and applied by the scripted agent and reviewer.
fn applied(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("gate")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    copy_dir(&fixture("project"), &root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);
    let project = sdlang_graph::loader::load(&root).unwrap();
    let graph = sdlang_graph::lower::lower_project(&project).unwrap();
    let head = format!("git:{}", git(&root, &["rev-parse", "HEAD"]).trim());
    let plan = sdlang_plan::plan(&sdlang_plan::PlanInputs {
        root: &root,
        project: &project,
        graph: &graph,
        plan_id: "p-test-01",
        baseline_commit: &head,
        acquire: None,
        adopt: None,
    })
    .unwrap();
    fs::write(root.join("plan.json"), plan.to_string()).unwrap();
    let tools = fixture("tools");
    let mut o = ApplyOptions::new(root.clone(), root.join("plan.json"));
    o.agent = Some(format!(
        "AGENT_MODE=good python3 {}",
        tools.join("agent.py").display()
    ));
    o.reviewer = Some(format!("python3 {}", tools.join("reviewer.py").display()));
    let report = apply(&o).unwrap();
    assert!(report.all_published(), "{:?}", report.transactions);
    root
}

fn failing(findings: &[Finding]) -> Vec<(&str, &str)> {
    findings
        .iter()
        .filter(|f| !f.warning)
        .map(|f| (f.kind, f.subject.as_str()))
        .collect()
}

fn snapshot(root: &Path) -> (String, Vec<(String, Vec<u8>)>) {
    let status = git(root, &["status", "--porcelain", "--untracked-files=all"]);
    let mut files = Vec::new();
    for e in walkdir::WalkDir::new(root.join(".spec")).sort_by_file_name() {
        let e = e.unwrap();
        if e.file_type().is_file() {
            files.push((e.path().display().to_string(), fs::read(e.path()).unwrap()));
        }
    }
    (status, files)
}

#[test]
fn an_applied_project_is_consistent_and_check_writes_nothing() {
    let root = applied("clean");
    let before = snapshot(&root);
    let findings = check(&CheckOptions::new(root.clone())).unwrap();
    assert_eq!(
        failing(&findings),
        Vec::<(&str, &str)>::new(),
        "{findings:?}"
    );
    // bye has no requirements: a warning, which fails only under --strict.
    let warned: Vec<_> = findings.iter().filter(|f| f.warning).collect();
    assert_eq!(warned.len(), 1);
    assert_eq!(
        (warned[0].kind, warned[0].subject.as_str()),
        ("unspecified", "bye")
    );
    let mut strict = CheckOptions::new(root.clone());
    strict.strict = true;
    assert_eq!(failing(&check(&strict).unwrap()), [("unspecified", "bye")]);
    assert_eq!(snapshot(&root), before);
}

#[test]
fn a_hand_edit_is_drift_and_stales_the_closure() {
    let root = applied("drift");
    fs::write(root.join("notes/hello.txt"), "note edited by hand\n").unwrap();
    let findings = check(&CheckOptions::new(root.clone())).unwrap();
    let drift: Vec<&Finding> = findings.iter().filter(|f| f.kind == "drift").collect();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].subject, "notes/hello.txt");
    assert!(drift[0]
        .detail
        .starts_with("hello#outputs.main: expected sha256:"));
    assert!(drift[0].detail.contains("actual sha256:"));
    // hello's test run read notes/hello.txt, so its evidence is stale.
    assert!(findings.iter().any(|f| f.kind == "evidence"
        && f.subject == "hello#expect.greets"
        && f.detail.starts_with("stale: run inputs changed")));
    // --allow drift still fails on the stale evidence.
    let mut allow = CheckOptions::new(root.clone());
    allow.allow = vec!["drift".into()];
    assert_eq!(
        failing(&check(&allow).unwrap()),
        [("evidence", "hello#expect.greets")]
    );
    // An unrelated file stales nothing.
    let root = applied("unrelated");
    fs::write(root.join("README.txt"), "notes\n").unwrap();
    assert_eq!(
        failing(&check(&CheckOptions::new(root)).unwrap()),
        Vec::<(&str, &str)>::new()
    );
}

#[test]
fn a_spec_edit_is_unapplied_and_its_evidence_is_not_reported() {
    let root = applied("unapplied");
    let app = root.join("spec/app.sd");
    let text = fs::read_to_string(&app).unwrap();
    fs::write(&app, text.replace("says hello", "says hello twice")).unwrap();
    let findings = check(&CheckOptions::new(root)).unwrap();
    assert_eq!(failing(&findings), [("unapplied", "hello")], "{findings:?}");
    assert_eq!(findings[0].detail, "update (own_projection)");
}

#[test]
fn ownership_and_reconcile() {
    let root = applied("ownership");
    // A second record claiming hello's note.
    let state = root.join(".spec/state");
    let text = fs::read_to_string(state.join("bye.json")).unwrap();
    let mut bye: serde_json::Value = serde_json::from_str(&text).unwrap();
    bye["outputs"]["main"]["path"] = "notes/hello.txt".into();
    fs::write(state.join("bye.json"), bye.to_string()).unwrap();
    fs::create_dir_all(root.join(".spec/reconcile")).unwrap();
    let findings = check(&CheckOptions::new(root)).unwrap();
    let kinds: Vec<(&str, &str)> = failing(&findings)
        .into_iter()
        .filter(|(k, _)| *k == "ownership" || *k == "reconcile")
        .collect();
    assert_eq!(
        kinds,
        [
            ("ownership", "notes/hello.txt"),
            ("reconcile", ".spec/reconcile")
        ]
    );
}

#[test]
fn rerun_reproduces_recorded_runs() {
    let root = applied("rerun");
    let before = snapshot(&root);
    let mut o = CheckOptions::new(root.clone());
    o.rerun = true;
    assert_eq!(failing(&check(&o).unwrap()), Vec::<(&str, &str)>::new());
    // The marker makes the toy test check fail every case on this machine.
    fs::write(root.join("spec-packages/toy/FAIL"), "").unwrap();
    let findings = check(&o).unwrap();
    assert_eq!(failing(&findings), [("evidence", "hello#expect.greets")]);
    assert!(findings
        .iter()
        .any(|f| f.detail == "rerun: case greets of hello#outputs.tests now fails"));
    fs::remove_file(root.join("spec-packages/toy/FAIL")).unwrap();
    assert_eq!(snapshot(&root), before);
    assert!(!root.join(".spec/run/candidates/check-rerun").exists());
}
