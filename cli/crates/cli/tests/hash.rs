//! `spec hash`, run as a process.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn spec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spec"))
        .args(args)
        .output()
        .unwrap()
}

fn repo_file(rel: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(rel)
        .to_str()
        .unwrap()
        .to_string()
}

fn scratch_file(name: &str, contents: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::write(&path, contents).unwrap();
    path
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn digest_by_default() {
    let file = scratch_file("digest.json", "{ \"b\": 1, \"a\": [true, null, \"x\"] }");
    let out = spec(&["hash", file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        stdout(&out).trim(),
        "sha256:54a65415ad370228851a1da4b31b6fd42dc58b19a50d35cae759325f7388ce64"
    );
}

#[test]
fn canonical_text() {
    let file = scratch_file("canonical.json", "{ \"n\": 2.0, \"m\": 1e-7 }");
    let out = spec(&["hash", "--canonical", file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out).trim(), r#"{"m":1e-7,"n":2}"#);
}

#[test]
fn graph_projections_match_the_fixture() {
    let out = spec(&["hash", "--graph", &repo_file("examples/graph.after.json")]);
    assert_eq!(out.status.code(), Some(0));
    let got: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let fixture: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo_file("examples/hashes.json")).unwrap())
            .unwrap();
    assert_eq!(got, fixture["after"]);
}

#[test]
fn invalid_input_exits_one() {
    let bad = scratch_file("bad.json", "{ not json");
    assert_eq!(
        spec(&["hash", bad.to_str().unwrap()]).status.code(),
        Some(1)
    );
    let huge = scratch_file("huge.json", "[1e999]");
    let out = spec(&["hash", huge.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("non-finite"));
}
