//! Crate-level tests: the shared vectors and the worked example's projection hashes.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_file(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(repo_file(rel)).unwrap()).unwrap()
}

#[test]
fn vectors() {
    let doc = read_json("examples/hash-vectors.json");
    let vectors = doc["vectors"].as_array().unwrap();
    assert!(vectors.len() >= 15, "only {} vectors", vectors.len());
    let mut wrong = Vec::new();
    for v in vectors {
        let name = v["name"].as_str().unwrap();
        let text = intlang_canon::canonical(&v["value"]).unwrap();
        let digest = intlang_canon::digest(&v["value"]).unwrap();
        if text != v["canonical"].as_str().unwrap() {
            wrong.push(format!("{name}: canonical {text} != {}", v["canonical"]));
        }
        if digest != v["sha256"].as_str().unwrap() {
            wrong.push(format!("{name}: digest {digest} != {}", v["sha256"]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn worked_hashes() {
    let expected = read_json("examples/hashes.json");
    let mut wrong = Vec::new();
    for (side, file) in [
        ("before", "examples/graph.before.json"),
        ("after", "examples/graph.after.json"),
    ] {
        let graph = read_json(file);
        let got = intlang_canon::projections(&graph).unwrap();
        let want = expected[side].as_object().unwrap();
        assert_eq!(got.len(), want.len(), "{side}: declaration count");
        for (address, projections) in &got {
            if projections.to_value() != want[address] {
                wrong.push(format!(
                    "{side} {address}: {} != {}",
                    projections.to_value(),
                    want[address]
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Generated numbers through both implementations: Rust and tools/canon.py must write
/// identical canonical text. Skipped (with a note) when python3 is not installed.
#[test]
fn numbers_agree_with_the_python_reference() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut values: Vec<f64> = Vec::new();
    for e in -12..=22 {
        for m in [1.0, 1.5, 2.5, 9.999999, 1.234_567_890_123] {
            values.push(m * 10f64.powi(e));
            values.push(-m * 10f64.powi(e));
        }
    }
    values.extend([
        0.1,
        0.2,
        0.30000000000000004,
        1.0 / 3.0,
        5e-324,
        2.2250738585072014e-308,
        1.7976931348623157e308,
    ]);
    // A fixed linear congruential generator over raw bit patterns.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    while values.len() < 4000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = f64::from_bits(state);
        if x.is_finite() {
            values.push(x);
        }
    }
    let texts: Vec<String> = values.iter().map(|x| format!("{x:e}")).collect();
    let json_array = format!("[{}]", texts.join(","));

    let script = "import json,sys; sys.path.insert(0, sys.argv[1]); import canon\n\
                  for v in json.loads(sys.stdin.read()): print(canon.canonical(v))";
    let tools = repo_file("tools");
    let child = Command::new("python3")
        .args(["-c", script, tools.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        eprintln!("python3 not found; skipping the cross-language check");
        return;
    };
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_array.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "python failed");
    let python: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(python.len(), texts.len());
    let mut wrong = Vec::new();
    for (text, py) in texts.iter().zip(&python) {
        let value: Value = serde_json::from_str(text).unwrap();
        let rust = intlang_canon::canonical(&value).unwrap();
        if &rust != py {
            wrong.push(format!("{text}: rust {rust} python {py}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} differ:\n{}",
        wrong.len(),
        texts.len(),
        wrong[..wrong.len().min(20)].join("\n")
    );
}
