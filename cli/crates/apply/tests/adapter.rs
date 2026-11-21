//! The @intlang/nextjs adapter's own tests (node:test), run when Node.js is installed.

use std::path::Path;
use std::process::Command;

#[test]
fn nextjs_adapter_scripts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../adapters/nextjs");
    let Ok(out) = Command::new("node").arg("--version").output() else {
        eprintln!("node is not installed; skipping the adapter's tests");
        return;
    };
    assert!(out.status.success());
    let tests: Vec<String> = std::fs::read_dir(dir.join("test"))
        .unwrap()
        .flatten()
        .map(|e| e.path().to_string_lossy().into_owned())
        .filter(|p| p.ends_with(".test.js"))
        .collect();
    let out = Command::new("node")
        .arg("--test")
        .args(&tests)
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
