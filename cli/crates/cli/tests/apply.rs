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
    // The applied branch is consistent; bye's lack of requirements is only a warning.
    let out = spec(&root, &["check"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(
        text.contains("unspecified (warning)") && text.ends_with("consistent\n"),
        "{text}"
    );
    assert_eq!(spec(&root, &["check", "--strict"]).status.code(), Some(1));
    let out = spec(&root, &["check", "--json", "--strict"]);
    let findings: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(findings[0]["kind"], "unspecified");
    assert_eq!(
        spec(&root, &["check", "--allow", "nonsense"]).status.code(),
        Some(2)
    );
    fs::write(root.join("notes/hello.txt"), "note by hand\n").unwrap();
    let out = spec(&root, &["check", "--allow", "evidence"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.starts_with("drift "), "{text}");
    git(&root, &["checkout", "--", "notes/hello.txt"]);
    // Applying the same plan again refuses: its binding no longer holds without a journal.
    fs::remove_dir_all(root.join(".spec/journal")).unwrap();
    let out = spec(
        &root,
        &["apply", "--agent", &agent, "--reviewer", &reviewer],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("binding mismatch"));
}

#[test]
fn agents_use_the_mcp_server() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-apply-mcp");
    let _ = fs::remove_dir_all(&root);
    copy_dir(&fixture("project"), &root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "baseline"]);
    assert_eq!(
        spec(&root, &["plan", "--id", "p-mcp-01"]).status.code(),
        Some(0)
    );
    let log = root.parent().unwrap().join("cli-apply-mcp.log");
    let _ = fs::remove_file(&log);
    let agent = format!(
        "MCP_LOG={} python3 {}",
        log.display(),
        fixture("tools/mcp_agent.py").display()
    );
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
    let sessions: Vec<serde_json::Value> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        sessions.len(),
        2,
        "one session per coding dispatch (hello, bye)"
    );
    let hello = sessions
        .iter()
        .find(|s| s["get_task"]["subjects"][0]["address"] == "hello")
        .unwrap();
    assert_eq!(hello["tools"].as_array().unwrap().len(), 6);
    assert_eq!(hello["initialize"]["serverInfo"]["name"], "spec");
    assert_eq!(
        hello["get_task"]["subjects"][0]["obligations"],
        serde_json::json!(["verify", "review"])
    );
    assert!(hello["own_public"]["hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(hello["missing"]["error"], "not_found");
    assert_eq!(hello["validate"]["valid"], true, "{}", hello["validate"]);
    assert_eq!(hello["validate_with_rogue"]["valid"], false);
    let bare = &hello["validate_bare_output"];
    assert_eq!(bare["valid"], false, "{bare}");
    assert!(
        bare["problems"][0]["message"]
            .as_str()
            .is_some_and(|m| m.contains("hello#outputs.tests")),
        "{bare}"
    );
    assert_eq!(
        hello["validate_with_rogue"]["problems"][0]["stage"],
        "envelope"
    );
    assert_eq!(
        hello["self_check"]["result"], "passed",
        "{}",
        hello["self_check"]
    );
    assert_eq!(hello["self_check"]["advisory"], true);
    assert!(hello["not_a_check"]["tool_error"].is_string());
    let calls =
        fs::read_to_string(root.join(".spec/run/p-mcp-01/r1/tx-2/1/mcp-calls.jsonl")).unwrap();
    assert!(calls.contains("\"tool\":\"validate_result\""), "{calls}");
}
