//! The planner against the worked example: a baseline recorded from
//! examples/worked/before, then the after tree planned against it.

use std::fs;
use std::path::{Path, PathBuf};

use intlang_plan::state::{record_baseline, Store};
use intlang_plan::{materialise::materialise, plan, PlanInputs};
use serde_json::{json, Value};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(rel)
}

fn copy_dir(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src).follow_links(true) {
        let entry = entry.unwrap();
        let target = dst.join(entry.path().strip_prefix(src).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap();
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A project directory holding examples/worked/before's spec, with every managed
/// declaration's slots written to disk and recorded as the baseline. Returns its path.
fn baseline(name: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&base);
    copy_dir(&repo("examples/worked/packages"), &base.join("packages"));
    let root = base.join("project");
    copy_dir(&repo("examples/worked/before"), &root);
    let project = intlang_graph::loader::load(&root).unwrap();
    let graph =
        intlang_graph::lower::lower_project(&project).unwrap_or_else(|e| panic!("{}", e.render()));
    let mats = materialise(&project, &graph).unwrap();
    for (address, m) in &mats {
        for (slot, path) in &m.slots {
            let full = root.join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, format!("// {address} {slot}\n")).unwrap();
        }
    }
    let docs = intlang_canon::documents(&graph).unwrap();
    record_baseline(&Store::new(&root), &root, &graph, &docs, &mats, None).unwrap();
    root
}

fn replace_spec(root: &Path, from: &str) {
    fs::remove_dir_all(root.join("spec")).unwrap();
    copy_dir(&repo(from).join("spec"), &root.join("spec"));
}

fn run(root: &Path) -> Value {
    let project = intlang_graph::loader::load(root).unwrap();
    let graph =
        intlang_graph::lower::lower_project(&project).unwrap_or_else(|e| panic!("{}", e.render()));
    plan(&PlanInputs {
        root,
        project: &project,
        graph: &graph,
        plan_id: "p-2026-11-18-01",
        baseline_commit: "git:2fb0bc6f21fcbd443db01e91e32dc8b13f8e767c",
    })
    .unwrap()
}

#[test]
fn worked_plan() {
    let root = baseline("worked-plan");
    replace_spec(&root, "examples/worked/after");
    let signup = root.join("src/app/signup/page.tsx");
    let mut text = fs::read_to_string(&signup).unwrap();
    text.push_str("// edited by hand\n");
    fs::write(&signup, text).unwrap();

    let got = run(&root);
    let want: Value =
        serde_json::from_str(&fs::read_to_string(repo("examples/plan.json")).unwrap()).unwrap();
    assert_eq!(
        got["entries"],
        want["entries"],
        "entries:\n{}",
        serde_json::to_string_pretty(&got["entries"]).unwrap()
    );
    assert_eq!(
        got["transactions"],
        want["transactions"],
        "transactions:\n{}",
        serde_json::to_string_pretty(&got["transactions"]).unwrap()
    );
    assert_eq!(got["reviews"], want["reviews"]);
    // The fixture's digests are labels, not hashes of these files; the rest must match.
    let strip = |b: &Value| {
        b.as_array()
            .unwrap()
            .iter()
            .map(|x| json!({ "address": x["address"], "reason": x["reason"], "path": x["path"], "resolutions": x["resolutions"] }))
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(&got["blocked"]), strip(&want["blocked"]));
    for path in want["binding"]["fingerprints"].as_object().unwrap().keys() {
        assert!(
            got["binding"]["fingerprints"].get(path).is_some(),
            "no fingerprint for {path}"
        );
    }
    let drifted = &got["blocked"][0];
    assert_ne!(drifted["expected"], drifted["actual"]);
    fs::write(
        root.join("plan.json"),
        serde_json::to_string_pretty(&got).unwrap(),
    )
    .unwrap();
}

#[test]
fn module_prompt_change_is_review_only() {
    let root = baseline("module-prompt");
    let module = root.join("spec/auth/module.int");
    let text = fs::read_to_string(&module).unwrap();
    let edited = text.replace(
        "All auth screens use one centered card layout.",
        "All auth screens use a split layout.",
    );
    assert_ne!(text, edited);
    fs::write(&module, edited).unwrap();
    let got = run(&root);
    let entries = got["entries"].as_array().unwrap();
    let addresses: Vec<&str> = entries
        .iter()
        .map(|e| e["address"].as_str().unwrap())
        .collect();
    assert_eq!(addresses, ["auth.login", "auth.sign_in", "auth.signup"]);
    let context = json!([
        { "kind": "review", "because": "context:module:auth" },
        { "kind": "verify", "because": "context:module:auth" }
    ]);
    for e in entries {
        assert_eq!(e["change"], "none");
        let obligations = e["obligations"].as_array().unwrap();
        assert_eq!(
            obligations[..2],
            context.as_array().unwrap()[..],
            "{}",
            e["address"]
        );
    }
    // login calls sign_in, whose inherited context changed: observed, so also reviewed.
    assert_eq!(
        entries[0]["obligations"].as_array().unwrap()[2..],
        json!([
            { "kind": "review", "because": "observed:auth.sign_in" },
            { "kind": "verify", "because": "observed:auth.sign_in" }
        ])
        .as_array()
        .unwrap()[..]
    );
    assert_eq!(got["transactions"].as_array().unwrap().len(), 1);
}

#[test]
fn unchanged_tree_plans_nothing() {
    let root = baseline("unchanged");
    let got = run(&root);
    assert_eq!(got["entries"], json!([]));
    assert_eq!(got["transactions"], json!([]));
    assert_eq!(got["blocked"], json!([]));
}

