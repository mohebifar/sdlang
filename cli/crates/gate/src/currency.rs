//! Whether a requirement's recorded evidence still meets its effective policy on the
//! current inputs (docs/spec.md, Verification -> Currency). Reviews, coverage judgements and
//! approvals are never reproduced here; what can be recomputed is the contract each was
//! given and the digest of every test run's input closure.

use sdlang_apply::evidence::Requirement;
use sdlang_plan::state::StateRecord;
use serde_json::{json, Value};

use crate::error::GResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Met,
    /// No evidence, or evidence that never met its policy.
    Missing,
    Failed,
    /// Evidence whose inputs changed; the reason names which.
    Stale(String),
}

/// The evidence record of `req` in `record`, if any.
pub fn evidence_of<'r>(record: &'r StateRecord, req: &Requirement) -> Option<&'r Value> {
    record
        .evidence
        .iter()
        .find(|e| e["subject"]["requirement"] == json!(req.member))
}

/// The status of one requirement. `run_digest(check, address)` recomputes the input
/// digest a run of `check` on `address` would have now; `None` when the check no longer
/// applies.
pub fn requirement_status(
    req: &Requirement,
    record: &StateRecord,
    run_digest: &dyn Fn(&str, &str) -> GResult<Option<String>>,
) -> GResult<Status> {
    let Some(e) = evidence_of(record, req) else {
        return Ok(Status::Missing);
    };
    if e["policy"] != req.policy.to_value() {
        return Ok(Status::Stale("policy changed".into()));
    }
    let contract = e["coverage"]["contract"]
        .as_str()
        .or(e["review"]["contract"].as_str());
    if let Some(c) = contract {
        if c != req.contract_hash {
            return Ok(Status::Stale("requirement text changed".into()));
        }
    }
    match e["summary"].as_str() {
        Some("failed") => return Ok(Status::Failed),
        Some("stale") => return Ok(Status::Stale("recorded stale".into())),
        Some("unverified") | None => return Ok(Status::Missing),
        _ => {}
    }
    if req.policy.tests {
        let (Some(check), Some(recorded)) =
            (e["run"]["check"].as_str(), e["run"]["inputs"].as_str())
        else {
            return Ok(Status::Missing);
        };
        match run_digest(check, &req.target)? {
            Some(now) if now == recorded => {}
            Some(_) => return Ok(Status::Stale("run inputs changed".into())),
            None => return Ok(Status::Stale(format!("check {check} no longer applies"))),
        }
    }
    Ok(Status::Met)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdlang_apply::evidence::Policy;
    use sdlang_plan::state::Hashes;
    use std::collections::BTreeMap;

    fn req(tests: bool) -> Requirement {
        Requirement {
            target: "a".into(),
            member: "a#expect.e".into(),
            text: "x".into(),
            contract_hash: "sha256:c".into(),
            policy: Policy {
                tests,
                review: true,
                ..Policy::default()
            },
            policy_source: "project",
            tests: vec![],
        }
    }

    fn record(evidence: Vec<Value>) -> StateRecord {
        StateRecord {
            address: "a".into(),
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
            outputs: BTreeMap::new(),
            evidence,
            provenance: None,
        }
    }

    fn tested(contract: &str, inputs: &str, summary: &str) -> Value {
        json!({
            "subject": { "target": "a", "requirement": "a#expect.e" },
            "policy": req(true).policy.to_value(),
            "coverage": { "contract": contract, "verdict": "sufficient" },
            "run": { "check": "toy.test", "inputs": inputs, "attempts": ["passed"] },
            "summary": summary,
        })
    }

    #[test]
    fn statuses() {
        let same = |_: &str, _: &str| Ok(Some("sha256:i".to_string()));
        let r = req(true);
        let s = |e: Vec<Value>| requirement_status(&r, &record(e), &same).unwrap();
        assert_eq!(s(vec![]), Status::Missing);
        assert_eq!(
            s(vec![tested("sha256:c", "sha256:i", "tested")]),
            Status::Met
        );
        assert_eq!(
            s(vec![tested("sha256:c", "sha256:i", "failed")]),
            Status::Failed
        );
        assert_eq!(
            s(vec![tested("sha256:c", "sha256:i", "unverified")]),
            Status::Missing
        );
        assert!(matches!(
            s(vec![tested("sha256:old", "sha256:i", "tested")]),
            Status::Stale(_)
        ));
        assert!(matches!(
            s(vec![tested("sha256:c", "sha256:old", "tested")]),
            Status::Stale(_)
        ));
        let judged = req(false);
        assert!(matches!(
            requirement_status(
                &judged,
                &record(vec![tested("sha256:c", "sha256:i", "tested")]),
                &same
            )
            .unwrap(),
            Status::Stale(_)
        ));
    }
}
