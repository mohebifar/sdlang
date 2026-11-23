//! Crate-level tests: the worked example's source trees against its graph fixtures.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(repo(rel)).unwrap()).unwrap()
}

/// Lists every difference between two JSON values, by path.
fn diff(path: &str, got: &Value, want: &Value, out: &mut Vec<String>) {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            for (k, wv) in w {
                match g.get(k) {
                    Some(gv) => diff(&format!("{path}.{k}"), gv, wv, out),
                    None => out.push(format!("{path}.{k}: missing (want {wv})")),
                }
            }
            for k in g.keys().filter(|k| !w.contains_key(*k)) {
                out.push(format!("{path}.{k}: unexpected {}", g[k]));
            }
        }
        (Value::Array(g), Value::Array(w)) if g.len() == w.len() => {
            for (i, (gv, wv)) in g.iter().zip(w).enumerate() {
                diff(&format!("{path}[{i}]"), gv, wv, out);
            }
        }
        _ if got != want => out.push(format!("{path}: got {got}, want {want}")),
        _ => {}
    }
}

fn check(side: &str) {
    let mut got = sdlang_graph::build(&repo(&format!("examples/worked/{side}")))
        .unwrap_or_else(|e| panic!("{}", e.render()));
    let mut want = read_json(&format!("examples/graph.{side}.json"));
    // The lock digest's input format is open (O.5); everything else must match.
    got.as_object_mut().unwrap().remove("lock");
    want.as_object_mut().unwrap().remove("lock");
    let mut out = Vec::new();
    diff("graph", &got, &want, &mut out);
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
}

#[test]
fn worked_before() {
    check("before");
}

#[test]
fn worked_after() {
    check("after");
}

#[test]
fn projections() {
    let hashes = read_json("examples/hashes.json");
    for side in ["before", "after"] {
        let graph = sdlang_graph::build(&repo(&format!("examples/worked/{side}"))).unwrap();
        let got = sdlang_canon::projections(&graph).unwrap();
        for (address, p) in got {
            assert_eq!(p.to_value(), hashes[side][&address], "{side} {address}");
        }
    }
}