#[test]
fn greenfield_is_all_creates() {
    let root = baseline("greenfield");
    fs::remove_dir_all(root.join(".spec")).unwrap();
    let got = run(&root);
    let entries = got["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    assert!(entries
        .iter()
        .all(|e| e["change"] == "create" && e["identity"] == "new"));
    assert_eq!(got["blocked"], json!([]));
}

#[test]
fn guidance_only_change_plans_nothing() {
    let root = baseline("guidance");
    let login = root.join("spec/auth/login.int");
    let text = fs::read_to_string(&login).unwrap();
    let edited = text.replace(
        "Use the library's email and password flow rather than custom hashing.",
        "Use the library's flow.",
    );
    assert_ne!(text, edited);
    fs::write(&login, edited).unwrap();
    assert_eq!(run(&root)["entries"], json!([]));
}

#[test]
fn state_round_trips() {
    let root = baseline("round-trip");
    let store = Store::new(&root);
    let records = store.records().unwrap();
    assert!(records.contains_key("auth.login"));
    let login = &records["auth.login"];
    store.write_record(login).unwrap();
    assert_eq!(&store.records().unwrap()["auth.login"], login);
    for c in &login.consumed {
        assert!(
            store.applied(&c.hash).unwrap().is_some(),
            "{} {} not stored",
            c.target,
            c.projection
        );
    }
    for hash in login.context_parts.values() {
        assert!(store.applied(hash).unwrap().is_some());
    }
}

fn edit(root: &Path, file: &str, from: &str, to: &str) {
    let path = root.join(file);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{file} lacks {from:?}");
    fs::write(&path, text.replace(from, to)).unwrap();
}

fn summary(plan: &Value) -> Vec<(String, String, String)> {
    plan["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["address"].as_str().unwrap().to_string(),
                e["change"].as_str().unwrap().to_string(),
                e["transaction"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn s(a: &str, c: &str, t: &str) -> (String, String, String) {
    (a.into(), c.into(), t.into())
}

#[test]
fn incompatible_consumers_join_the_transaction() {
    let root = baseline("incompatible");
    edit(
        &root,
        "spec/auth/login.int",
        "output: authentication.session",
        "output: { ok: boolean }",
    );
    let got = run(&root);
    assert_eq!(
        summary(&got),
        [
            s("auth.sign_in", "update", "tx-1"),
            s("auth.login", "update", "tx-1")
        ]
    );
    assert_eq!(
        got["entries"][1]["causes"],
        json!(["consumed_incompatible"])
    );
}

#[test]
fn observed_behaviour_gives_verify_and_review() {
    let root = baseline("observed");
    edit(
        &root,
        "spec/auth/login.int",
        "rate_limited: \"Too many attempts; try again later\"",
        "rate_limited: \"Slow down\"",
    );
    let got = run(&root);
    assert_eq!(
        summary(&got),
        [
            s("auth.sign_in", "update", "tx-1"),
            s("auth.login", "none", "tx-1")
        ]
    );
    assert_eq!(
        got["entries"][1]["obligations"],
        json!([
            { "kind": "review", "because": "observed:auth.sign_in" },
            { "kind": "verify", "because": "observed:auth.sign_in" }
        ])
    );
    assert_eq!(
        got["transactions"][0]["obligations"]["auth.login"]["kinds"],
        json!(["review", "verify"])
    );
    assert_eq!(
        got["transactions"][0]["obligations"]["auth.login"]["contingent"],
        json!(["src/app/login/page.tsx", "tests/app/login/page.test.ts"])
    );
}

#[test]
fn removed_declaration_is_deleted() {
    let root = baseline("delete");
    fs::remove_file(root.join("spec/auth/signup.int")).unwrap();
    let got = run(&root);
    assert_eq!(summary(&got), [s("auth.signup", "delete", "tx-1")]);
    assert_eq!(got["entries"][0]["identity"], "gone");
}

#[test]
fn moved_declaration_keeps_its_record() {
    let root = baseline("moved");
    edit(
        &root,
        "spec/auth/signup.int",
        "export nextjs.page signup {",
        "moved auth.signup -> auth.register\n\nexport nextjs.page register {",
    );
    let got = run(&root);
    assert_eq!(summary(&got), [s("auth.register", "none", "tx-1")]);
    assert_eq!(got["entries"][0]["identity"], "moved");
    assert_eq!(got["entries"][0]["moved_from"], "auth.signup");
}

#[test]
fn frozen_declaration_blocks_its_update() {
    let root = baseline("frozen");
    edit(
        &root,
        "spec/auth/signup.int",
        "summary: \"Create an account\"",
        "summary: \"Create an account\"\n  lifecycle: \"frozen\"",
    );
    let got = run(&root);
    assert_eq!(got["entries"], json!([]));
    assert_eq!(
        got["blocked"],
        json!([{ "address": "auth.signup", "reason": "frozen" }])
    );
}

#[test]
fn empty_slot_schedules_an_update() {
    let root = baseline("empty-slot");
    let store = Store::new(&root);
    let mut login = store.records().unwrap().remove("auth.login").unwrap();
    login.outputs.remove("tests");
    store.write_record(&login).unwrap();
    let got = run(&root);
    assert_eq!(summary(&got), [s("auth.login", "update", "tx-1")]);
    assert_eq!(got["entries"][0]["causes"], json!(["convention"]));
}
