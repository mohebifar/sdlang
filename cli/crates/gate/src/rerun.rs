//! `sdc check --rerun` (docs/spec.md, Team workflow): reruns every test run the evidence
//! relies on, in a scratch worktree at HEAD, and reports any case whose outcome differs
//! from the recorded one. It records nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use sdlang_apply::checks::{resolve_checks, run_check, RunContext};
use sdlang_apply::workspace::Repo;
use sdlang_plan::state::declarations;

use crate::error::{GResult, GateError};
use crate::findings::{CheckOptions, Finding, Loaded};

/// One test run to reproduce: the check on one test output of one declaration.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Run {
    address: String,
    check: String,
    output: String,
}

pub fn rerun(
    options: &CheckOptions,
    loaded: &Loaded,
    skip: &BTreeSet<String>,
) -> GResult<Vec<Finding>> {
    // Which runs, and which recorded cases each must reproduce: (requirement, case).
    let mut runs: BTreeMap<Run, Vec<(String, String)>> = BTreeMap::new();
    for (address, record) in &loaded.records {
        if skip.contains(address) {
            continue;
        }
        for e in &record.evidence {
            let (Some(check), Some(requirement)) = (
                e["run"]["check"].as_str(),
                e["subject"]["requirement"].as_str(),
            ) else {
                continue;
            };
            for t in e["coverage"]["tests"].as_array().into_iter().flatten() {
                let (Some(output), Some(case)) = (t["output"].as_str(), t["case"].as_str()) else {
                    continue;
                };
                runs.entry(Run {
                    address: address.clone(),
                    check: check.into(),
                    output: output.into(),
                })
                .or_default()
                .push((requirement.to_string(), case.to_string()));
            }
        }
    }
    if runs.is_empty() {
        return Ok(vec![]);
    }
    let repo = Repo::open(&options.root)?;
    let head = repo.head()?;
    let candidate = repo.candidate("check-rerun", &head)?;
    let result = reproduce(options, loaded, &candidate.dir, &head, &runs);
    repo.remove_candidate(&candidate);
    result
}

fn reproduce(
    options: &CheckOptions,
    loaded: &Loaded,
    dir: &std::path::Path,
    head: &str,
    runs: &BTreeMap<Run, Vec<(String, String)>>,
) -> GResult<Vec<Finding>> {
    let decls = declarations(&loaded.graph);
    let area = dir.join(".spec/run/check-rerun");
    let scratch = area.join("scratch");
    fs::create_dir_all(&scratch).map_err(|e| GateError::new(e.to_string()))?;
    let graph_path = area.join("graph.json");
    fs::write(&graph_path, loaded.graph.to_string()).map_err(|e| GateError::new(e.to_string()))?;
    let mut out = Vec::new();
    for (run, cases) in runs {
        let finding = |requirement: &str, why: String| {
            Finding::new("evidence", requirement, format!("rerun: {why}"))
        };
        let decl = decls.get(run.address.as_str()).copied();
        let def = decl.and_then(|d| {
            resolve_checks(&loaded.project, d)
                .into_iter()
                .find(|c| c.id == run.check)
        });
        let Some(def) = def else {
            for (requirement, _) in cases {
                out.push(finding(
                    requirement,
                    format!("check {} no longer applies", run.check),
                ));
            }
            continue;
        };
        let outputs = loaded.outputs_of(&run.address);
        let Some(path) = outputs
            .iter()
            .find(|(m, _)| *m == run.output)
            .map(|(_, p)| p.clone())
        else {
            for (requirement, _) in cases {
                out.push(finding(requirement, format!("{} has no path", run.output)));
            }
            continue;
        };
        let inputs = loaded
            .run_digest(dir, &options.profile, &run.check, &run.address)?
            .unwrap_or_default();
        let expected: BTreeMap<String, Vec<String>> =
            cases.iter().map(|(_, c)| (c.clone(), vec![])).collect();
        let ctx = RunContext {
            receipt: format!("rerun:{}:{}", run.check, run.output),
            workdir: dir,
            scratch: &scratch,
            graph_path: &graph_path,
            snapshot: head,
            profile: &options.profile,
            subject: Some(&run.address),
            outputs: outputs.clone(),
            test_output: Some((run.output.clone(), path)),
            expected_cases: expected,
            inputs,
            timeout: options.check_timeout,
        };
        let result = run_check(&def, &ctx)?;
        for (requirement, case) in cases {
            let why = match result.cases.get(case) {
                Some(c) if c.passed => continue,
                Some(_) => format!("case {case} of {} now fails", run.output),
                None if result.classification == Some("infrastructure") => {
                    format!("{} could not run: {}", run.check, result.excerpt)
                }
                None => format!("case {case} of {} is missing", run.output),
            };
            out.push(finding(requirement, why));
        }
    }
    Ok(out)
}
