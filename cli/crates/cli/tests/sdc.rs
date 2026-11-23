//! The `sdc` binary itself: help and version.

use std::process::Command;

fn spec(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sdc"))
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
    for command in [
        "parse", "hash", "graph", "plan", "apply", "mcp", "check", "import",
    ] {
        assert!(
            stdout
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{command} "))),
            "{command} missing from {stdout}"
        );
    }
    assert!(stdout.contains("Parse .sd files and report syntax errors"));
}

#[test]
fn version_prints_crate_version() {
    let (code, stdout) = spec(&["--version"]);
    assert_eq!(code, Some(0));
    assert_eq!(stdout.trim(), format!("sdc {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, _) = spec(&["frobnicate"]);
    assert_eq!(code, Some(2));
}
