//! `apply` end to end on a toy project: a local package whose agent, reviewer, generator
//! and checks are Python scripts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use intlang_apply::{apply, ApplyOptions, Outcome};
use serde_json::Value;

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixture")
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

/// The toy project in a fresh repository, planned; returns its root.
fn setup(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("apply")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    copy_dir(&fixture("project"), &root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);
    write_plan(&root);
    root
}

fn write_plan(root: &Path) -> Value {
    let project = intlang_graph::loader::load(root).unwrap();
    let graph = intlang_graph::lower::lower_project(&project).unwrap();
    let head = format!("git:{}", git(root, &["rev-parse", "HEAD"]).trim());
    let plan = intlang_plan::plan(&intlang_plan::PlanInputs {
        root,
        project: &project,
        graph: &graph,
        plan_id: "p-test-01",
        baseline_commit: &head,
    })
    .unwrap();
    fs::write(
        root.join("plan.json"),
        serde_json::to_string_pretty(&plan).unwrap(),
    )
    .unwrap();
    plan
}

fn options(root: &Path, agent_env: &str, review_env: &str) -> ApplyOptions {
    let tools = fixture("tools");
    let mut o = ApplyOptions::new(root.to_path_buf(), root.join("plan.json"));
    o.agent = Some(format!(
        "{agent_env} python3 {}",
        tools.join("agent.py").display()
    ));
    o.reviewer = Some(format!(
        "{review_env} python3 {}",
        tools.join("reviewer.py").display()
    ));
    o.agent_timeout = std::time::Duration::from_secs(60);
    o.check_timeout = std::time::Duration::from_secs(60);
    o
}

fn outcomes(report: &intlang_apply::ApplyReport) -> Vec<(String, &'static str)> {
    report
        .transactions
        .iter()
        .map(|(id, o)| {
            let kind = match o {
                Outcome::Published { .. } => "published",
                Outcome::AlreadyPublished { .. } => "already",
                Outcome::Failed { .. } => "failed",
                Outcome::Skipped { .. } => "skipped",
            };
            (id.clone(), kind)
        })
        .collect()
}

fn ids(v: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    v.iter().map(|(a, b)| (a.to_string(), *b)).collect()
}

