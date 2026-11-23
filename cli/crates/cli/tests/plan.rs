//! `sdc plan`, run as a process on a copy of the worked example.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap()
        .status;
    assert!(status.success(), "git {args:?}");
}

/// The worked example's before tree in a fresh git repository; returns the project dir.
fn project(name: &str, commit: bool) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&base);
    copy_dir(&repo("examples/worked/packages"), &base.join("packages"));
    let root = base.join("project");
    copy_dir(&repo("examples/worked/before"), &root);
    if commit {
        git(&root, &["init", "-q"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "baseline"]);
    }
    root
}

fn spec(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sdc"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn writes_the_plan_and_prints_a_summary() {
    let root = project("plan-cli", true);
    let out = spec(&root, &["plan", "--id", "p-test-01"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Spec changes"), "{stdout}");
    assert!(stdout.contains("Saved to"), "{stdout}");
    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("plan.json")).unwrap()).unwrap();
    assert_eq!(plan["plan"], "p-test-01");
    assert!(plan["binding"]["baseline_commit"]
        .as_str()
        .unwrap()
        .starts_with("git:"));
    // No state yet: every managed declaration is a create.
    let entries = plan["entries"].as_array().unwrap();
    assert!(entries.iter().all(|e| e["change"] == "create"));
}

#[test]
fn json_prints_instead_of_writing() {
    let root = project("plan-cli-json", true);
    let out = spec(&root, &["plan", "--json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(plan["schema"], "sdlang.plan/1");
    assert!(plan["plan"].as_str().unwrap().starts_with("p-"));
    assert!(!root.join("plan.json").exists());
}

#[test]
fn outside_git_is_a_usage_error() {
    let root = project("plan-cli-nogit", false);
    // Stop git from finding this repository above the temporary directory.
    let out = Command::new(env!("CARGO_BIN_EXE_sdc"))
        .current_dir(&root)
        .env("GIT_CEILING_DIRECTORIES", root.parent().unwrap())
        .arg("plan")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("not in a git work tree"));
}
