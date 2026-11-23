//! `sdc parse`, run as a process.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn spec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sdc"))
        .args(args)
        .output()
        .unwrap()
}

/// A fresh directory for one test, under the target directory.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const GOOD: &str = "import nextjs\n\nexport nextjs.page login {\n  route: \"/login\"\n}\n";
const BAD: &str = "action a {\n  input: { a: string b: string }\n}\n";

#[test]
fn ok_exits_zero_and_counts() {
    let dir = scratch("ok");
    fs::write(dir.join("a.sd"), GOOD).unwrap();
    fs::create_dir(dir.join("sub")).unwrap();
    fs::write(dir.join("sub/b.sd"), GOOD).unwrap();
    fs::write(dir.join("notes.txt"), "not int").unwrap();
    let out = spec(&["parse", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "parsed 2 files\n");
}

#[test]
fn error_exits_one_with_location() {
    let dir = scratch("error");
    let file = dir.join("bad.sd");
    fs::write(&file, BAD).unwrap();
    let out = spec(&["parse", file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("bad.sd:2:22"), "{stderr}");
    assert!(
        stderr.contains("expected ',' or a line break or '}'"),
        "{stderr}"
    );
    assert!(stderr.contains("found 'b'"), "{stderr}");
}

#[test]
fn json_prints_one_object_per_file() {
    let dir = scratch("json");
    fs::write(dir.join("a.sd"), GOOD).unwrap();
    fs::write(dir.join("b.sd"), BAD).unwrap();
    let out = spec(&["parse", "--json", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0]["path"].as_str().unwrap().ends_with("a.sd"));
    assert_eq!(lines[0]["tree"]["statements"].as_array().unwrap().len(), 2);
    assert_eq!(lines[1]["error"]["line"], 2);
    assert_eq!(lines[1]["error"]["column"], 22);
}

#[test]
fn missing_path_exits_two() {
    let out = spec(&["parse", "definitely/not/here.sd"]);
    assert_eq!(out.status.code(), Some(2));
}
