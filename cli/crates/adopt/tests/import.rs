//! `spec import` on a small existing app: propose, accept, plan, apply the acquisition,
//! then discharge the obligations the acquisition leaves.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use intlang_adopt::{accept, propose};
use intlang_apply::{apply, ApplyOptions, Outcome};
use intlang_plan::state::Store;
use serde_json::{json, Value};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/project")
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

fn project(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adopt")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    copy_dir(&fixture(), &root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);
    root
}

fn plan(root: &Path, id: &str) -> Value {
    let project = intlang_graph::loader::load(root).unwrap();
    let graph = intlang_graph::lower::lower_project(&project).unwrap();
    let head = format!("git:{}", git(root, &["rev-parse", "HEAD"]).trim());
    let pending = intlang_plan::pending_imports(root).unwrap();
    let plan = intlang_plan::plan(&intlang_plan::PlanInputs {
        root,
        project: &project,
        graph: &graph,
        plan_id: id,
        baseline_commit: &head,
        acquire: Some(&pending),
    })
    .unwrap();
    fs::write(root.join("plan.json"), plan.to_string()).unwrap();
    plan
}

fn entry<'p>(plan: &'p Value, address: &str) -> &'p Value {
    plan["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["address"] == address)
        .unwrap_or_else(|| panic!("no entry for {address}: {plan}"))
}

