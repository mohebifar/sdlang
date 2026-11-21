//! `spec import`, then `spec plan`, `spec apply` and `spec check`, as processes on the adopt
//! crate's small existing app.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn spec(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spec"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn import_then_plan_apply_and_check() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-import");
    let _ = fs::remove_dir_all(&root);
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../adopt/tests/fixture/project"),
        &root,
    );
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);

    // A proposal writes nothing.
    let out = spec(&root, &["import", "src", "tests"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let shown = text(&out);
    assert!(shown.contains("+ lib  auth.auth_lib   outputs.main = src/lib/auth.ts   exports signIn, requestReset   strategy: acquire"), "{shown}");
    assert!(
        shown.contains("~ auth.forgot_password, auth.login   need uses: [auth_lib]"),
        "{shown}"
    );
    assert_eq!(git(&root, &["status", "--porcelain"]), "");
    assert_eq!(spec(&root, &["import", "nowhere"]).status.code(), Some(1));

    let out = spec(&root, &["import", "src", "tests", "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(text(&out).contains("wrote spec/auth/auth_lib.int"));
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "accept import"]);

    let out = spec(&root, &["plan", "--id", "p-cli-import-01"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        text(&out).matches("by acquire").count(),
        3,
        "{}",
        text(&out)
    );
    let out = spec(&root, &["apply"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    // Adopted, but nothing is verified yet: check says so, and the lib's empty test slot
    // shows as unapplied.
    let out = spec(&root, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let found = text(&out);
    assert!(
        found.contains("unapplied              auth.auth_lib"),
        "{found}"
    );
    assert!(
        found.lines().any(|l| l.starts_with("evidence ")
            && l.contains("auth.login#expect.shows_form")
            && l.ends_with("missing (policy tested)")),
        "{found}"
    );
}
