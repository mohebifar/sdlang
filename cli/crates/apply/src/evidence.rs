//! Requirements, policies and evidence records (SPEC.md, Verification).

use std::collections::BTreeMap;

use intlang_canon::digest;
use serde_json::{json, Value};

use crate::checks::CheckRun;
use crate::error::AResult;

/// A policy is a conjunction, never a rank.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub tests: bool,
    pub review: bool,
    pub checks: Vec<String>,
    pub approval: bool,
}

impl Policy {
    /// Reads a `verify` value: `"tested"`, `"judged"`, `"accepted"` or a record.
    pub fn parse(v: &Value) -> Option<Policy> {
        let v = v.get("$record").unwrap_or(v);
        match v.as_str() {
            Some("tested") => Some(Policy {
                tests: true,
                review: true,
                ..Policy::default()
            }),
            Some("judged") => Some(Policy {
                review: true,
                ..Policy::default()
            }),
            Some("accepted") => Some(Policy {
                approval: true,
                ..Policy::default()
            }),
            Some(_) => None,
            None => {
                let o = v.as_object()?;
                let flag = |k: &str| o.get(k).and_then(Value::as_bool).unwrap_or(false);
                Some(Policy {
                    tests: flag("tests"),
                    review: flag("review"),
                    checks: o
                        .get("checks")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|c| c.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    approval: flag("approval"),
                })
            }
        }
    }

    pub fn union(&mut self, other: &Policy) {
        self.tests |= other.tests;
        self.review |= other.review;
        self.approval |= other.approval;
        for c in &other.checks {
            if !self.checks.contains(c) {
                self.checks.push(c.clone());
            }
        }
        self.checks.sort();
    }

    pub fn to_value(&self) -> Value {
        json!({ "tests": self.tests, "review": self.review, "checks": self.checks, "approval": self.approval })
    }

    /// The label a met policy earns.
    pub fn label(&self) -> &'static str {
        if self.tests {
            "tested"
        } else if self.review {
            "judged"
        } else if self.approval {
            "accepted"
        } else {
            "none"
        }
    }
}

/// One requirement of a declaration.
#[derive(Clone, Debug)]
pub struct Requirement {
    pub target: String,
    /// `<owner>#expect.<key>` or `<owner>#errors.<key>`.
    pub member: String,
    pub text: String,
    pub contract_hash: String,
    pub policy: Policy,
    pub policy_source: &'static str,
    /// Test bindings `(output member, case)`.
    pub tests: Vec<(String, String)>,
}

impl Requirement {
    pub fn subject(&self) -> Value {
        json!({ "target": self.target, "requirement": self.member })
    }
}

fn prose(v: &Value) -> String {
    v.get("$prose")
        .and_then(Value::as_str)
        .or_else(|| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// A declaration's requirements: its own `expect` and `errors` members and the `expect`
/// members of the rules that apply to it, each under its effective policy.
pub fn requirements(graph: &Value, decl: &Value) -> AResult<Vec<Requirement>> {
    let decls: BTreeMap<&str, &Value> = graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["address"].as_str()?, d)))
        .collect();
    let project_policy = graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|d| d["kind"] == "core:project")
        .and_then(|p| Policy::parse(&p["fields"]["verify"]));
    let rules: Vec<&Value> = decl["applies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| decls.get(r.as_str()?).copied())
        .collect();
    let rule_policy = rules
        .iter()
        .filter_map(|r| Policy::parse(&r["fields"]["verify"]))
        .fold(None, |acc: Option<Policy>, p| {
            let mut a = acc.unwrap_or_default();
            a.union(&p);
            Some(a)
        });
    let target = decl["address"].as_str().unwrap_or_default();
    let effective = |own: Option<Policy>| -> (Policy, &'static str) {
        let source = if own.is_some() {
            "member"
        } else if rule_policy.is_some() {
            "rule"
        } else if project_policy.is_some() {
            "project"
        } else {
            "default"
        };
        let mut p = own.unwrap_or_default();
        for other in [&rule_policy, &project_policy].into_iter().flatten() {
            p.union(other);
        }
        if p == Policy::default() {
            p.review = true;
        }
        (p, source)
    };
    let mut out = Vec::new();
    let mut add = |owner: &str, collection: &str, members: &Value| -> AResult<()> {
        for (key, m) in members.as_object().into_iter().flatten() {
            let text = if collection == "errors" {
                prose(&m["summary"])
            } else {
                prose(&m["text"])
            };
            let (policy, policy_source) = effective(Policy::parse(&m["verify"]));
            out.push(Requirement {
                target: target.to_string(),
                member: format!("{owner}#{collection}.{key}"),
                text,
                contract_hash: digest(m)?,
                policy,
                policy_source,
                tests: vec![],
            });
        }
        Ok(())
    };
    add(target, "expect", &decl["members"]["expect"])?;
    add(target, "errors", &decl["members"]["errors"])?;
    for rule in rules {
        add(
            rule["address"].as_str().unwrap_or_default(),
            "expect",
            &rule["members"]["expect"],
        )?;
    }
    Ok(out)
}

/// A reviewer's answer for one subject.
#[derive(Clone, Debug, Default)]
pub struct Verdict {
    pub dispatch: String,
    pub conformance: Option<String>,
    pub coverage: Option<String>,
    /// The conformance reason.
    pub reason: String,
    /// The coverage reason, given back to the agent when coverage is insufficient.
    pub coverage_reason: String,
}