fn strings(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

#[test]
fn propose_accept_acquire_then_discharge() {
    let root = project("worked");
    let p = propose(&root, &["src".into(), "tests".into()]).unwrap();

    // The walk-through's lib: one integration owner for a file two pages use.
    assert_eq!(p.new_libs.len(), 1);
    let lib = &p.new_libs[0];
    assert_eq!(lib.address(), "auth.auth_lib");
    assert!(lib.with_path);
    assert_eq!(lib.exports, ["signIn", "requestReset"]);
    assert_eq!(
        lib.importers,
        ["src/app/forgot-password/page.tsx", "src/app/login/page.tsx"]
    );
    assert!(lib.integration());
    assert_eq!(
        p.grants,
        [
            (
                "auth.forgot_password".to_string(),
                "auth.auth_lib".to_string()
            ),
            ("auth.login".to_string(), "auth.auth_lib".to_string())
        ]
    );
    // Existing declarations: slots, a created output keyed by its stem, test bindings.
    let login = &p.claims["auth.login"];
    assert_eq!(login.slots.keys().collect::<Vec<_>>(), ["page", "tests"]);
    assert_eq!(
        login.created["login_form"],
        (
            "parts".to_string(),
            "LoginForm.tsx".to_string(),
            "src/app/login/_parts/LoginForm.tsx".to_string()
        )
    );
    assert_eq!(
        login.tests["auth.login#expect.shows_form"],
        [(
            "auth.login#outputs.tests".to_string(),
            "shows_form".to_string()
        )]
    );
    assert!(p.claims.contains_key("auth.forgot_password"));
    assert_eq!(p.unmanaged, ["src/app/login/page.css"]);
    assert_eq!(
        p.unresolved
            .iter()
            .map(|(f, t, _)| (f.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
        [("src/app/forgot-password/page.tsx", "vendor/strings.js")]
    );
    assert!(p.collisions.is_empty() && p.owned.is_empty());
    let text = intlang_adopt::proposal::render(&p);
    assert!(
        text.contains("proposal: one integration owner (a file serving 2 would-be declarations)"),
        "{text}"
    );
    assert!(text.contains("accept with: spec import src tests --accept  (writes spec/auth/auth_lib.int and .spec/import.json)"), "{text}");

    // Accept writes the lib and the pending acquisitions, and nothing else.
    let accepted = accept(&root, &p).unwrap();
    assert_eq!(accepted.written, ["spec/auth/auth_lib.int"]);
    assert_eq!(
        accepted.pending,
        ["auth.auth_lib", "auth.forgot_password", "auth.login"]
    );
    let lib_src = fs::read_to_string(root.join("spec/auth/auth_lib.int")).unwrap();
    assert!(
        lib_src.contains("path: \"src/lib/auth.ts\"")
            && lib_src.contains("exports: [\"signIn\", \"requestReset\"]")
    );
    let status = git(&root, &["status", "--porcelain", "--untracked-files=all"]);
    assert_eq!(status, "?? .spec/import.json\n?? spec/auth/auth_lib.int\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "accept import"]);

    // The plan acquires all three; nothing is a coverage subject.
    let first = plan(&root, "p-import-01");
    for a in ["auth.auth_lib", "auth.forgot_password", "auth.login"] {
        let e = entry(&first, a);
        assert_eq!(
            (e["change"].as_str(), e["strategy"].as_str()),
            (Some("create"), Some("acquire")),
            "{a}"
        );
    }
    assert_eq!(
        entry(&first, "auth.login")["acquire"]["created"]["login_form"]["relative_path"],
        "LoginForm.tsx"
    );
    assert_eq!(first["reviews"]["coverage"], serde_json::json!([]));
    assert!(first["binding"]["fingerprints"][".spec/import.json"].is_string());

    // Apply runs no agent and no reviewer, and changes no code.
    let before_code = git(&root, &["rev-parse", "HEAD:src"]);
    let report = apply(&ApplyOptions::new(root.clone(), root.join("plan.json"))).unwrap();
    assert!(report.all_published(), "{:?}", report.transactions);
    assert_eq!(git(&root, &["rev-parse", "HEAD:src"]), before_code);
    assert!(!root.join(".spec/import.json").exists());
    let records = Store::new(&root).records().unwrap();
    let login = &records["auth.login"];
    assert_eq!(
        login.outputs.keys().collect::<Vec<_>>(),
        ["login_form", "page", "tests"]
    );
    assert_eq!(login.outputs["login_form"].origin, "created");
    assert_eq!(login.evidence[0]["summary"], "unverified");
    assert_eq!(
        login.evidence[0]["coverage"]["tests"][0]["case"],
        "shows_form"
    );
    assert_eq!(
        records["auth.auth_lib"].outputs.keys().collect::<Vec<_>>(),
        ["main"]
    );

    // The next plan: the lib's empty tests slot is an update, and the pages' unverified
    // requirements are verify and review obligations.
    let second = plan(&root, "p-import-02");
    let lib = entry(&second, "auth.auth_lib");
    assert_eq!(
        (lib["change"].as_str(), lib["causes"][0].as_str()),
        (Some("update"), Some("convention"))
    );
    let login = entry(&second, "auth.login");
    assert_eq!(login["change"], "none");
    assert_eq!(
        login["obligations"],
        serde_json::json!([
            { "kind": "review", "because": "unverified:auth.login#expect.shows_form" },
            { "kind": "verify", "because": "unverified:auth.login#expect.shows_form" }
        ])
    );
    // Without an agent the lib's update fails; the obligation-only transaction still
    // runs the bound tests and a review, and the pages' evidence becomes tested.
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../apply/tests/fixture/tools");
    let mut o = ApplyOptions::new(root.clone(), root.join("plan.json"));
    o.reviewer = Some(format!("python3 {}", tools.join("reviewer.py").display()));
    let report = apply(&o).unwrap();
    let outcomes: Vec<(&str, bool)> = report
        .transactions
        .iter()
        .map(|(id, o)| (id.as_str(), matches!(o, Outcome::Published { .. })))
        .collect();
    assert_eq!(
        outcomes,
        [("tx-1", false), ("tx-2", true)],
        "{:?}",
        report.transactions
    );
    let records = Store::new(&root).records().unwrap();
    for a in ["auth.login", "auth.forgot_password"] {
        assert_eq!(
            records[a].evidence[0]["summary"], "tested",
            "{a}: {}",
            records[a].evidence[0]
        );
    }
    let third = plan(&root, "p-import-03");
    let left: Vec<&str> = third["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["address"].as_str())
        .collect();
    assert_eq!(left, ["auth.auth_lib"]);
}

#[test]
fn collisions_block_acceptance() {
    let root = project("collision");
    // A page whose slot lies inside login's creation root.
    let pages = root.join("spec/auth/pages.int");
    let mut text = fs::read_to_string(&pages).unwrap();
    text.push_str("\nexport web.page login_parts {\n  route: \"/login/_parts\"\n  summary: \"Shadows login's parts\"\n}\n");
    fs::write(&pages, text).unwrap();
    fs::write(
        root.join("src/app/login/_parts/page.tsx"),
        "export default 1;\n",
    )
    .unwrap();
    let p = propose(&root, &["src/app/login".into()]).unwrap();
    assert_eq!(p.collisions.len(), 1, "{:?}", p.collisions);
    assert_eq!(p.collisions[0].0, "src/app/login/_parts/page.tsx");
    assert!(p.collisions[0]
        .1
        .contains("auth.login (creation root parts)"));
    assert!(p.collisions[0].1.contains("auth.login_parts#outputs.page"));
    let err = accept(&root, &p).unwrap_err();
    assert!(err.message.contains("collision"));
    assert!(!root.join(".spec/import.json").exists());
    // Only what it was pointed at: tests/ was not, so login's test file is not claimed.
    assert!(p
        .unclaimed
        .iter()
        .any(|(a, s, _)| a == "auth.login" && s == "tests"));
}

#[test]
fn owned_files_are_not_claimed_again() {
    let root = project("owned");
    let p = propose(
        &root,
        &["src/app/login/page.tsx".into(), "tests/app/login".into()],
    )
    .unwrap();
    accept(&root, &p).unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "accept"]);
    // forgot_password was not pointed at, so it is an ordinary create needing an agent;
    // login's acquisition publishes on its own.
    let planned = plan(&root, "p-owned-01");
    let tx = entry(&planned, "auth.login")["transaction"].clone();
    let report = apply(&ApplyOptions::new(root.clone(), root.join("plan.json"))).unwrap();
    let login = report
        .transactions
        .iter()
        .find(|(id, _)| json!(id) == tx)
        .unwrap();
    assert!(
        matches!(login.1, Outcome::Published { .. }),
        "{:?}",
        login.1
    );
    let again = propose(&root, &["src/app/login/page.tsx".into()]).unwrap();
    assert_eq!(
        strings(&again.owned),
        [("src/app/login/page.tsx", "auth.login")]
    );
    assert!(again.is_empty());
}
