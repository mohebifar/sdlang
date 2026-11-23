//! The transaction loop (docs/spec.md, Apply): binding, tasks, freeze, checks, reviews,
//! retries, publish and journal.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sdlang_canon::{digest, documents, GraphDocuments};
use sdlang_graph::loader::Project;
use sdlang_graph::scope::Scope;
use sdlang_plan::materialise::{kind_body, kind_package, materialise, Materialisation};
use sdlang_plan::state::{
    declarations, file_digest, publish_records, Output, Publish, StateRecord, Store,
};
use sdlang_syntax::ast::{Exportable, Expr, MemberValue, Statement};
use serde_json::{json, Value};

use crate::checks::{
    resolve_checks, run_check, run_input_digest, CheckDef, CheckRun, Closure, RunContext,
};
use crate::error::{AResult, ApplyError};
use crate::evidence::{requirement_evidence, requirements, Requirement, Verdict};
use crate::orders::{self, Env, ReviewSubject, Subject};
use crate::workspace::{Candidate, Change, Repo};

pub struct ApplyOptions {
    pub root: PathBuf,
    pub plan_path: PathBuf,
    /// Shell command run for each coding dispatch.
    pub agent: Option<String>,
    /// Shell command run for each review dispatch.
    pub reviewer: Option<String>,
    pub profile: String,
    pub max_attempts: u32,
    /// How agents start the MCP server, without the dispatch id:
    /// `[<sdc binary>, "mcp", "--dispatch"]`.
    pub spec_command: Vec<String>,
    pub agent_timeout: Duration,
    pub check_timeout: Duration,
}

impl ApplyOptions {
    /// Options with the defaults `sdc apply` uses.
    pub fn new(root: PathBuf, plan_path: PathBuf) -> ApplyOptions {
        ApplyOptions {
            root,
            plan_path,
            agent: None,
            reviewer: None,
            profile: "local".into(),
            max_attempts: 3,
            spec_command: vec!["sdc".into(), "mcp".into(), "--dispatch".into()],
            agent_timeout: Duration::from_secs(1800),
            check_timeout: Duration::from_secs(900),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Published { commit: String },
    AlreadyPublished { commit: String },
    Failed { reason: String },
    Skipped { because: String },
}

#[derive(Clone, Debug)]
pub struct ApplyReport {
    pub transactions: Vec<(String, Outcome)>,
}

impl ApplyReport {
    pub fn all_published(&self) -> bool {
        self.transactions.iter().all(|(_, o)| {
            matches!(
                o,
                Outcome::Published { .. } | Outcome::AlreadyPublished { .. }
            )
        })
    }
}

/// A classified failure of one attempt (`#failure` in the work-order schema).
fn failure(
    attempt: u32,
    snapshot: &str,
    kind: &str,
    classification: &str,
    excerpt: &str,
    check: Option<(&str, &str)>,
) -> Value {
    let mut v = json!({
        "attempt": attempt, "snapshot": snapshot, "kind": kind, "classification": classification,
        "fingerprint": digest(&json!([kind, excerpt])).unwrap_or_default(),
        "excerpt": excerpt, "diagnostics": format!("ctx:log:{}", &digest(&json!(excerpt)).unwrap_or_default()[7..19]),
    });
    if let Some((id, hash)) = check {
        v["check"] = json!({ "id": id, "definition_hash": hash });
    }
    v
}

fn run_command(
    command: &str,
    dir: &Path,
    stdin: &str,
    env: &[(&str, String)],
    timeout: Duration,
) -> AResult<crate::process::Finished> {
    crate::process::run(
        crate::process::Program::Shell(command),
        dir,
        stdin,
        env,
        timeout,
        false,
    )
}

fn write_json(path: &Path, v: &Value) -> AResult<()> {
    fs::create_dir_all(path.parent().unwrap())
        .map_err(|e| ApplyError::new(format!("{}: {e}", path.display())))?;
    fs::write(
        path,
        serde_json::to_string_pretty(v).expect("serialisable") + "\n",
    )
    .map_err(|e| ApplyError::new(format!("{}: {e}", path.display())))
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

struct Loaded {
    project: Project,
    graph: Value,
    docs: GraphDocuments,
    mats: BTreeMap<String, Materialisation>,
}

fn load(root: &Path) -> AResult<Loaded> {
    let project = sdlang_graph::loader::load(root).map_err(|e| ApplyError::new(e.render()))?;
    let graph =
        sdlang_graph::lower::lower_project(&project).map_err(|e| ApplyError::new(e.render()))?;
    let docs = documents(&graph)?;
    let mats = materialise(&project, &graph)?;
    Ok(Loaded {
        project,
        graph,
        docs,
        mats,
    })
}

/// The binding must hold before the first transaction.
fn verify_binding(repo: &Repo, plan: &Value, loaded: &Loaded, root: &Path) -> AResult<()> {
    let b = &plan["binding"];
    let mismatch = |what: &str, want: &Value, got: &str| {
        ApplyError::new(format!(
            "plan binding mismatch: {what} is {got}, the plan expects {want}"
        ))
    };
    let head = repo.head()?;
    if b["baseline_commit"] != json!(head) {
        return Err(mismatch("HEAD", &b["baseline_commit"], &head));
    }
    let lock = digest(&loaded.project.lock)?;
    if b["lock_hash"] != json!(lock) {
        return Err(mismatch("the lock digest", &b["lock_hash"], &lock));
    }
    let state = Store::new(root).state_digest()?;
    if b["state_digest"] != json!(state) {
        return Err(mismatch("the state digest", &b["state_digest"], &state));
    }
    for (path, want) in b["fingerprints"].as_object().into_iter().flatten() {
        let got = file_digest(root, path)?.unwrap_or_else(|| "absent".into());
        if *want != json!(got) {
            return Err(mismatch(path, want, &got));
        }
    }
    let dirty = repo.dirty()?;
    if !dirty.is_empty() {
        return Err(ApplyError::new(format!(
            "the worktree has uncommitted changes: {}",
            dirty.join(", ")
        )));
    }
    Ok(())
}

/// Applies the plan at `options.plan_path`.
pub fn apply(options: &ApplyOptions) -> AResult<ApplyReport> {
    let repo = Repo::open(&options.root)?;
    let root = repo.root.clone();
    let plan_text = fs::read_to_string(&options.plan_path)
        .map_err(|e| ApplyError::new(format!("{}: {e}", options.plan_path.display())))?;
    let plan: Value = serde_json::from_str(&plan_text)
        .map_err(|e| ApplyError::new(format!("{}: {e}", options.plan_path.display())))?;
    if plan["schema"] != "sdlang.plan/1" {
        return Err(ApplyError::new(
            "not a plan document (schema sdlang.plan/1)",
        ));
    }
    let loaded = load(&root)?;
    let graph_hash = digest(&loaded.graph)?;
    if plan["binding"]["graph_after"] != json!(graph_hash) {
        return Err(ApplyError::new(format!(
            "plan binding mismatch: the spec's graph is {graph_hash}, the plan expects {}",
            plan["binding"]["graph_after"]
        )));
    }
    let plan_id = plan["plan"].as_str().unwrap_or_default().to_string();
    let revision = plan["revision"].as_u64().unwrap_or(1);
    let journal_path = root.join(format!(".spec/journal/{plan_id}.jsonl"));
    let receipts: Vec<Value> = fs::read_to_string(&journal_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    match receipts.last() {
        None => verify_binding(&repo, &plan, &loaded, &root)?,
        Some(last) => {
            let head = repo.head()?;
            if last["result_commit"] != json!(head) {
                return Err(ApplyError::new(format!(
                    "HEAD ({head}) is not the last published transaction's commit ({}); an unexpected edit needs a new plan",
                    last["result_commit"]
                )));
            }
        }
    }
    let scope = Scope::build(&loaded.project).map_err(|e| ApplyError::new(e.render()))?;
    let store = Store::new(&root);
    let env = Env {
        project: &loaded.project,
        scope: &scope,
        graph: &loaded.graph,
        docs: &loaded.docs,
        mats: &loaded.mats,
        store: &store,
        repo: &repo,
        plan: &plan,
        plan_hash: digest(&plan)?,
        graph_hash,
        lock_hash: digest(&loaded.project.lock)?,
        spec_command: options.spec_command.clone(),
    };
    let mut report = ApplyReport {
        transactions: vec![],
    };
    let mut not_published: BTreeSet<String> = BTreeSet::new();
    for tx in plan["transactions"].as_array().into_iter().flatten() {
        let id = tx["id"].as_str().unwrap_or_default().to_string();
        if let Some(r) = receipts.iter().find(|r| r["transaction"] == json!(id)) {
            report.transactions.push((
                id,
                Outcome::AlreadyPublished {
                    commit: r["result_commit"].as_str().unwrap_or_default().to_string(),
                },
            ));
            continue;
        }
        let blocked_by: Vec<&str> = tx["after"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|a| not_published.contains(*a))
            .collect();
        if !blocked_by.is_empty() {
            not_published.insert(id.clone());
            report.transactions.push((
                id,
                Outcome::Skipped {
                    because: format!("after {}", blocked_by.join(", ")),
                },
            ));
            continue;
        }
        let runner = TxRunner {
            env: &env,
            options,
            plan_id: &plan_id,
            revision,
            tx,
            id: &id,
        };
        let outcome = runner.run()?;
        if let Outcome::Published { commit } = &outcome {
            let base = receipts_base(&journal_path, &repo)?;
            repo.fast_forward(commit)?;
            let receipt =
                json!({ "transaction": id, "input_commit": base, "result_commit": commit });
            fs::create_dir_all(journal_path.parent().unwrap())
                .map_err(|e| ApplyError::new(e.to_string()))?;
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&journal_path)
                .map_err(|e| ApplyError::new(e.to_string()))?;
            writeln!(f, "{receipt}").map_err(|e| ApplyError::new(e.to_string()))?;
        } else {
            not_published.insert(id.clone());
        }
        report.transactions.push((id, outcome));
    }
    Ok(report)
}

fn receipts_base(_journal: &Path, repo: &Repo) -> AResult<String> {
    repo.head()
}

/// Evidence records per address.
type EvidenceMap = BTreeMap<String, Vec<Value>>;

struct TxRunner<'a> {
    env: &'a Env<'a>,
    options: &'a ApplyOptions,
    plan_id: &'a str,
    revision: u64,
    tx: &'a Value,
    id: &'a str,
}

/// What the coding part of an attempt produced.
#[derive(Default)]
struct Claims {
    /// address -> created outputs claimed so far.
    created: BTreeMap<String, BTreeMap<String, Output>>,
    /// requirement member -> test bindings.
    tests: BTreeMap<String, Vec<(String, String)>>,
    /// What the last result named that resolved to nothing; told to the next attempt.
    notes: Vec<String>,
}

impl TxRunner<'_> {
    fn dispatch(&self, n: u32) -> String {
        format!("{}/r{}/{}/{n}", self.plan_id, self.revision, self.id)
    }

    /// The transaction's protocol area inside the candidate.
    fn tx_area(&self, c: &Candidate) -> PathBuf {
        c.dir.join(self.tx_area_rel())
    }

    fn tx_area_rel(&self) -> String {
        format!(".spec/run/{}/r{}/{}", self.plan_id, self.revision, self.id)
    }

    /// Writes the dispatch's MCP context and client configuration; returns the latter.
    #[allow(clippy::too_many_arguments)]
    fn write_mcp_context(
        &self,
        c: &Candidate,
        dispatch: &str,
        order: &Value,
        base: &str,
        entries: &[&Value],
        records: &BTreeMap<String, StateRecord>,
        reqs: &[Vec<Requirement>],
        claims: &Claims,
        protected: &[String],
    ) -> AResult<PathBuf> {
        let env = self.env;
        let area_rel = format!(".spec/run/{dispatch}");
        let area = c.dir.join(&area_rel);
        let decls = declarations(env.graph);
        // Resources keep the pre-edit bytes of each manifest item, so they match its digest.
        let mut resources = BTreeMap::new();
        for item in order["context"]["manifest"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(hash)) = (item["resource"].as_str(), item["hash"].as_str()) else {
                continue;
            };
            let output = item["id"]
                .as_str()
                .unwrap_or_default()
                .trim_start_matches("code:");
            let path = order["subjects"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|s| s["envelope"]["write"].as_array().into_iter().flatten())
                .find(|w| w["output"] == output)
                .and_then(|w| w["path"].as_str());
            if let Some(path) = path {
                if let Ok(bytes) = fs::read(c.dir.join(path)) {
                    let rel = format!("{area_rel}/resources/{}", &id[id.len() - 12..]);
                    let _ = fs::create_dir_all(c.dir.join(&rel).parent().unwrap());
                    let _ = fs::write(c.dir.join(&rel), bytes);
                    resources.insert(id.to_string(), (rel, hash.to_string()));
                }
            }
        }
        let mut subjects = Vec::new();
        let mut checks = Vec::new();
        for (e, bound) in entries.iter().zip(reqs) {
            let address = e["address"].as_str().unwrap_or_default();
            let mut own = BTreeMap::new();
            if let Some(r) = record_for(records, e) {
                own.insert(address.to_string(), r.clone());
            }
            let outputs = self.outputs_of(address, &own, claims);
            let decl = decls.get(address).copied().unwrap_or(&Value::Null);
            let mut expected: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
            for r in bound {
                for (o, case) in &r.tests {
                    expected
                        .entry(o.clone())
                        .or_default()
                        .entry(case.clone())
                        .or_default();
                }
            }
            let exports: Vec<String> = kind_body(env.project, decl)
                .into_iter()
                .flat_map(|b| b.members.iter())
                .filter(|m| m.name.text == "exports")
                .flat_map(|m| match &m.value {
                    MemberValue::Section(s) => {
                        s.members.iter().map(|e| e.name.text.clone()).collect()
                    }
                    _ => vec![],
                })
                .collect();
            let mut obligations = vec!["verify".to_string()];
            if bound.iter().any(|r| r.policy.review || r.policy.tests) {
                obligations.push("review".into());
            }
            subjects.push(crate::mcp::McpSubject {
                address: address.to_string(),
                kind: decl["kind"].as_str().unwrap_or_default().to_string(),
                obligations,
                outputs,
                exports,
                roots: env
                    .mats
                    .get(address)
                    .map(|m| m.roots.clone())
                    .unwrap_or_default(),
                requirements: bound.iter().map(|r| r.member.clone()).collect(),
                expected_cases: expected,
            });
            checks.extend(
                resolve_checks(env.project, decl)
                    .into_iter()
                    .map(|d| (address.to_string(), d)),
            );
        }
        let context = crate::mcp::McpContext {
            dispatch: dispatch.to_string(),
            order: format!("{area_rel}/order.json"),
            graph: format!("{}/graph.json", self.tx_area_rel()),
            logs: format!("{}/logs", self.tx_area_rel()),
            base: base.to_string(),
            resources,
            subjects,
            protected: protected.to_vec(),
            checks,
            profile: self.options.profile.clone(),
            check_timeout_seconds: self.options.check_timeout.as_secs(),
        };
        write_json(
            &area.join("mcp.json"),
            &serde_json::to_value(&context).expect("serialisable"),
        )?;
        let mut command = env.spec_command.clone();
        command.push(dispatch.to_string());
        let config = json!({ "mcpServers": { "spec": { "type": "stdio", "command": command[0], "args": command[1..] } } });
        let path = area.join("mcp-config.json");
        write_json(&path, &config)?;
        Ok(path)
    }