#[test]
fn end_to_end() {
    let root = setup("end-to-end");
    let plan: Value =
        serde_json::from_str(&fs::read_to_string(root.join("plan.json")).unwrap()).unwrap();
    let order: Vec<&str> = plan["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["entries"][0].as_str().unwrap())
        .collect();
    assert_eq!(order, ["Item", "hello", "bye"]);

    let report = apply(&options(&root, "AGENT_MODE=good", "")).unwrap();
    assert_eq!(
        outcomes(&report),
        ids(&[
            ("tx-1", "published"),
            ("tx-2", "published"),
            ("tx-3", "published")
        ]),
        "{report:?}"
    );
    // One commit per transaction, each with its trailer.
    let log = git(&root, &["log", "--format=%B%x00"]);
    assert_eq!(
        log.matches("Spec-Transaction: p-test-01/r1/").count(),
        3,
        "{log}"
    );
    assert_eq!(
        fs::read_to_string(root.join("notes/hello.txt")).unwrap(),
        "note hello\n"
    );
    assert!(root.join("schema/Item.txt").exists());
    let hello: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".spec/state/hello.json")).unwrap())
            .unwrap();
    assert_eq!(hello["evidence"][0]["summary"], "tested");
    assert_eq!(
        hello["evidence"][0]["coverage"]["tests"][0]["case"],
        "greets"
    );
    assert_eq!(hello["provenance"]["transaction"], "tx-2");
    let journal = fs::read_to_string(root.join(".spec/journal/p-test-01.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 3);
    // The work tree is clean apart from plan.json, and a new plan has nothing to do.
    let status = git(&root, &["status", "--porcelain"]);
    assert_eq!(status.trim(), "?? plan.json", "{status}");
    let again = write_plan(&root);
    assert_eq!(
        again["entries"],
        serde_json::json!([]),
        "{}",
        serde_json::to_string_pretty(&again).unwrap()
    );
    assert_eq!(again["blocked"], serde_json::json!([]));
    // Orders and results are kept for audit.
    assert!(root
        .join(".spec/run/p-test-01/r1/tx-2/1/order.json")
        .exists());
    assert!(root
        .join(".spec/run/p-test-01/r1/tx-2/1/result.json")
        .exists());
}

#[test]
fn writes_outside_the_envelope_fail_and_dependents_skip_then_resume() {
    let root = setup("envelope");
    let report = apply(&options(&root, "AGENT_MODE=escape AGENT_TARGET=hello", "")).unwrap();
    assert_eq!(
        outcomes(&report),
        ids(&[
            ("tx-1", "published"),
            ("tx-2", "failed"),
            ("tx-3", "skipped")
        ])
    );
    let Outcome::Failed { reason } = &report.transactions[1].1 else {
        panic!()
    };
    assert!(
        reason.contains("rogue.txt") && reason.contains("attempt 3 of 3"),
        "{reason}"
    );
    assert!(!root.join("rogue.txt").exists());
    assert!(!root.join("notes/hello.txt").exists());
    assert!(root
        .join(".spec/run/p-test-01/r1/tx-2/failed.patch")
        .exists());
    // Resuming skips the published transaction by the journal.
    let report = apply(&options(&root, "AGENT_MODE=good", "")).unwrap();
    assert_eq!(
        outcomes(&report),
        ids(&[
            ("tx-1", "already"),
            ("tx-2", "published"),
            ("tx-3", "published")
        ])
    );
}

#[test]
fn a_failing_check_is_retried_with_the_failure_inline() {
    let root = setup("retry");
    let log = root.parent().unwrap().join("retry-agent.log");
    let _ = fs::remove_file(&log);
    let report = apply(&options(
        &root,
        &format!("AGENT_MODE=fail_first AGENT_LOG={}", log.display()),
        "",
    ))
    .unwrap();
    assert!(report.all_published(), "{report:?}");
    let lines: Vec<Value> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let hello: Vec<&Value> = lines
        .iter()
        .filter(|l| l["dispatch"].as_str().unwrap().contains("/tx-2/"))
        .collect();
    assert_eq!(hello.len(), 2);
    assert_eq!(hello[1]["attempt"], 2);
    assert_eq!(hello[1]["previous"], 1);
}

#[test]
fn nonconforming_review_fails_the_transaction() {
    let root = setup("nonconforming");
    let report = apply(&options(
        &root,
        "AGENT_MODE=good",
        "REVIEW_VERDICT=does_not_conform",
    ))
    .unwrap();
    assert_eq!(outcomes(&report)[1], ("tx-2".to_string(), "failed"));
    let Outcome::Failed { reason } = &report.transactions[1].1 else {
        panic!()
    };
    assert!(reason.contains("hello#expect.greets"), "{reason}");
}

#[test]
fn a_stale_binding_refuses_to_start() {
    let root = setup("binding");
    fs::write(root.join("README.md"), "moved on\n").unwrap();
    git(&root, &["add", "README.md"]);
    git(&root, &["commit", "-q", "-m", "unrelated"]);
    let err = apply(&options(&root, "AGENT_MODE=good", "")).unwrap_err();
    assert!(err.message.contains("plan binding mismatch: HEAD"), "{err}");
}

#[test]
fn an_interface_change_updates_its_consumer_in_the_same_transaction() {
    let root = setup("update");
    assert!(apply(&options(&root, "AGENT_MODE=good", ""))
        .unwrap()
        .all_published());
    let app = root.join("spec/app.int");
    let text = fs::read_to_string(&app)
        .unwrap()
        .replace("title: \"Hello\"", "title: \"Hello there\"");
    fs::write(&app, text).unwrap();
    git(&root, &["add", "spec/app.int"]);
    git(&root, &["commit", "-q", "-m", "retitle hello"]);
    fs::remove_dir_all(root.join(".spec/journal")).unwrap();
    let plan = write_plan(&root);
    let entries: Vec<(&str, &str)> = plan["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["address"].as_str().unwrap(),
                e["change"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(entries, [("hello", "update"), ("bye", "update")]);
    assert_eq!(plan["transactions"].as_array().unwrap().len(), 1);
    let report = apply(&options(&root, "AGENT_MODE=good", "")).unwrap();
    assert!(report.all_published(), "{report:?}");
    let order: Value = serde_json::from_str(
        &fs::read_to_string(root.join(".spec/run/p-test-01/r1/tx-1/1/order.json")).unwrap(),
    )
    .unwrap();
    let subject = &order["subjects"][0];
    assert_eq!(subject["change"], "update");
    assert!(subject["spec"]["previous"]
        .as_str()
        .unwrap()
        .contains("title: \"Hello\""));
    assert!(subject["spec"]["diff"]
        .as_str()
        .unwrap()
        .contains("+  title: \"Hello there\""));
    assert!(root.join(".spec/run/p-test-01/r1/tx-1/1/order.md").exists());
    assert_eq!(write_plan(&root)["entries"], serde_json::json!([]));
}

#[test]
fn an_agent_past_its_time_limit_is_stopped() {
    let root = setup("timeout");
    let mut o = options(&root, "AGENT_MODE=good", "");
    o.agent = Some("sleep 30".into());
    o.agent_timeout = std::time::Duration::from_secs(1);
    o.max_attempts = 1;
    let start = std::time::Instant::now();
    let report = apply(&o).unwrap();
    assert!(start.elapsed() < std::time::Duration::from_secs(20));
    let Outcome::Failed { reason } = &report.transactions[1].1 else {
        panic!("{report:?}")
    };
    assert!(reason.contains("did not finish within 1s"), "{reason}");
}

#[test]
fn an_invalid_verdict_is_asked_again_then_fails_only_its_transaction() {
    let root = setup("verdict-once");
    let mark = root.parent().unwrap().join("verdict-once.mark");
    let _ = fs::remove_file(&mark);
    let report = apply(&options(
        &root,
        "AGENT_MODE=good",
        &format!("REVIEW_BAD=once REVIEW_MARK={}", mark.display()),
    ))
    .unwrap();
    assert!(report.all_published(), "{report:?}");
    let md = fs::read_to_string(root.join(".spec/run/p-test-01/r1/tx-2/3/order.md")).unwrap();
    assert!(
        md.contains("Your previous verdict was rejected") && md.contains("expected \"s1\""),
        "{md}"
    );

    let root = setup("verdict-always");
    let report = apply(&options(&root, "AGENT_MODE=good", "REVIEW_BAD=always")).unwrap();
    assert_eq!(
        outcomes(&report),
        ids(&[
            ("tx-1", "published"),
            ("tx-2", "failed"),
            ("tx-3", "skipped")
        ])
    );
    let Outcome::Failed { reason } = &report.transactions[1].1 else {
        panic!()
    };
    assert!(reason.contains("invalid twice"), "{reason}");
}
