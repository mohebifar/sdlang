//! The `spec` binary itself: help and version.

use std::process::Command;

fn spec(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spec"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn help_lists_every_command() {
    let (code, stdout) = spec(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(
        stdout.contains("parse  Parse .int files and report syntax errors"),
        "{stdout}"
    );
}

#[test]
fn version_prints_crate_version() {
    let (code, stdout) = spec(&["--version"]);
    assert_eq!(code, Some(0));
    assert_eq!(stdout.trim(), format!("spec {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, _) = spec(&["frobnicate"]);
    assert_eq!(code, Some(2));
}