    fn entries(&self) -> Vec<&Value> {
        self.env.plan["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["transaction"] == json!(self.id))
            .collect()
    }

    fn run(&self) -> AResult<Outcome> {
        let repo = self.env.repo;
        let base = repo.head()?;
        let name = format!("{}-r{}-{}", self.plan_id, self.revision, self.id);
        let candidate = repo.candidate(&name, &base)?;
        let result = self.run_in(&candidate);
        // Keep the protocol area (orders, results, verdicts, generator inputs) for audit.
        copy_tree(
            &candidate.dir.join(".spec/run").join(self.plan_id),
            &repo.root.join(".spec/run").join(self.plan_id),
        );
        match &result {
            Ok(Outcome::Published { .. }) => repo.remove_candidate(&candidate),
            _ => {
                let patch_path = repo.root.join(format!(
                    ".spec/run/{}/r{}/{}/failed.patch",
                    self.plan_id, self.revision, self.id
                ));
                let _ = fs::create_dir_all(patch_path.parent().unwrap());
                let _ = fs::write(&patch_path, candidate.patch());
                repo.remove_candidate(&candidate);
            }
        }
        result
    }

    fn run_in(&self, c: &Candidate) -> AResult<Outcome> {
        let env = self.env;
        let decls = declarations(env.graph);
        let store = Store::new(&c.dir);
        let records = store.records()?;
        let entries = self.entries();
        let changed: Vec<&Value> = entries
            .iter()
            .copied()
            .filter(|e| e["change"] != "none")
            .collect();
        let obligated: Vec<String> = self.tx["obligations"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(a, _)| a.clone())
            .collect();
        let record_of = |e: &Value| -> Option<&StateRecord> {
            let from = e["moved_from"].as_str().or_else(|| e["address"].as_str())?;
            records.get(from)
        };
        let mut dispatch_n = 0u32;
        let mut next_dispatch = || {
            dispatch_n += 1;
            self.dispatch(dispatch_n)
        };
        write_json(&self.tx_area(c).join("graph.json"), env.graph)?;
        let input_snapshot = c.freeze(&format!("spec: {} input", self.id))?;

        // Deletions and generators run once; generator failures are never retried by agents.
        for e in &changed {
            if e["change"] == "delete" {
                if let Some(r) = record_of(e) {
                    for o in r.outputs.values() {
                        let _ = fs::remove_file(c.dir.join(&o.path));
                    }
                }
            }
        }
        // Moved outputs (a moved declaration, a route change) move before any agent runs,
        // so an update edits them in place instead of writing them again.
        for e in changed.iter().filter(|e| e["change"] == "update") {
            let address = e["address"].as_str().unwrap_or_default();
            let (Some(r), Some(m)) = (record_of(e), env.mats.get(address)) else {
                continue;
            };
            for (_, from, to) in orders::output_moves(r, m) {
                let (src, dst) = (c.dir.join(&from), c.dir.join(&to));
                if src.exists() && !dst.exists() {
                    fs::create_dir_all(dst.parent().expect("a file path"))
                        .and_then(|_| fs::rename(&src, &dst))
                        .map_err(|err| ApplyError::new(format!("moving {from} to {to}: {err}")))?;
                }
            }
        }
        let mut by_generator: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        for e in changed
            .iter()
            .copied()
            .filter(|e| e["strategy"] == "generator")
        {
            let address = e["address"].as_str().unwrap_or_default();
            if let Some(g) = env.mats.get(address).and_then(|m| m.generator.clone()) {
                by_generator.entry(g).or_default().push(e);
            }
        }
        for (generator, group) in &by_generator {
            let dispatch = next_dispatch();
            if let Err(why) =
                self.run_generator(c, &dispatch, &input_snapshot, generator, group, &records)
            {
                return Ok(Outcome::Failed {
                    reason: format!("generator {generator}: {why}"),
                });
            }
        }

        let agent_entries: Vec<&Value> = changed
            .iter()
            .copied()
            .filter(|e| matches!(e["strategy"].as_str(), Some("agent" | "replace")))
            .collect();
        if !agent_entries.is_empty() && self.options.agent.is_none() {
            return Ok(Outcome::Failed {
                reason: "the plan needs an agent and no --agent command was given".into(),
            });
        }
        let base_reqs: BTreeMap<String, Vec<Requirement>> = entries
            .iter()
            .filter_map(|e| e["address"].as_str())
            .chain(obligated.iter().map(String::as_str))
            .filter_map(|a| Some((a.to_string(), requirements(env.graph, decls.get(a)?).ok()?)))
            .collect();
        let max = self.options.max_attempts.max(1);
        let mut previous: Vec<Value> = Vec::new();
        let mut claims = Claims::default();
        for (address, r) in &records {
            // Created outputs at their current paths: a moved root takes them along.
            let moves: BTreeMap<String, String> = env
                .mats
                .get(address)
                .map(|m| orders::output_moves(r, m))
                .unwrap_or_default()
                .into_iter()
                .map(|(k, _, to)| (k, to))
                .collect();
            let created: BTreeMap<String, Output> = r
                .outputs
                .iter()
                .filter(|(_, o)| o.origin == "created")
                .map(|(k, o)| {
                    let mut o = o.clone();
                    if let Some(to) = moves.get(k) {
                        o.path = to.clone();
                    }
                    (k.clone(), o)
                })
                .collect();
            if !created.is_empty() {
                claims.created.insert(address.clone(), created);
            }
        }
        // An acquired declaration's created outputs and proposed test bindings come from
        // the plan, since no agent claims them.
        for e in changed.iter().filter(|e| e["strategy"] == "acquire") {
            let address = e["address"].as_str().unwrap_or_default();
            let Some(m) = env.mats.get(address) else {
                continue;
            };
            for (key, o) in e["acquire"]["created"].as_object().into_iter().flatten() {
                let root = o["root"].as_str().unwrap_or_default();
                let rel = o["relative_path"].as_str().unwrap_or_default();
                let Some(dir) = m.roots.get(root) else {
                    continue;
                };
                claims
                    .created
                    .entry(address.to_string())
                    .or_default()
                    .insert(
                        key.clone(),
                        Output {
                            origin: "created".into(),
                            path: format!("{dir}{rel}"),
                            bytes: String::new(),
                            root: Some(root.into()),
                            relative_path: Some(rel.into()),
                        },
                    );
            }
            for (member, ts) in e["acquire"]["tests"].as_object().into_iter().flatten() {
                let tests = ts
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| {
                        Some((
                            t["output"].as_str()?.to_string(),
                            t["case"].as_str()?.to_string(),
                        ))
                    })
                    .collect();
                claims.tests.insert(member.clone(), tests);
            }
        }
        // Obligated declarations a review found nonconforming join the transaction as
        // updates inside their contingent envelope (docs/spec.md, Apply, step 4).
        let mut escalated: Vec<Value> = Vec::new();
        let adopted = self.adopted_paths();
        let mut inferred: BTreeSet<String> = BTreeSet::new();
        for attempt in 1..=max {
            let mut failures: Vec<Value> = Vec::new();
            for m in std::mem::take(&mut inferred) {
                claims.tests.remove(&m);
            }
            let coding: Vec<&Value> = agent_entries
                .iter()
                .copied()
                .chain(escalated.iter())
                .collect();
            let changed_now: Vec<&Value> =
                changed.iter().copied().chain(escalated.iter()).collect();
            // One dispatch for the whole transaction: the agent sees every subject at once.
            if !coding.is_empty() {
                let dispatch = next_dispatch();
                match self.run_agent(
                    c,
                    &dispatch,
                    &input_snapshot,
                    attempt,
                    max,
                    &previous,
                    &coding,
                    &records,
                    &base_reqs,
                    &adopted,
                    &mut claims,
                ) {
                    Ok(None) => {}
                    Ok(Some(f)) => failures.push(f),
                    Err(AgentStop::Blocked(why)) => {
                        return Ok(Outcome::Failed {
                            reason: format!("blocked: {why}"),
                        })
                    }
                    Err(AgentStop::Error(e)) => return Err(e),
                }
            }
            let snapshot = c.freeze(&format!("spec: {} attempt {attempt}", self.id))?;
            // Protected files, created outputs and the declarations the change reached.
            let mut touched: BTreeSet<String> = BTreeSet::new();
            if failures.is_empty() {
                match self.check_scope(
                    c,
                    &snapshot,
                    attempt,
                    &changed_now,
                    &obligated,
                    &records,
                    &mut claims,
                    &adopted,
                )? {
                    Ok(t) => touched = t,
                    Err(f) => failures.push(f),
                }
            }
            let mut reqs = base_reqs.clone();
            for a in &touched {
                if let Some(list) = decls
                    .get(a.as_str())
                    .and_then(|d| requirements(env.graph, d).ok())
                {
                    reqs.insert(a.clone(), list);
                }
            }
            let checked: Vec<String> = obligated
                .iter()
                .cloned()
                .chain(touched.iter().cloned())
                .collect();
            let mut runs = Vec::new();
            if failures.is_empty() {
                let (r, f, failed) = self.run_checks(
                    c, &snapshot, attempt, &entries, &checked, &reqs, &claims, &records,
                )?;
                runs = r;
                failures.extend(f);
                inferred = infer_tests(&reqs, &runs, &mut claims);
                // A check that fails for an obligated declaration confirms a violation:
                // it escalates to an update inside its contingent envelope.
                let escalate: Vec<&String> = failed
                    .iter()
                    .filter(|a| {
                        obligated.contains(a)
                            && !escalated.iter().any(|e| e["address"] == json!(a))
                            && decls
                                .get(a.as_str())
                                .is_some_and(|d| d["lifecycle"] != "frozen")
                    })
                    .collect();
                if self.options.agent.is_some() && attempt < max {
                    for a in escalate {
                        escalated.push(json!({
                            "address": a, "identity": "same", "change": "update",
                            "strategy": "agent", "causes": ["escalation"], "transaction": self.id,
                        }));
                    }
                }
            }
            if failures
                .iter()
                .any(|f| f["classification"] == "infrastructure")
            {
                let why: Vec<&str> = failures
                    .iter()
                    .filter_map(|f| f["excerpt"].as_str())
                    .collect();
                return Ok(Outcome::Failed {
                    reason: format!("infrastructure: {}", why.join("; ")),
                });
            }
            let mut verdicts = BTreeMap::new();
            if failures.is_empty() {
                match self.review(
                    c,
                    &snapshot,
                    &mut next_dispatch,
                    &entries,
                    &checked,
                    &touched,
                    &reqs,
                    &claims,
                    &runs,
                    &records,
                ) {
                    Err(e) => {
                        return Ok(Outcome::Failed {
                            reason: format!("review: {e}"),
                        })
                    }
                    Ok(Ok(v)) => verdicts = v,
                    Ok(Err(nonconforming)) => {
                        let mut escalate: BTreeSet<String> = BTreeSet::new();
                        for (a, _) in &nonconforming {
                            let obliged = obligated.contains(a)
                                && !escalated.iter().any(|e| e["address"] == json!(a));
                            if obliged {
                                escalate.insert(a.clone());
                            }
                        }
                        let frozen = escalate.iter().any(|a| {
                            decls
                                .get(a.as_str())
                                .is_some_and(|d| d["lifecycle"] == "frozen")
                        });
                        if !escalate.is_empty()
                            && (self.options.agent.is_none() || frozen || attempt == max)
                        {
                            return Ok(Outcome::Failed { reason: format!("review found nonconformance in an obligated declaration it cannot update here ({}), which needs an amended plan: {}", if frozen { "frozen" } else if self.options.agent.is_none() { "no agent" } else { "no attempt left" }, nonconforming.iter().map(|(a, r)| format!("{a}: {r}")).collect::<Vec<_>>().join("; ")) });
                        }
                        for a in &escalate {
                            escalated.push(json!({
                                "address": a, "identity": "same", "change": "update",
                                "strategy": "agent", "causes": ["escalation"], "transaction": self.id,
                            }));
                        }
                        for (a, reason) in nonconforming {
                            failures.push(failure(
                                attempt,
                                &snapshot,
                                "review_nonconforming",
                                "implementation",
                                &format!("{a}: {reason}"),
                                None,
                            ));
                        }
                    }
                }
            }
            if failures.is_empty() {
                // A requirement whose policy is not met (tests the reviewer finds do not
                // cover it, say) is a test failure: a subject being coded gets another
                // attempt with the finding, and an obligated declaration escalates like a
                // nonconforming one. Publishing reports it once no attempt is left.
                let (_, unmet) =
                    self.evidence(&entries, &reqs, &claims, &runs, &verdicts, &records)?;
                let coded = |a: &String| {
                    agent_entries.iter().any(|e| e["address"] == json!(a))
                        || escalated.iter().any(|e| e["address"] == json!(a))
                };
                let escalate: BTreeSet<String> = unmet
                    .iter()
                    .filter(|(a, _)| {
                        obligated.contains(a)
                            && !coded(a)
                            && decls
                                .get(a.as_str())
                                .is_some_and(|d| d["lifecycle"] != "frozen")
                    })
                    .map(|(a, _)| a.clone())
                    .collect();
                let retry: Vec<&(String, String)> = unmet
                    .iter()
                    .filter(|(a, _)| coded(a) || escalate.contains(a))
                    .collect();
                if retry.is_empty() || self.options.agent.is_none() || attempt == max {
                    return self.publish(
                        c, &entries, &checked, &reqs, &claims, &runs, &verdicts, &records,
                    );
                }
                for a in &escalate {
                    escalated.push(json!({
                        "address": a, "identity": "same", "change": "update",
                        "strategy": "agent", "causes": ["escalation"], "transaction": self.id,
                    }));
                }
                for (a, m) in retry {
                    let member = m.split(' ').next().unwrap_or_default();
                    let mut why = verdicts
                        .get(member)
                        .filter(|v| v.coverage.as_deref() == Some("insufficient"))
                        .map(|v| {
                            format!(
                                "; the reviewer found its tests insufficient: {}",
                                v.coverage_reason
                            )
                        })
                        .unwrap_or_default();
                    let unbound = claims.tests.get(member).is_none_or(Vec::is_empty)
                        && records
                            .get(a.as_str())
                            .is_none_or(|rec| previous_tests(rec, member).is_empty());
                    if unbound {
                        why.push_str(&format!(
                            "; no test covers it yet: a test case tagged `{}` in its test output does",
                            crate::names::key(member)
                        ));
                        if !claims.notes.is_empty() {
                            why.push_str(&format!(" (your result: {})", claims.notes.join("; ")));
                        }
                    }
                    failures.push(failure(
                        attempt,
                        &snapshot,
                        "policy_unmet",
                        "test",
                        &format!("{a}: policy not met: {m}{why}"),
                        None,
                    ));
                }
            }
            if (agent_entries.is_empty() && escalated.is_empty()) || attempt == max {
                let why: Vec<&str> = failures
                    .iter()
                    .filter_map(|f| f["excerpt"].as_str())
                    .collect();
                return Ok(Outcome::Failed {
                    reason: format!("attempt {attempt} of {max} failed: {}", why.join("; ")),
                });
            }
            previous.extend(failures);
        }
        unreachable!("the loop returns on its last attempt")
    }

