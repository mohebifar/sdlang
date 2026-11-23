//! Crate-level tests: the corpus of real `.sd` source in the repository, and the
//! grammar cases from the reference checker (`tools/check_examples.py`).

use std::fs;
use std::path::{Path, PathBuf};

use sdlang_syntax::{diagnostics, lexer, parse};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

/// The bodies of the ```spec blocks in a Markdown file, with their line numbers.
fn spec_blocks(markdown: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (i, line) in markdown.lines().enumerate() {
        match &mut current {
            None if line == "```spec" => current = Some((i + 1, String::new())),
            Some(_) if line == "```" => blocks.push(current.take().unwrap()),
            Some((_, body)) => {
                body.push_str(line);
                body.push('\n');
            }
            None => {}
        }
    }
    blocks
}

fn int_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            int_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "sd") {
            out.push(path);
        }
    }
}

fn check(name: &str, src: &str, failures: &mut Vec<String>) {
    let src = lexer::normalize(src);
    if let Err(e) = parse(&src) {
        failures.push(diagnostics::render(name, &src, &e));
    }
}

#[test]
fn corpus_parses() {
    let root = repo_root();
    let mut failures = Vec::new();
    let mut blocks = 0;
    for doc in [
        "docs/spec.md",
        "docs/protocol.md",
        "docs/extensions.md",
        "docs/trial.md",
    ] {
        let text = fs::read_to_string(root.join(doc)).unwrap();
        for (line, body) in spec_blocks(&text) {
            blocks += 1;
            check(
                &format!("{doc} (block at line {line})"),
                &body,
                &mut failures,
            );
        }
    }
    let mut files = Vec::new();
    int_files(&root.join("examples"), &mut files);
    int_files(&root.join("cli"), &mut files);
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let name = path.strip_prefix(&root).unwrap().display().to_string();
        check(&name, &src, &mut failures);
    }
    assert!(blocks >= 30, "found only {blocks} spec blocks");
    assert!(files.len() >= 9, "found only {} .sd files", files.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Inputs the reference checker must reject (its NEGATIVE table).
const NEGATIVE: &[(&str, &str)] = &[
    (
        "union continued on next line",
        "type Plan = \"free\"\n  | \"team\"\n",
    ),
    ("two statements on one line", "import a import b\n"),
    ("keyword as declaration name", "entity type {\n}\n"),
    ("unnamed named declaration", "nextjs.page {\n}\n"),
    ("named stack", "stack main {\n}\n"),
    (
        "mention outside a string",
        "action a {\n  to: @{login}\n}\n",
    ),
    (
        "malformed mention",
        "action a {\n  prompt: \"go to @{ login }\"\n}\n",
    ),
    (
        "missing separator",
        "action a {\n  input: { a: string b: string }\n}\n",
    ),
    (
        "closing prose not on own line",
        "action a {\n  prompt: \"\"\"\n    text\"\"\"\n}\n",
    ),
    (
        "tab indentation in prose",
        "action a {\n  prompt: \"\"\"\n\ttext\n  \"\"\"\n}\n",
    ),
    (
        "raw newline in line string",
        "action a {\n  summary: \"a\nb\"\n}\n",
    ),
    ("repeated export", "export export action a {\n}\n"),
    (
        "member address without collection",
        "moved a.b#redirects -> a.b#expect.x\n",
    ),
    ("template is not v0", "template crud(e: entity) {\n}\n"),
    (
        "record inside a selector operand list",
        "rule r {\n  applies_to: select * where config in [{ mode: \"a\" }]\n}\n",
    ),
    (
        "empty selector kind list",
        "rule r {\n  applies_to: select []\n}\n",
    ),
    (
        "meta with unknown field",
        "rule r {\n  applies_to: select * where meta.path == \"x\"\n}\n",
    ),
];

/// Inputs the reference checker must accept (its POSITIVE table).
const POSITIVE: &[(&str, &str)] = &[
    ("keyword field names", "ui.component Button {\n  type: \"button\"\n  module { kind: \"x\" }\n}\n"),
    ("union continued after bar", "type Plan = \"free\" |\n  \"team\"\n"),
    ("literal openers", "action a {\n  prompt: \"write @@{x} and $${y} and a@b.com\"\n}\n"),
    (
        "selector across lines",
        "rule r {\n  applies_to: select [nextjs.page, nextjs.endpoint] where\n    route matches\n      \"/app/**\" and not exists layout\n  require: \"x\"\n}\n",
    ),
    ("empty file", ""),
    ("comment before statement boundary", "import a // note\nimport b\n"),
];

#[test]
fn grammar_cases() {
    let mut wrong = Vec::new();
    for (name, src) in NEGATIVE {
        if parse(src).is_ok() {
            wrong.push(format!("accepted (should reject): {name}"));
        }
    }
    for (name, src) in POSITIVE {
        if let Err(e) = parse(src) {
            wrong.push(format!("rejected (should accept): {name}: {e}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn grammar_cases_match_the_reference_checker() {
    // Guard against the two tables drifting apart: every case name here must appear in
    // the Python checker's tables, and vice versa.
    let checker = fs::read_to_string(repo_root().join("tools/check_examples.py")).unwrap();
    let start = checker.find("NEGATIVE = {").unwrap();
    let end = checker[start..].find("\n\n\ndef ").unwrap() + start;
    let tables = &checker[start..end];
    let python_names: Vec<&str> = tables
        .lines()
        .filter_map(|l| l.trim().strip_prefix('"'))
        .filter_map(|l| l.split_once("\":").map(|(name, _)| name))
        .collect();
    let rust_names: Vec<&str> = NEGATIVE.iter().chain(POSITIVE).map(|(n, _)| *n).collect();
    for name in &python_names {
        assert!(rust_names.contains(name), "case missing here: {name}");
    }
    for name in &rust_names {
        assert!(
            python_names.contains(name),
            "case missing in the checker: {name}"
        );
    }
}
