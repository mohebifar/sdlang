//! The ownership index (docs/spec.md, Ownership and drift): derived from the state records,
//! never a second source of truth, and checked for collisions and dangling claims.

use std::collections::BTreeMap;

use sdlang_plan::state::StateRecord;

use crate::findings::Finding;

/// Path -> every `(address, output key)` that claims it.
pub fn index(records: &[(String, StateRecord)]) -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (_, r) in records {
        for (key, o) in &r.outputs {
            out.entry(o.path.clone())
                .or_default()
                .push((r.address.clone(), key.clone()));
        }
    }
    out
}

fn unsafe_path(path: &str) -> bool {
    path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
}

/// Collisions and dangling claims among the records, given with their file names.
pub fn ownership_findings(records: &[(String, StateRecord)]) -> Vec<Finding> {
    let mut out = Vec::new();
    for (path, owners) in index(records) {
        if owners.len() > 1 {
            let names: Vec<String> = owners.iter().map(|(a, k)| format!("{a}#{k}")).collect();
            out.push(Finding::new(
                "ownership",
                &path,
                format!("claimed by {}", names.join(" and ")),
            ));
        }
    }
    for (file, r) in records {
        if *file != format!("{}.json", r.address) {
            out.push(Finding::new(
                "ownership",
                &format!(".spec/state/{file}"),
                format!("dangling: holds the record of {}", r.address),
            ));
        }
        for (key, o) in &r.outputs {
            let subject = format!("{}#outputs.{key}", r.address);
            if unsafe_path(&o.path) {
                out.push(Finding::new(
                    "ownership",
                    &subject,
                    format!("dangling: path '{}' is absolute or not normalised", o.path),
                ));
                continue;
            }
            if o.origin == "created" {
                let rel = o.relative_path.as_deref().unwrap_or_default();
                let inside = !rel.is_empty() && !unsafe_path(rel) && o.path.ends_with(rel);
                if !inside {
                    out.push(Finding::new(
                        "ownership",
                        &subject,
                        format!(
                            "dangling: created output {} is not under its root {}",
                            o.path,
                            o.root.as_deref().unwrap_or("(none)")
                        ),
                    ));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdlang_plan::state::{Hashes, Output};

    fn record(address: &str, outputs: &[(&str, &str)]) -> StateRecord {
        StateRecord {
            address: address.into(),
            kind: "core:lib".into(),
            entry: None,
            source: None,
            hashes: Hashes {
                public: String::new(),
                behaviour: String::new(),
                body: String::new(),
                context: String::new(),
            },
            context_parts: BTreeMap::new(),
            consumed: vec![],
            members: BTreeMap::new(),
            outputs: outputs
                .iter()
                .map(|(k, p)| {
                    (
                        k.to_string(),
                        Output {
                            origin: "slot".into(),
                            path: p.to_string(),
                            bytes: String::new(),
                            root: None,
                            relative_path: None,
                        },
                    )
                })
                .collect(),
            evidence: vec![],
            provenance: None,
        }
    }

    #[test]
    fn collision_names_both_owners() {
        let records = vec![
            ("a.json".into(), record("a", &[("main", "src/x.ts")])),
            ("b.json".into(), record("b", &[("main", "src/x.ts")])),
        ];
        let f = ownership_findings(&records);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].subject, "src/x.ts");
        assert!(f[0].detail.contains("a#main") && f[0].detail.contains("b#main"));
    }

    #[test]
    fn dangling_claims() {
        let records = vec![
            ("other.json".into(), record("a", &[("main", "src/a.ts")])),
            ("b.json".into(), record("b", &[("main", "../escape.ts")])),
        ];
        let f = ownership_findings(&records);
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|f| f.detail.starts_with("dangling")));
    }
}