    fn run_generator(
        &self,
        c: &Candidate,
        dispatch: &str,
        snapshot: &str,
        generator: &str,
        group: &[&Value],
        records: &BTreeMap<String, StateRecord>,
    ) -> Result<(), String> {
        let env = self.env;
        let decls = declarations(env.graph);
        let first = decls
            .get(group[0]["address"].as_str().unwrap_or_default())
            .copied()
            .ok_or("unknown subject")?;
        let package = kind_package(env.project, first).ok_or("no package")?;
        let run = generator_run(package, generator)
            .ok_or_else(|| format!("package {} has no generator {generator}", package.id))?;
        let staging = format!(".spec/run/{dispatch}/staging/");
        let subjects: Vec<(String, Option<&StateRecord>)> = group
            .iter()
            .map(|e| {
                let a = e["address"].as_str().unwrap_or_default().to_string();
                let r = records.get(e["moved_from"].as_str().unwrap_or(&a));
                (a, r)
            })
            .collect();
        let generation_id = format!("{}-{}", self.plan_id, self.id);
        let input = orders::generator_input(
            env,
            dispatch,
            snapshot,
            &subjects,
            &c.dir,
            &staging,
            &generation_id,
        )
        .map_err(|e| e.message)?;
        fs::create_dir_all(c.dir.join(&staging)).map_err(|e| e.to_string())?;
        write_json(
            &c.dir.join(format!(".spec/run/{dispatch}/input.json")),
            &input,
        )
        .map_err(|e| e.message)?;
        let mut argv = run.clone();
        for a in argv.iter_mut().take(2) {
            if let Some(rest) = a.strip_prefix("./") {
                *a = package.root.join(rest).to_string_lossy().into_owned();
            }
        }
        let out = crate::process::run(
            crate::process::Program::Argv(&argv),
            &c.dir,
            &input.to_string(),
            &[(
                "SPEC_PACKAGE_ROOT",
                package.root.to_string_lossy().into_owned(),
            )],
            self.options.check_timeout,
            true,
        )
        .map_err(|e| e.message)?;
        if out.timed_out {
            return Err(format!(
                "timed out after {}s",
                self.options.check_timeout.as_secs()
            ));
        }
        if out.code != 0 {
            return Err(format!(
                "exited {}: {}",
                out.code,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let output: Value =
            serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid output: {e}"))?;
        if output["protocol"] != "sdlang.generator/1"
            || output["dispatch"] != json!(dispatch)
            || output["order_hash"] != input["binding"]["order_hash"]
        {
            return Err("output does not echo the protocol, dispatch and order_hash".into());
        }
        match output["status"].as_str() {
            Some("written") => {}
            Some(other) => {
                return Err(format!(
                    "{other}: {}",
                    output["reason"]
                        .as_str()
                        .or(output["questions"].as_str())
                        .unwrap_or("")
                ))
            }
            None => return Err("output has no status".into()),
        }
        let writable: BTreeMap<String, String> = input["envelope"]["write"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| {
                Some((
                    w["output"].as_str()?.to_string(),
                    w["path"].as_str()?.to_string(),
                ))
            })
            .collect();
        let retained: BTreeMap<String, String> = input["envelope"]["retain"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| {
                Some((
                    format!("retain:{}", r["root"].as_str()?),
                    r["path"].as_str()?.to_string(),
                ))
            })
            .collect();
        for f in output["files"].as_array().into_iter().flatten() {
            let target = f["target"].as_str().unwrap_or_default();
            let rel = f["relative_path"].as_str().unwrap_or_default();
            let allowed = writable.get(target).is_some_and(|p| p == rel)
                || retained
                    .get(target)
                    .is_some_and(|root| rel.starts_with(root.as_str()) && !rel.contains(".."));
            if !allowed {
                return Err(format!("wrote {rel} for {target}, outside its envelope"));
            }
            let dest = c.dir.join(rel);
            match f["op"].as_str() {
                Some("create" | "update") => {
                    let bytes = fs::read(c.dir.join(&staging).join(rel))
                        .map_err(|e| format!("staging/{rel}: {e}"))?;
                    if let Some(want) = f["digest"].as_str() {
                        if sdlang_canon::digest_bytes(&bytes) != want {
                            return Err(format!("{rel}: digest does not match the staged bytes"));
                        }
                    }
                    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
                    fs::write(&dest, bytes).map_err(|e| e.to_string())?;
                }
                Some("delete") => {
                    let _ = fs::remove_file(&dest);
                }
                other => return Err(format!("unsupported file op {other:?}")),
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_agent(
        &self,
        c: &Candidate,
        dispatch: &str,
        input_snapshot: &str,
        attempt: u32,
        max: u32,
        previous: &[Value],
        entries: &[&Value],
        records: &BTreeMap<String, StateRecord>,
        reqs: &BTreeMap<String, Vec<Requirement>>,
        adopted: &BTreeSet<String>,
        claims: &mut Claims,
    ) -> Result<Option<Value>, AgentStop> {
        let env = self.env;
        let decls = declarations(env.graph);
        let address_of = |e: &Value| e["address"].as_str().unwrap_or_default().to_string();
        let label = entries
            .iter()
            .map(|e| address_of(e))
            .collect::<Vec<_>>()
            .join(", ");
        let mut checks: Vec<Value> = Vec::new();
        let mut bound_all: Vec<Vec<Requirement>> = Vec::new();
        for e in entries {
            let address = address_of(e);
            for d in decls
                .get(address.as_str())
                .map(|d| resolve_checks(env.project, d))
                .unwrap_or_default()
            {
                if !checks.iter().any(|x| x["id"] == json!(d.id)) {
                    checks.push(json!({ "id": d.id, "definition_hash": d.definition_hash, "self_check": true }));
                }
            }
            let record = record_for(records, e);
            let mut bound = reqs.get(&address).cloned().unwrap_or_default();
            for r in &mut bound {
                if let Some(t) = claims.tests.get(&r.member) {
                    r.tests = t.clone();
                } else if let Some(rec) = record {
                    r.tests = previous_tests(rec, &r.member);
                }
            }
            bound_all.push(bound);
        }
        let subjects: Vec<Subject> = entries
            .iter()
            .zip(&bound_all)
            .map(|(e, b)| Subject {
                address: e["address"].as_str().unwrap_or_default(),
                entry: e,
                record: record_for(records, e),
                requirements: b,
            })
            .collect();
        let order = orders::work_order(
            env,
            dispatch,
            self.id,
            &c.base,
            input_snapshot,
            (attempt, max, previous),
            &subjects,
            &checks,
            &c.dir,
        )
        .map_err(AgentStop::Error)?;
        let area = c.dir.join(format!(".spec/run/{dispatch}"));
        let order_path = area.join("order.json");
        write_json(&order_path, &order).map_err(AgentStop::Error)?;
        let result_path = c
            .dir
            .join(order["result_path"].as_str().unwrap_or_default());
        let command = self.options.agent.clone().unwrap_or_default();
        let protected: Vec<String> = adopted.iter().cloned().collect();
        let guidance = skills(
            env.project,
            entries
                .iter()
                .filter_map(|e| decls.get(e["address"].as_str()?).copied()),
        );
        let markdown = orders::render_work_order(&order, Some(&c.dir), &protected, &guidance);
        let _ = fs::write(area.join("order.md"), &markdown);
        let mcp_config = self
            .write_mcp_context(
                c,
                dispatch,
                &order,
                input_snapshot,
                entries,
                records,
                &bound_all,
                claims,
                &protected,
            )
            .map_err(AgentStop::Error)?;
        let finished = run_command(
            &command,
            &c.dir,
            &markdown,
            &[
                ("SPEC_WORK_ORDER", order_path.to_string_lossy().into_owned()),
                ("SPEC_RESULT", result_path.to_string_lossy().into_owned()),
                ("SPEC_DISPATCH", dispatch.to_string()),
                ("SPEC_MCP_CONFIG", mcp_config.to_string_lossy().into_owned()),
            ],
            self.options.agent_timeout,
        )
        .map_err(AgentStop::Error)?;
        if finished.timed_out && !result_path.exists() {
            return Ok(Some(failure(
                attempt,
                input_snapshot,
                "timeout",
                "protocol",
                &format!(
                    "{label}: the agent did not finish within {}s",
                    self.options.agent_timeout.as_secs()
                ),
                None,
            )));
        }
        // The result is optional (docs/protocol.md, Result): without one, the candidate is the
        // claim, and created outputs and test bindings are inferred from it.
        let snapshot = input_snapshot;
        let result = read_json(&result_path).unwrap_or_else(|| json!({}));
        let echoes = |field: &str, want: &Value| result[field].is_null() || result[field] == *want;
        if !echoes("dispatch", &json!(dispatch))
            || !echoes("order_hash", &order["binding"]["order_hash"])
        {
            return Ok(Some(failure(
                attempt,
                snapshot,
                "binding_mismatch",
                "protocol",
                &format!("{label}: the result names another order's dispatch or order_hash"),
                None,
            )));
        }
        if result["status"] == "blocked" {
            return Err(AgentStop::Blocked(format!(
                "{label}: {} ({})",
                result["blocked"]["kind"].as_str().unwrap_or("unspecified"),
                result["summary"].as_str().unwrap_or("")
            )));
        }
        let mut notes: Vec<String> = Vec::new();
        let addresses: Vec<String> = entries.iter().map(|e| address_of(e)).collect();
        // Output claims: an owner may be left out when the subject or root says which.
        for o in result["outputs"].as_array().into_iter().flatten() {
            let op = o["op"].as_str().unwrap_or("create");
            let root = o["root"].as_str().unwrap_or_default();
            let owner = match o["owner"].as_str() {
                Some(owner) => addresses.iter().find(|a| a.as_str() == owner).cloned(),
                None if addresses.len() == 1 => addresses.first().cloned(),
                None => {
                    let with_root: Vec<&String> = addresses
                        .iter()
                        .filter(|a| {
                            env.mats
                                .get(a.as_str())
                                .is_some_and(|m| m.roots.contains_key(root))
                        })
                        .collect();
                    (with_root.len() == 1).then(|| with_root[0].clone())
                }
            };
            let (Some(owner), Some(key)) = (owner, o["key"].as_str()) else {
                notes.push(format!(
                    "an output claim names no subject of this order: {o}"
                ));
                continue;
            };
            let created = claims.created.entry(owner.clone()).or_default();
            if op == "remove" {
                created.remove(key);
                continue;
            }
            let rel = o["relative_path"].as_str().unwrap_or_default();
            let Some(root_path) = env.mats.get(&owner).and_then(|m| m.roots.get(root)) else {
                notes.push(format!("{owner} has no creation root {root:?}"));
                continue;
            };
            if rel.is_empty() || rel.contains("..") || rel.starts_with('/') {
                notes.push(format!("{rel:?} is not a path under {root_path}"));
                continue;
            }
            let path = format!("{root_path}{rel}");
            if op == "create" && c_existed_at_base(c, &path) {
                notes.push(format!(
                    "{path} existed before this attempt, so it is not a new output"
                ));
                continue;
            }
            created.insert(
                key.to_string(),
                Output {
                    origin: "created".into(),
                    path,
                    bytes: String::new(),
                    root: Some(root.into()),
                    relative_path: Some(rel.into()),
                },
            );
        }
        // Test bindings, by full member or any unambiguous tail of it.
        let members: Vec<(String, String)> = addresses
            .iter()
            .flat_map(|a| {
                reqs.get(a)
                    .into_iter()
                    .flatten()
                    .map(move |r| (a.clone(), r.member.clone()))
            })
            .collect();
        for t in result["tests"].as_array().into_iter().flatten() {
            let name = t["subject"]["requirement"]
                .as_str()
                .or(t["requirement"].as_str())
                .unwrap_or_default();
            let target = t["subject"]["target"].as_str().or(t["target"].as_str());
            let candidates = members
                .iter()
                .filter(|(a, _)| target.is_none_or(|x| x == a))
                .map(|(_, m)| m.as_str());
            let member = match crate::names::resolve(name, candidates) {
                Ok(m) => m,
                Err(why) => {
                    notes.push(format!("requirement {why}"));
                    continue;
                }
            };
            let address = members
                .iter()
                .find(|(_, m)| *m == member)
                .map(|(a, _)| a.clone())
                .unwrap_or_default();
            let mut outputs: BTreeSet<String> = BTreeSet::new();
            if let Some(m) = env.mats.get(&address) {
                outputs.extend(m.slots.keys().map(|k| format!("{address}#outputs.{k}")));
            }
            if let Some(r) = records.get(&address) {
                outputs.extend(
                    r.outputs
                        .iter()
                        .filter(|(_, o)| o.origin == "created")
                        .map(|(k, _)| format!("{address}#outputs.{k}")),
                );
            }
            if let Some(created) = claims.created.get(&address) {
                outputs.extend(created.keys().map(|k| format!("{address}#outputs.{k}")));
            }
            let test_outputs: Vec<&str> = outputs
                .iter()
                .map(String::as_str)
                .filter(|m| crate::names::key(m).contains("test"))
                .collect();
            let mut tests = Vec::new();
            for x in t["tests"].as_array().into_iter().flatten() {
                let Some(case) = x["case"].as_str() else {
                    continue;
                };
                let output = match x["output"].as_str() {
                    Some(o) => crate::names::resolve(o, outputs.iter().map(String::as_str)),
                    None if test_outputs.len() == 1 => Ok(test_outputs[0].to_string()),
                    None => Err(format!(
                        "no output given for case {case:?}, and {address} has several"
                    )),
                };
                match output {
                    Ok(o) => tests.push((o, case.to_string())),
                    Err(why) => notes.push(format!("test output {why}")),
                }
            }
            if !tests.is_empty() {
                claims.tests.insert(member, tests);
            }
        }
        claims.notes = notes;
        Ok(None)
    }

    /// Paths adopted from a hand edit in this transaction (`sdc plan --adopt`): their bytes
    /// are the baseline, so no dispatch may change them.
    fn adopted_paths(&self) -> BTreeSet<String> {
        self.entries()
            .iter()
            .flat_map(|e| e["obligations"].as_array().into_iter().flatten())
            .filter_map(|o| {
                o["because"]
                    .as_str()?
                    .strip_prefix("adopt:")
                    .map(String::from)
            })
            .collect()
    }

    /// The candidate's changes against what may not change and what they reach (docs/spec.md,
    /// Ownership and drift). The agent may change any file the change needs. The spec, its
    /// lock, packages and state, adopted files and frozen declarations' files are protected.
    /// A new file under a creation root becomes a created output of that root's declaration
    /// unless a result claimed it. A changed file owned by a declaration outside the
    /// transaction makes it touched: it is checked, reviewed and recorded with the
    /// transaction. Returns the touched declarations, or the failure.
    #[allow(clippy::too_many_arguments)]
    fn check_scope(
        &self,
        c: &Candidate,
        snapshot: &str,
        attempt: u32,
        changed: &[&Value],
        obligated: &[String],
        records: &BTreeMap<String, StateRecord>,
        claims: &mut Claims,
        adopted: &BTreeSet<String>,
    ) -> AResult<Result<BTreeSet<String>, Value>> {
        let env = self.env;
        let decls = declarations(env.graph);
        let in_tx: BTreeSet<&str> = changed
            .iter()
            .filter_map(|e| e["address"].as_str())
            .chain(obligated.iter().map(String::as_str))
            .collect();
        let deleting: BTreeSet<&str> = changed
            .iter()
            .filter(|e| e["change"] == "delete")
            .filter_map(|e| e["address"].as_str())
            .collect();
        let mut owner: BTreeMap<String, String> = BTreeMap::new();
        for (a, r) in records {
            for o in r.outputs.values() {
                owner.insert(o.path.clone(), a.clone());
            }
        }
        for (a, m) in env.mats {
            for p in m.slots.values() {
                owner.insert(p.clone(), a.clone());
            }
        }
        for (a, created) in &claims.created {
            for o in created.values() {
                owner.insert(o.path.clone(), a.clone());
            }
        }
        let mut violations = Vec::new();
        let mut touched = BTreeSet::new();
        for change in c.diff_from_base()? {
            let path = change.path();
            if path == "sd.lock"
                || path.starts_with("spec/")
                || path.starts_with("spec-packages/")
                || (path.starts_with(".spec/") && !path.starts_with(".spec/run/"))
            {
                violations.push(format!(
                    "{path} (the spec, its lock, packages and state change only through spec)"
                ));
                continue;
            }
            if adopted.contains(path) {
                violations.push(format!(
                    "{path} (adopted from a hand edit, so its bytes are the baseline)"
                ));
                continue;
            }
            if matches!(change, Change::Added(_)) && !owner.contains_key(path) {
                if let Some((a, root, dir)) = creation_root(env.mats, path) {
                    let rel = path[dir.len()..].to_string();
                    let created = claims.created.entry(a.clone()).or_default();
                    let key = new_key(&rel, created, env.mats.get(&a));
                    created.insert(
                        key,
                        Output {
                            origin: "created".into(),
                            path: path.to_string(),
                            bytes: String::new(),
                            root: Some(root),
                            relative_path: Some(rel),
                        },
                    );
                    owner.insert(path.to_string(), a);
                }
            }
            let Some(a) = owner.get(path) else {
                continue;
            };
            if in_tx.contains(a.as_str())
                || deleting.contains(a.as_str())
                || !decls.contains_key(a.as_str())
            {
                continue;
            }
            if decls
                .get(a.as_str())
                .is_some_and(|d| d["lifecycle"] == "frozen")
            {
                violations.push(format!("{path} (owned by {a}, which is frozen)"));
                continue;
            }
            touched.insert(a.clone());
        }
        // A created output whose file is gone is no longer an output.
        for created in claims.created.values_mut() {
            created.retain(|_, o| c.dir.join(&o.path).exists());
        }
        if violations.is_empty() {
            return Ok(Ok(touched));
        }
        Ok(Err(failure(
            attempt,
            snapshot,
            "protected_changed",
            "implementation",
            &format!(
                "changed files that must not change: {}",
                violations.join(", ")
            ),
            None,
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn run_checks(
        &self,
        c: &Candidate,
        snapshot: &str,
        attempt: u32,
        entries: &[&Value],
        obligated: &[String],
        reqs: &BTreeMap<String, Vec<Requirement>>,
        claims: &Claims,
        records: &BTreeMap<String, StateRecord>,
    ) -> AResult<(Vec<CheckRun>, Vec<Value>, BTreeSet<String>)> {
        let env = self.env;
        let decls = declarations(env.graph);
        let mut failed_subjects: BTreeSet<String> = BTreeSet::new();
        let area = c.dir.join(format!(
            ".spec/run/{}/r{}/{}",
            self.plan_id, self.revision, self.id
        ));
        let graph_path = area.join("graph.json");
        let scratch = area.join("scratch");
        let _ = fs::create_dir_all(&scratch);
        let mut subjects: Vec<String> = entries
            .iter()
            .filter(|e| e["change"] != "delete")
            .filter_map(|e| e["address"].as_str().map(String::from))
            .collect();
        for a in obligated {
            if !subjects.contains(a) {
                subjects.push(a.clone());
            }
        }
        let mut runs = Vec::new();
        let mut failures = Vec::new();
        let mut project_done: BTreeSet<String> = BTreeSet::new();
        for address in &subjects {
            let Some(decl) = decls.get(address.as_str()) else {
                continue;
            };
            let outputs = self.outputs_of(address, records, claims);
            let acquired = entries
                .iter()
                .any(|e| e["address"] == json!(address) && e["strategy"] == "acquire");
            // Acquisition changes no code and claims nothing new: build checks only.
            for def in resolve_checks(env.project, decl)
                .into_iter()
                .filter(|d| !acquired || d.purpose == "build")
            {
                let mut plan_runs: Vec<PlannedRun> = Vec::new();
                match (def.purpose.as_str(), def.scope.as_str()) {
                    (_, "project") => {
                        if project_done.insert(def.id.clone()) {
                            plan_runs.push((
                                format!("run:{}:{attempt}:{}", self.id, def.id),
                                None,
                                BTreeMap::new(),
                            ));
                        }
                    }
                    ("test", _) => {
                        for (member, path) in outputs.iter().filter(|(m, _)| {
                            m.rsplit('.').next().is_some_and(|k| k.contains("test"))
                        }) {
                            let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();
                            for r in reqs.get(address).into_iter().flatten() {
                                let tests = claims
                                    .tests
                                    .get(&r.member)
                                    .cloned()
                                    .or_else(|| {
                                        records
                                            .get(address)
                                            .map(|rec| previous_tests(rec, &r.member))
                                    })
                                    .unwrap_or_default();
                                for (o, case) in tests {
                                    if &o == member {
                                        expected.entry(case).or_default();
                                    }
                                }
                            }
                            plan_runs.push((
                                format!("run:{}:{attempt}:{}:{member}", self.id, def.id),
                                Some((member.clone(), path.clone())),
                                expected,
                            ));
                        }
                    }
                    _ => plan_runs.push((
                        format!("run:{}:{attempt}:{}:{address}", self.id, def.id),
                        None,
                        BTreeMap::new(),
                    )),
                }
                for (receipt, test_output, expected) in plan_runs {
                    let project_scope = def.scope == "project";
                    let inputs =
                        self.run_inputs(c, address, &def, project_scope, records, claims)?;
                    let ctx = RunContext {
                        receipt,
                        workdir: &c.dir,
                        scratch: &scratch,
                        graph_path: &graph_path,
                        snapshot,
                        profile: &self.options.profile,
                        subject: (!project_scope).then_some(address.as_str()),
                        outputs: if project_scope {
                            vec![]
                        } else {
                            outputs.clone()
                        },
                        test_output,
                        expected_cases: expected,
                        inputs,
                        timeout: self.options.check_timeout,
                    };
                    let mut run = run_check(&def, &ctx)?;
                    if c.changed_since_freeze()? {
                        c.restore()?;
                    }
                    // Infrastructure failures are rerun once and never cost a coding attempt.
                    if !run.passed && run.classification == Some("infrastructure") {
                        run = run_check(&def, &ctx)?;
                        if c.changed_since_freeze()? {
                            c.restore()?;
                        }
                    }
                    if !run.passed {
                        if !project_scope && run.classification != Some("infrastructure") {
                            failed_subjects.insert(address.clone());
                        }
                        let classification = run.classification.unwrap_or("implementation");
                        let mut f = failure(
                            attempt,
                            snapshot,
                            "check_failed",
                            classification,
                            &format!("{} on {}: {}", def.id, address, run.excerpt),
                            Some((&def.id, &def.definition_hash)),
                        );
                        if !run.log.is_empty() {
                            let hex =
                                sdlang_canon::digest_bytes(run.log.as_bytes())[7..19].to_string();
                            let logs = area.join("logs");
                            let _ = fs::create_dir_all(&logs);
                            let _ = fs::write(logs.join(format!("{hex}.log")), &run.log);
                            f["diagnostics"] = json!(format!("ctx:log:{hex}"));
                        }
                        failures.push(f);
                    }
                    runs.push(run);
                }
            }
        }
        Ok((runs, failures, failed_subjects))
    }

    /// `(member, path)` of a declaration's outputs in the candidate.
    fn outputs_of(
        &self,
        address: &str,
        records: &BTreeMap<String, StateRecord>,
        claims: &Claims,
    ) -> Vec<(String, String)> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        if let Some(m) = self.env.mats.get(address) {
            for (k, p) in &m.slots {
                out.insert(format!("{address}#outputs.{k}"), p.clone());
            }
        }
        let created = claims.created.get(address).cloned().or_else(|| {
            records.get(address).map(|r| {
                r.outputs
                    .iter()
                    .filter(|(_, o)| o.origin == "created")
                    .map(|(k, o)| (k.clone(), o.clone()))
                    .collect()
            })
        });
        for (k, o) in created.into_iter().flatten() {
            out.insert(format!("{address}#outputs.{k}"), o.path);
        }
        out.into_iter().collect()
    }

    /// The digest of a run's input closure (docs/spec.md, Currency).
    fn run_inputs(
        &self,
        c: &Candidate,
        address: &str,
        def: &CheckDef,
        project_scope: bool,
        records: &BTreeMap<String, StateRecord>,
        claims: &Claims,
    ) -> AResult<String> {
        let outputs_of = |a: &str| self.outputs_of(a, records, claims);
        let closure = Closure {
            dir: &c.dir,
            project: self.env.project,
            graph: self.env.graph,
            outputs_of: &outputs_of,
            profile: &self.options.profile,
        };
        run_input_digest(&closure, address, &def.definition_hash, project_scope)
    }

    #[allow(clippy::too_many_arguments)]
    fn review(
        &self,
        c: &Candidate,
        snapshot: &str,
        next_dispatch: &mut dyn FnMut() -> String,
        entries: &[&Value],
        obligated: &[String],
        touched: &BTreeSet<String>,
        reqs: &BTreeMap<String, Vec<Requirement>>,
        claims: &Claims,
        runs: &[CheckRun],
        records: &BTreeMap<String, StateRecord>,
    ) -> AResult<ReviewOutcome> {
        let env = self.env;
        let mut subjects: Vec<(String, ReviewSubject)> = Vec::new();
        let mut reused: BTreeMap<String, Verdict> = BTreeMap::new();
        let reviewed_obligations: BTreeSet<&str> = self.tx["obligations"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, o)| {
                o["kinds"]
                    .as_array()
                    .is_some_and(|k| k.contains(&json!("review")))
            })
            .map(|(a, _)| a.as_str())
            .collect();
        let coding: BTreeSet<&str> = entries
            .iter()
            .filter(|e| e["change"] != "none" && e["change"] != "delete")
            .filter(|e| e["strategy"] != "acquire")
            .filter_map(|e| e["address"].as_str())
            .collect();
        for (address, list) in reqs {
            let record = records.get(address);
            // A declaration the change reached outside its plan entry had its code changed,
            // so it is reviewed like one being coded.
            let needs_review = coding.contains(address.as_str())
                || reviewed_obligations.contains(address.as_str())
                || touched.contains(address);
            let code: Vec<String> = self
                .outputs_of(address, records, claims)
                .into_iter()
                .map(|(_, p)| p)
                .collect();
            for r in list {
                let mut r = r.clone();
                r.tests = claims
                    .tests
                    .get(&r.member)
                    .cloned()
                    .or_else(|| record.map(|rec| previous_tests(rec, &r.member)))
                    .unwrap_or_default();
                if !needs_review || !(r.policy.review || r.policy.tests) {
                    // A verify-only obligation keeps its coverage and conformance verdicts.
                    if let Some(v) = record.and_then(|rec| previous_verdict(rec, &r.member)) {
                        reused.insert(r.member.clone(), v);
                    }
                    if !obligated.contains(address)
                        || record
                            .and_then(|rec| previous_verdict(rec, &r.member))
                            .is_some()
                    {
                        continue;
                    }
                }
                let coverage = r.policy.tests.then(|| {
                    let tests: Vec<(String, String, String)> = r
                        .tests
                        .iter()
                        .map(|(o, case)| {
                            let oracle = runs
                                .iter()
                                .find(|x| x.output.as_deref() == Some(o.as_str()))
                                .and_then(|x| x.cases.get(case))
                                .map(|x| x.oracle.clone())
                                .unwrap_or_default();
                            (o.clone(), case.clone(), oracle)
                        })
                        .collect();
                    let receipts: Vec<String> = runs
                        .iter()
                        .filter(|x| {
                            r.tests
                                .iter()
                                .any(|(o, _)| x.output.as_deref() == Some(o.as_str()))
                        })
                        .map(|x| x.receipt.clone())
                        .collect();
                    (tests, receipts)
                });
                let context_hash = digest(&env.docs.declarations[address].context)?;
                subjects.push((
                    address.clone(),
                    ReviewSubject {
                        requirement: r,
                        context_hash,
                        previous_text: None,
                        code: code.clone(),
                        coverage,
                    },
                ));
            }
        }
        let mut verdicts = reused;
        if subjects.is_empty() {
            return Ok(Ok(verdicts));
        }
        let Some(command) = &self.options.reviewer else {
            return Ok(Ok(verdicts));
        };
        let addresses: BTreeSet<&String> = subjects.iter().map(|(a, _)| a).collect();
        let mut inherited = Vec::new();
        let mut references = Vec::new();
        for a in addresses {
            inherited.extend(orders::inherited(env, a, records.get(a))?);
            references.extend(orders::references(env, a, records.get(a))?);
        }
        let review_subjects: Vec<ReviewSubject> = subjects
            .iter()
            .map(|(_, s)| ReviewSubject {
                requirement: s.requirement.clone(),
                context_hash: s.context_hash.clone(),
                previous_text: s.previous_text.clone(),
                code: s.code.clone(),
                coverage: s.coverage.clone(),
            })
            .collect();
        let snapshot_digest = digest(&json!(snapshot))?;
        // An invalid verdict is rejected and the review asked once more, with the problem
        // stated; a second invalid verdict fails the transaction.
        let mut rejected: Option<String> = None;
        for _ in 0..2 {
            let dispatch = next_dispatch();
            let order = orders::review_order(
                env,
                &dispatch,
                &snapshot_digest,
                &review_subjects,
                (inherited.clone(), references.clone()),
            )?;
            let area = c.dir.join(format!(".spec/run/{dispatch}"));
            let order_path = area.join("order.json");
            write_json(&order_path, &order)?;
            let verdict_path = c
                .dir
                .join(order["verdict_path"].as_str().unwrap_or_default());
            let mut markdown = orders::render_review_order(&order, Some(&c.dir));
            if let Some(why) = &rejected {
                markdown.push_str(&format!(
                    "\n## Your previous verdict was rejected\n\n{why}\n"
                ));
            }
            let _ = fs::write(area.join("order.md"), &markdown);
            run_command(
                command,
                &c.dir,
                &markdown,
                &[
                    (
                        "SPEC_REVIEW_ORDER",
                        order_path.to_string_lossy().into_owned(),
                    ),
                    ("SPEC_VERDICT", verdict_path.to_string_lossy().into_owned()),
                    ("SPEC_DISPATCH", dispatch.clone()),
                ],
                self.options.agent_timeout,
            )?;
            if c.changed_since_freeze()? {
                c.restore()?;
                return Err(ApplyError::new(
                    "the reviewer changed the candidate; reviews have no write authority",
                ));
            }
            match read_verdict(&verdict_path, &order, &dispatch, &subjects) {
                Ok((found, nonconforming)) => {
                    verdicts.extend(found);
                    if !nonconforming.is_empty() {
                        return Ok(Err(nonconforming));
                    }
                    return Ok(Ok(verdicts));
                }
                Err(why) => rejected = Some(why),
            }
        }
        Err(ApplyError::new(format!(
            "the reviewer's verdict was invalid twice: {}",
            rejected.unwrap_or_default()
        )))
    }

    /// Every requirement's evidence, and the requirements whose policy is not met as
    /// `(address, "<member> (<label>)")`. Acquired requirements publish unverified.
    #[allow(clippy::too_many_arguments)]
    fn evidence(
        &self,
        entries: &[&Value],
        reqs: &BTreeMap<String, Vec<Requirement>>,
        claims: &Claims,
        runs: &[CheckRun],
        verdicts: &BTreeMap<String, Verdict>,
        records: &BTreeMap<String, StateRecord>,
    ) -> AResult<(EvidenceMap, Vec<(String, String)>)> {
        let acquired: BTreeSet<&str> = entries
            .iter()
            .filter(|e| e["strategy"] == "acquire")
            .filter_map(|e| e["address"].as_str())
            .collect();
        let mut unmet = Vec::new();
        let mut evidence: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for (address, list) in reqs {
            for r in list {
                let mut r = r.clone();
                r.tests = claims
                    .tests
                    .get(&r.member)
                    .cloned()
                    .or_else(|| {
                        records
                            .get(address)
                            .map(|rec| previous_tests(rec, &r.member))
                    })
                    .unwrap_or_default();
                let (e, met) = requirement_evidence(&r, runs, verdicts.get(&r.member))?;
                // Acquired requirements publish unverified; the next plan obliges them.
                if !met && !acquired.contains(address.as_str()) {
                    unmet.push((
                        address.clone(),
                        format!("{} ({})", r.member, e["summary"].as_str().unwrap_or("")),
                    ));
                }
                evidence.entry(address.clone()).or_default().push(e);
            }
        }
        Ok((evidence, unmet))
    }

    #[allow(clippy::too_many_arguments)]
    fn publish(
        &self,
        c: &Candidate,
        entries: &[&Value],
        obligated: &[String],
        reqs: &BTreeMap<String, Vec<Requirement>>,
        claims: &Claims,
        runs: &[CheckRun],
        verdicts: &BTreeMap<String, Verdict>,
        records: &BTreeMap<String, StateRecord>,
    ) -> AResult<Outcome> {
        let env = self.env;
        let acquired: BTreeMap<&str, BTreeSet<String>> = entries
            .iter()
            .filter(|e| e["strategy"] == "acquire")
            .filter_map(|e| {
                let slots = e["acquire"]["slots"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect();
                Some((e["address"].as_str()?, slots))
            })
            .collect();
        let (evidence, unmet) = self.evidence(entries, reqs, claims, runs, verdicts, records)?;
        let unmet: Vec<String> = unmet.into_iter().map(|(_, m)| m).collect();
        if !unmet.is_empty() {
            return Ok(Outcome::Failed {
                reason: format!("policy not met: {}", unmet.join(", ")),
            });
        }
        let store = Store::new(&c.dir);
        let mut only: BTreeSet<String> = BTreeSet::new();
        for e in entries {
            let address = e["address"].as_str().unwrap_or_default();
            if let Some(from) = e["moved_from"].as_str() {
                store.remove_record(from)?;
            }
            if e["change"] == "delete" {
                store.remove_record(address)?;
            } else {
                only.insert(address.to_string());
            }
        }
        only.extend(obligated.iter().cloned());
        let sources: BTreeMap<String, String> = only
            .iter()
            .filter_map(|a| Some((a.clone(), env.current_spec(a)?)))
            .collect();
        let publish = Publish {
            only: Some(&only),
            slots: acquired
                .iter()
                .map(|(a, s)| (a.to_string(), s.clone()))
                .collect(),
            sources,
            created: claims.created.clone(),
            evidence,
            provenance: Some(
                json!({ "transaction": self.id, "plan": format!("{}/r{}", self.plan_id, self.revision), "lock": env.lock_hash }),
            ),
        };
        publish_records(&store, &c.dir, env.graph, env.docs, env.mats, &publish)?;
        if !acquired.is_empty() {
            let mut pending = sdlang_plan::pending_imports(&c.dir)?;
            pending.retain(|a, _| !acquired.contains_key(a.as_str()));
            sdlang_plan::write_pending_imports(&c.dir, &pending)?;
        }
        let names: Vec<&str> = entries
            .iter()
            .filter_map(|e| e["address"].as_str())
            .collect();
        let message = format!(
            "spec: {} {}: {}\n\nSpec-Transaction: {}/r{}/{}\n",
            self.plan_id,
            self.id,
            names.join(", "),
            self.plan_id,
            self.revision,
            self.id
        );
        let commit = c.squash(&message)?;
        Ok(Outcome::Published { commit })
    }
}

/// A check run to make: receipt, test output `(member, path)`, expected cases.
type PlannedRun = (
    String,
    Option<(String, String)>,
    BTreeMap<String, Vec<String>>,
);

/// Verdicts by requirement member, or the nonconforming `(address, reason)` pairs.
type ReviewOutcome = Result<BTreeMap<String, Verdict>, Vec<(String, String)>>;

/// Reads and validates a verdict against its review order: verdicts by requirement member,
/// and the nonconforming `(address, reason)` pairs.
type ReadVerdict = (BTreeMap<String, Verdict>, Vec<(String, String)>);

fn read_verdict(
    path: &Path,
    order: &Value,
    dispatch: &str,
    subjects: &[(String, ReviewSubject)],
) -> Result<ReadVerdict, String> {
    let verdict = read_json(path)
        .ok_or_else(|| format!("no valid JSON verdict at {}", order["verdict_path"]))?;
    if verdict["schema"] != "sdlang.review-verdict/1"
        || verdict["dispatch"] != json!(dispatch)
        || verdict["order_hash"] != order["binding"]["order_hash"]
    {
        return Err(format!(
            "the verdict must carry \"schema\": \"sdlang.review-verdict/1\", \"dispatch\": \"{dispatch}\" and \"order_hash\": {}",
            order["binding"]["order_hash"]
        ));
    }
    let answers: Vec<&Value> = verdict["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let mut verdicts = BTreeMap::new();
    let mut nonconforming = Vec::new();
    for (i, (address, s)) in subjects.iter().enumerate() {
        let id = format!("s{}", i + 1);
        let matching: Vec<&&Value> = answers.iter().filter(|a| a["id"] == json!(id)).collect();
        if matching.len() != 1 {
            let ids: Vec<String> = answers.iter().map(|a| a["id"].to_string()).collect();
            return Err(format!("each subject must be answered exactly once by its id: expected \"{id}\" (for {}), found ids [{}]", s.requirement.member, ids.join(", ")));
        }
        let a = matching[0];
        let conformance = a["conformance"]["verdict"].as_str().map(String::from);
        if !matches!(
            conformance.as_deref(),
            Some("conforms" | "does_not_conform" | "cannot_determine")
        ) {
            return Err(format!("{id} needs a conformance verdict of conforms, does_not_conform or cannot_determine"));
        }
        let coverage = a["coverage"]
            .as_array()
            .and_then(|c| c.iter().find(|x| x["candidate"] == "c1"))
            .and_then(|x| x["verdict"].as_str())
            .map(String::from);
        if s.coverage.is_some() && coverage.is_none() {
            return Err(format!(
                "{id} needs a coverage assessment for candidate \"c1\""
            ));
        }
        if conformance.as_deref() == Some("does_not_conform") {
            nonconforming.push((
                address.clone(),
                format!(
                    "{}: {}",
                    s.requirement.member,
                    a["conformance"]["reason"].as_str().unwrap_or("")
                ),
            ));
        }
        verdicts.insert(
            s.requirement.member.clone(),
            Verdict {
                dispatch: dispatch.to_string(),
                conformance,
                coverage,
                reason: a["conformance"]["reason"].as_str().unwrap_or("").into(),
                coverage_reason: a["coverage"]
                    .as_array()
                    .and_then(|c| c.iter().find(|x| x["candidate"] == "c1"))
                    .and_then(|x| x["reason"].as_str())
                    .unwrap_or("")
                    .into(),
            },
        );
    }
    if answers.len() != subjects.len() {
        return Err("the verdict answers subjects that were not requested".into());
    }
    Ok((verdicts, nonconforming))
}

enum AgentStop {
    Blocked(String),
    Error(ApplyError),
}

/// The skills the subjects' kinds name (`skills: [app_router]`), as `(name, text)` from
/// their packages' headers: guidance for the agent, never obligations.
fn skills<'d>(project: &Project, decls: impl Iterator<Item = &'d Value>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for decl in decls {
        let (Some(body), Some(package)) = (kind_body(project, decl), kind_package(project, decl))
        else {
            continue;
        };
        let names: Vec<String> = crate::checks::names(crate::checks::field(body, "skills"))
            .into_iter()
            .map(|n| n.rsplit('.').next().unwrap_or(&n).to_string())
            .collect();
        for name in names {
            let label = format!("{}.{name}", package.import_name);
            if out.iter().any(|(n, _)| *n == label) {
                continue;
            }
            let Some(rel) = package.skills.get(&name) else {
                continue;
            };
            if let Ok(text) = fs::read_to_string(package.root.join(rel)) {
                out.push((label, text));
            }
        }
    }
    out
}

/// The state record an entry starts from: its previous address for a moved declaration.
fn record_for<'r>(
    records: &'r BTreeMap<String, StateRecord>,
    e: &Value,
) -> Option<&'r StateRecord> {
    let from = e["moved_from"].as_str().or_else(|| e["address"].as_str())?;
    records.get(from)
}

/// The declaration, root key and root directory of the deepest creation root holding `path`.
fn creation_root(
    mats: &BTreeMap<String, Materialisation>,
    path: &str,
) -> Option<(String, String, String)> {
    mats.iter()
        .flat_map(|(a, m)| m.roots.iter().map(move |(k, d)| (a, k, d)))
        .filter(|(_, _, d)| path.starts_with(d.as_str()))
        .max_by_key(|(_, _, d)| d.len())
        .map(|(a, k, d)| (a.clone(), k.clone(), d.clone()))
}

/// A stable key for an inferred created output: its file stem in snake case, made unique
/// among the declaration's slots and created outputs.
fn new_key(rel: &str, created: &BTreeMap<String, Output>, mat: Option<&Materialisation>) -> String {
    let stem = rel.rsplit('/').next().unwrap_or(rel);
    let stem = stem.split('.').next().unwrap_or(stem);
    let mut key: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if key.is_empty() || key.starts_with(|ch: char| ch.is_ascii_digit()) {
        key = format!("f_{key}");
    }
    let taken = |k: &str| created.contains_key(k) || mat.is_some_and(|m| m.slots.contains_key(k));
    if !taken(&key) {
        return key;
    }
    (2..)
        .map(|n| format!("{key}_{n}"))
        .find(|k| !taken(k))
        .expect("an unused key")
}

/// Binds each tested requirement that no result bound to the cases tagged with its key in
/// its declaration's test outputs (docs/protocol.md, Result). Returns the members it bound.
fn infer_tests(
    reqs: &BTreeMap<String, Vec<Requirement>>,
    runs: &[CheckRun],
    claims: &mut Claims,
) -> BTreeSet<String> {
    let mut bound = BTreeSet::new();
    for (address, list) in reqs {
        for r in list.iter().filter(|r| r.policy.tests) {
            if claims.tests.contains_key(&r.member) {
                continue;
            }
            let key = crate::names::key(&r.member);
            let mut found: Vec<(String, String)> = runs
                .iter()
                .filter(|x| {
                    x.subject.as_deref() == Some(address.as_str()) && x.cases.contains_key(key)
                })
                .filter_map(|x| Some((x.output.clone()?, key.to_string())))
                .collect();
            found.dedup();
            if !found.is_empty() {
                claims.tests.insert(r.member.clone(), found);
                bound.insert(r.member.clone());
            }
        }
    }
    bound
}

fn copy_tree(from: &Path, to: &Path) {
    let Ok(entries) = fs::read_dir(from) else {
        return;
    };
    let _ = fs::create_dir_all(to);
    for e in entries.flatten() {
        let path = e.path();
        let dest = to.join(e.file_name());
        if path.is_dir() {
            copy_tree(&path, &dest);
        } else {
            let _ = fs::copy(&path, &dest);
        }
    }
}

fn c_existed_at_base(c: &Candidate, path: &str) -> bool {
    std::process::Command::new("git")
        .current_dir(&c.dir)
        .args([
            "cat-file",
            "-e",
            &format!("{}:{path}", crate::workspace::untagged(&c.base)),
        ])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A requirement's test bindings from its previous evidence.
fn previous_tests(record: &StateRecord, member: &str) -> Vec<(String, String)> {
    record
        .evidence
        .iter()
        .find(|e| e["subject"]["requirement"] == json!(member))
        .and_then(|e| e["coverage"]["tests"].as_array())
        .map(|ts| {
            ts.iter()
                .filter_map(|t| {
                    Some((
                        t["output"].as_str()?.to_string(),
                        t["case"].as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A requirement's previous verdicts, which stay current when only run inputs changed.
fn previous_verdict(record: &StateRecord, member: &str) -> Option<Verdict> {
    let e = record
        .evidence
        .iter()
        .find(|e| e["subject"]["requirement"] == json!(member))?;
    let coverage = e["coverage"]["verdict"]
        .as_str()
        .filter(|v| *v != "missing")
        .map(String::from);
    let conformance = e["review"]["verdict"]
        .as_str()
        .filter(|v| *v != "missing")
        .map(String::from);
    if coverage.is_none() && conformance.is_none() {
        return None;
    }
    let dispatch = e["coverage"]["dispatch"]
        .as_str()
        .or(e["review"]["dispatch"].as_str())
        .unwrap_or_default()
        .to_string();
    Some(Verdict {
        dispatch,
        conformance,
        coverage,
        reason: String::new(),
        coverage_reason: String::new(),
    })
}

fn generator_run(package: &sdlang_graph::loader::Package, name: &str) -> Option<Vec<String>> {
    package
        .files
        .iter()
        .flat_map(|f| f.ast.statements.iter())
        .find_map(|s| match s {
            Statement::Decl(d) => match &d.item {
                Exportable::Named(n) if n.kind.dotted() == "generator" && n.name.text == name => {
                    n.body.members.iter().find_map(|m| match &m.value {
                        MemberValue::Field(f) if m.name.text == "run" => match &f.value {
                            Expr::List { items, .. } => Some(
                                items
                                    .iter()
                                    .filter_map(|i| match i {
                                        Expr::String(s) => Some(sdlang_graph::scope::lit_text(s)),
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => None,
                        },
                        _ => None,
                    })
                }
                _ => None,
            },
            _ => None,
        })
}
