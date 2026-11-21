//! `spec plan` then `spec apply`, run as processes on the apply crate's toy project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
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
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

fn spec(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spec"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn plan_then_apply() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-apply");
    let _ = fs::remove_dir_all(&root);
    copy_dir(&fixture("project"), &root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);
    let out = spec(&root, &["plan", "--id", "p-cli-01"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let agent = format!("python3 {}", fixture("tools/agent.py").display());
    let reviewer = format!("python3 {}", fixture("tools/reviewer.py").display());
    let out = spec(
        &root,
        &["apply", "--agent", &agent, "--reviewer", &reviewer],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout.matches("published").count(), 3, "{stdout}");
    // Applying the same plan again refuses: its binding no longer holds without a journal.
    fs::remove_dir_all(root.join(".spec/journal")).unwrap();
    let out = spec(
        &root,
        &["apply", "--agent", &agent, "--reviewer", &reviewer],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("binding mismatch"));
}
