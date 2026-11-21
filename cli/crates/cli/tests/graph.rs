//! `spec graph`, run as a process.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn spec_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spec"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(rel)
}

#[test]
fn prints_the_graph() {
    let out = spec_in(&repo("."), &["graph", "examples/worked/after"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let graph: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(graph["schema"], "intlang.graph/1");
    assert_eq!(graph["declarations"].as_array().unwrap().len(), 13);
}

#[test]
fn defaults_to_the_current_directory() {
    let out = spec_in(&repo("examples/worked/before"), &["graph"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn errors_exit_one_with_a_location() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("graph-error");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("spec")).unwrap();
    fs::write(root.join("spec/app.int"), "lib x {\n  uses: [nowhere]\n}\n").unwrap();
    let out = spec_in(&root, &["graph"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("spec/app.int:2:10"), "{stderr}");
    assert!(stderr.contains("'nowhere' is not bound here"), "{stderr}");
}