/// The evidence record of one requirement and whether its policy is met.
pub fn requirement_evidence(
    req: &Requirement,
    runs: &[CheckRun],
    verdict: Option<&Verdict>,
) -> AResult<(Value, bool)> {
    let mut failed = false;
    let mut missing = false;
    let mut evidence = json!({ "subject": req.subject(), "policy": req.policy.to_value() });
    if req.policy.tests {
        let mut oracles = Vec::new();
        let mut attempts = Vec::new();
        let mut check = None;
        let mut inputs = None;
        if req.tests.is_empty() {
            missing = true;
        }
        for (output, case) in &req.tests {
            let run = runs
                .iter()
                .find(|r| r.output.as_deref() == Some(output.as_str()));
            match run.and_then(|r| r.cases.get(case).map(|c| (r, c))) {
                Some((r, c)) => {
                    oracles.push(json!({ "output": output, "case": case, "oracle": c.oracle }));
                    attempts.push(if c.passed { "passed" } else { "failed" });
                    failed |= !c.passed;
                    check = Some(r.check.clone());
                    inputs = Some(r.inputs.clone());
                }
                None => missing = true,
            }
        }
        let coverage_verdict = verdict.and_then(|v| v.coverage.clone());
        match coverage_verdict.as_deref() {
            Some("sufficient") => {}
            Some("insufficient") => failed = true,
            _ => missing = true,
        }
        evidence["coverage"] = json!({
            "contract": req.contract_hash,
            "tests": req.tests.iter().map(|(o, c)| json!({ "output": o, "case": c })).collect::<Vec<_>>(),
            "oracle": digest(&Value::Array(oracles))?,
            "verdict": coverage_verdict.unwrap_or_else(|| "missing".into()),
            "dispatch": verdict.map(|v| v.dispatch.clone()),
        });
        evidence["run"] = json!({ "inputs": inputs, "check": check, "attempts": attempts });
    } else if req.policy.review {
        let conformance = verdict.and_then(|v| v.conformance.clone());
        match conformance.as_deref() {
            Some("conforms") => {}
            Some("does_not_conform") => failed = true,
            _ => missing = true,
        }
        evidence["review"] = json!({
            "contract": req.contract_hash,
            "verdict": conformance.unwrap_or_else(|| "missing".into()),
            "dispatch": verdict.map(|v| v.dispatch.clone()),
        });
    }
    for rule in &req.policy.checks {
        let status = runs
            .iter()
            .find_map(|r| r.rules.get(rule))
            .map(String::as_str)
            .unwrap_or("missing");
        match status {
            "satisfied" => {}
            "violated" => failed = true,
            _ => missing = true,
        }
    }
    if req.policy.approval {
        // A human approval is never produced by apply.
        missing = true;
    }
    let label = summary_label(failed, false, missing, &req.policy);
    evidence["summary"] = json!(label);
    Ok((evidence, !failed && !missing))
}

/// `failed`, `stale`, `unverified`, or the label of the met policy, in that order.
pub fn summary_label(failed: bool, stale: bool, missing: bool, policy: &Policy) -> &'static str {
    if failed {
        "failed"
    } else if stale {
        "stale"
    } else if missing {
        "unverified"
    } else {
        policy.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_forms() {
        assert_eq!(Policy::parse(&json!("tested")).unwrap().label(), "tested");
        assert_eq!(Policy::parse(&json!("judged")).unwrap().label(), "judged");
        let p = Policy::parse(&json!({ "$record": { "tests": true, "checks": ["x.y"] } })).unwrap();
        assert!(p.tests && !p.review && p.checks == ["x.y"]);
    }

    #[test]
    fn effective_policy_is_a_union_judged_by_default() {
        let graph = json!({ "declarations": [
            { "address": "p", "kind": "core:project", "fields": { "verify": "tested" } },
            { "address": "a", "kind": "core:action", "fields": {}, "members": {
                "expect": { "e": { "text": { "$prose": "x" } } },
                "errors": { "bad": { "summary": "Bad", "code": "bad" } } } }
        ]});
        let reqs = requirements(&graph, &graph["declarations"][1]).unwrap();
        assert_eq!(reqs.len(), 2);
        assert!(reqs
            .iter()
            .all(|r| r.policy.tests && r.policy.review && r.policy_source == "project"));
        assert_eq!(reqs[1].member, "a#errors.bad");
        assert_eq!(reqs[1].text, "Bad");
        let bare = json!({ "declarations": [graph["declarations"][1].clone()] });
        let reqs = requirements(&bare, &bare["declarations"][0]).unwrap();
        assert_eq!(reqs[0].policy.label(), "judged");
    }

    #[test]
    fn approval_is_never_met() {
        let req = Requirement {
            target: "a".into(),
            member: "a#expect.e".into(),
            text: "x".into(),
            contract_hash: digest(&json!(1)).unwrap(),
            policy: Policy {
                approval: true,
                ..Policy::default()
            },
            policy_source: "member",
            tests: vec![],
        };
        let (e, met) = requirement_evidence(&req, &[], None).unwrap();
        assert!(!met);
        assert_eq!(e["summary"], "unverified");
    }
}
