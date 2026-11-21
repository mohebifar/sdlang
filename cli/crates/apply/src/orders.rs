//! Work orders, review orders and generator inputs (PROTOCOL.md), with their binding and
//! Markdown rendering.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use intlang_canon::{canonical, digest, GraphDocuments};
use intlang_graph::loader::Project;
use intlang_graph::scope::{Owner, Scope};
use intlang_plan::materialise::{kind_body, Materialisation};
use intlang_plan::state::{consumed_for, file_digest, StateRecord, Store};
use intlang_syntax::ast::{Block, Exportable, Expr, MemberValue, Statement};
use intlang_syntax::strings::Part;
use serde_json::{json, Map, Value};

use crate::error::AResult;
use crate::evidence::Requirement;
use crate::workspace::Repo;

/// What every order is assembled from.
pub struct Env<'a> {
    pub project: &'a Project,
    pub scope: &'a Scope<'a>,
    pub graph: &'a Value,
    pub docs: &'a GraphDocuments,
    pub mats: &'a BTreeMap<String, Materialisation>,
    pub store: &'a Store,
    pub repo: &'a Repo,
    pub plan: &'a Value,
    pub plan_hash: String,
    pub graph_hash: String,
    pub lock_hash: String,
}

/// Sets `binding.order_hash` (or the top-level one, for a generator input) to the digest of
/// the order's canonical JSON without it.
pub fn seal(mut order: Value) -> AResult<Value> {
    order["binding"]["order_hash"] = json!("");
    if let Some(b) = order["binding"].as_object_mut() {
        b.remove("order_hash");
    }
    let hash = digest(&order)?;
    order["binding"]["order_hash"] = json!(hash);
    Ok(order)
}

/// Recomputes an order's hash, for checking a reply's echo.
pub fn order_hash(order: &Value) -> AResult<String> {
    let mut o = order.clone();
    if let Some(b) = o["binding"].as_object_mut() {
        b.remove("order_hash");
    }
    Ok(digest(&o)?)
}

fn decl_slice(text: &str, statements: &[Statement], name: &str) -> Option<String> {
    statements.iter().find_map(|s| match s {
        Statement::Decl(d) => {
            let n = match &d.item {
                Exportable::Named(n) => &n.name.text,
                Exportable::Type { name, .. } | Exportable::Let { name, .. } => &name.text,
                _ => return None,
            };
            (n == name).then(|| format!("{}\n", text[d.span.start..d.span.end].trim_end()))
        }
        _ => None,
    })
}

impl Env<'_> {
    fn info(&self, address: &str) -> Option<&intlang_graph::scope::DeclInfo> {
        self.scope.decls.iter().find(|d| d.address == address)
    }

    fn source_file(&self, address: &str) -> Option<&intlang_graph::loader::SourceFile> {
        let info = self.info(address)?;
        self.project.modules.get(&info.module)?.files.get(info.file)
    }

    /// The declaration's current source text.
    pub fn current_spec(&self, address: &str) -> Option<String> {
        let info = self.info(address)?;
        let file = self.source_file(address)?;
        decl_slice(&file.text, &file.ast.statements, &info.name)
    }

    /// Its source text as last applied, from the applied store.
    pub fn previous_spec(&self, record: Option<&StateRecord>) -> Option<String> {
        let hash = record?.source.as_ref()?;
        let doc = self.store.applied(hash).ok()??;
        doc["source"].as_str().map(String::from)
    }

    /// Every name the declaration's source uses, resolved to its canonical address.
    pub fn resolution(&self, address: &str) -> Map<String, Value> {
        let mut out = Map::new();
        let (Some(info), Some(file)) = (self.info(address), self.source_file(address)) else {
            return out;
        };
        let Some(Statement::Decl(d)) = file.ast.statements.get(info.statement) else {
            return out;
        };
        let owner = Owner::Module(info.module.clone());
        let mut paths: Vec<Vec<String>> = Vec::new();
        if let Exportable::Named(n) = &d.item {
            if n.kind.segments.len() > 1 {
                paths.push(n.kind.segments.iter().map(|s| s.text.clone()).collect());
            }
            collect_block(&n.body, &mut paths);
        }
        for names in paths {
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            if let Ok(target) = self.scope.resolve_names(&owner, &refs, file, d.span) {
                if let Some(canonical) = self.scope.canonical(&target) {
                    out.insert(names.join("."), json!(canonical));
                } else if let intlang_graph::scope::Target::PackageItem { .. } = target {
                    // kinds resolve to package items above; nothing else to add
                }
            }
        }
        out
    }
}

fn collect_block(b: &Block, out: &mut Vec<Vec<String>>) {
    for m in &b.members {
        match &m.value {
            MemberValue::Field(f) => collect_expr(&f.value, out),
            MemberValue::Section(s) => collect_block(s, out),
        }
    }
}

fn collect_expr(e: &Expr, out: &mut Vec<Vec<String>>) {
    match e {
        Expr::Path { path, record, .. } => {
            out.push(path.segments.iter().map(|s| s.text.clone()).collect());
            if let Some(r) = record {
                r.fields
                    .iter()
                    .for_each(|f| collect_expr(&f.field.value, out));
            }
        }
        Expr::Ref { target, .. } | Expr::Optional { inner: target, .. } => {
            collect_expr(target, out)
        }
        Expr::List { items, .. }
        | Expr::Union {
            variants: items, ..
        } => items.iter().for_each(|i| collect_expr(i, out)),
        Expr::Record(r) => r
            .fields
            .iter()
            .for_each(|f| collect_expr(&f.field.value, out)),
        Expr::String(s) => {
            for p in &s.parts {
                if let Part::Mention(m) = p {
                    if m != "self" {
                        out.push(m.split('.').map(String::from).collect());
                    }
                }
            }
        }
        _ => {}
    }
}

/// A unified diff of two texts as one hunk, from a longest-common-subsequence alignment.
pub fn unified_diff(name: &str, old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut lines = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            lines.push(format!(" {}", a[i]));
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] > lcs[i + 1][j]) {
            lines.push(format!("+{}", b[j]));
            j += 1;
        } else {
            lines.push(format!("-{}", a[i]));
            i += 1;
        }
    }
    let start = |n: usize| if n == 0 { 0 } else { 1 };
    let mut out = format!(
        "--- a/{name}\n+++ b/{name}\n@@ -{},{} +{},{} @@\n",
        start(a.len()),
        a.len(),
        start(b.len()),
        b.len()
    );
    for l in lines {
        out.push_str(&l);
        out.push('\n');
    }
    out
}

fn prose_text(v: &Value) -> String {
    v.get("$prose")
        .and_then(Value::as_str)
        .or_else(|| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| canonical(v).unwrap_or_default())
}

/// The readable text of an inherited part's document.
fn part_text(source: &str, doc: &Value) -> String {
    if source.starts_with("rule:") {
        let mut s = String::new();
        if !doc["prompt"].is_null() {
            s.push_str(&prose_text(&doc["prompt"]));
        }
        for (k, v) in doc["expect"].as_object().into_iter().flatten() {
            let _ = writeln!(s, "expect {k}: {}", prose_text(v));
        }
        s
    } else {
        prose_text(doc)
    }
}

/// Inherited normative context, project first, then modules outermost first, then rules.
pub fn inherited(env: &Env, address: &str, record: Option<&StateRecord>) -> AResult<Vec<Value>> {
    let mut out = Vec::new();
    for (source, doc) in &env.docs.declarations[address].context_parts {
        let hash = digest(doc)?;
        let mut v = json!({ "source": source, "normative": true, "hash": hash, "text": part_text(source, doc) });
        if let Some(old) = record
            .and_then(|r| r.context_parts.get(source))
            .filter(|h| **h != hash)
        {
            v["previous_text"] = match env.store.applied(old)? {
                Some(d) => json!(part_text(source, &d)),
                None => Value::Null,
            };
        }
        out.push(v);
    }
    Ok(out)
}

/// Every consumed or observed projection of what the declaration references.
pub fn references(env: &Env, address: &str, record: Option<&StateRecord>) -> AResult<Vec<Value>> {
    let decl = env.graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|d| d["address"] == address)
        .cloned()
        .unwrap_or(Value::Null);
    let mut out = Vec::new();
    for c in consumed_for(&decl, env.docs)? {
        let d = &env.docs.declarations[&c.target];
        let doc = match c.projection.as_str() {
            "behaviour" => &d.behaviour,
            "context" => &d.context,
            _ => &d.public,
        };
        let projection =
            if c.projection == "public" || c.projection == "behaviour" || c.projection == "context"
            {
                c.projection.clone()
            } else {
                "public".into()
            };
        let mut v = json!({
            "from": address, "target": c.target, "relation": c.relation, "projection": projection,
            "viewpoint": "desired", "hash": c.hash, "text": serde_json::to_string_pretty(doc).unwrap_or_default(),
        });
        let old = record.and_then(|r| {
            r.consumed.iter().find(|o| {
                o.relation == c.relation && o.target == c.target && o.projection == c.projection
            })
        });
        if let Some(old) = old.filter(|o| o.hash != c.hash) {
            v["previous_text"] = match env.store.applied(&old.hash)? {
                Some(d) => json!(serde_json::to_string_pretty(&d).unwrap_or_default()),
                None => Value::Null,
            };
        }
        out.push(v);
    }
    Ok(out)
}

/// The kind's ambient imports, as allowed imports.
fn allowed_imports(env: &Env, decl: &Value) -> Vec<Value> {
    let Some(body) = kind_body(env.project, decl) else {
        return vec![];
    };
    let mut out = Vec::new();
    for m in &body.members {
        let MemberValue::Section(s) = &m.value else {
            continue;
        };
        if m.name.text != "ambient" {
            continue;
        }
        for f in &s.members {
            if let (true, MemberValue::Field(field)) = (f.name.text == "imports", &f.value) {
                if let Expr::List { items, .. } = &field.value {
                    for i in items {
                        if let Expr::String(lit) = i {
                            out.push(json!({
                                "module": intlang_graph::scope::lit_text(lit), "exports": ["*"], "mode": "runtime",
                                "via": format!("ambient:{}", crate::checks::kind_short(decl["kind"].as_str().unwrap_or_default())),
                            }));
                        }
                    }
                }
            }
        }
    }
    out
}

/// The write envelope of one declaration in the candidate at `dir`.
pub fn envelope(
    env: &Env,
    address: &str,
    record: Option<&StateRecord>,
    dir: &Path,
) -> AResult<Value> {
    let decl = env.graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|d| d["address"] == address)
        .cloned()
        .unwrap_or(Value::Null);
    let m = env.mats.get(address).cloned().unwrap_or_default();
    let mut write: BTreeMap<String, Value> = BTreeMap::new();
    for (key, path) in &m.slots {
        let expected = file_digest(dir, path)?.unwrap_or_else(|| "absent".into());
        write.insert(path.clone(), json!({ "output": format!("{address}#outputs.{key}"), "path": path, "expected": expected }));
    }
    for (key, out) in record.map(|r| &r.outputs).into_iter().flatten() {
        if out.origin == "created" {
            let expected = file_digest(dir, &out.path)?.unwrap_or_else(|| "absent".into());
            write.insert(out.path.clone(), json!({ "output": format!("{address}#outputs.{key}"), "path": out.path, "expected": expected }));
        }
    }
    let create_under: Vec<Value> = m
        .roots
        .iter()
        .map(|(k, p)| json!({ "root": k, "path": p }))
        .collect();
    Ok(json!({
        "write": write.into_values().collect::<Vec<_>>(),
        "create_under": create_under,
        "delete": [],
        "allowed_imports": allowed_imports(env, &decl),
    }))
}

fn requirement_value(r: &Requirement, baseline: &str) -> Value {
    json!({
        "subject": r.subject(), "text": r.text, "contract_hash": r.contract_hash,
        "policy": r.policy.to_value(), "policy_source": r.policy_source, "baseline": baseline,
        "tests": r.tests.iter().map(|(o, c)| json!({ "output": o, "case": c })).collect::<Vec<_>>(),
    })
}

/// One coding subject of a work order.
pub struct Subject<'a> {
    pub address: &'a str,
    pub entry: &'a Value,
    pub record: Option<&'a StateRecord>,
    pub requirements: &'a [Requirement],
}

/// The baseline label of a requirement from its previous evidence.
fn baseline_label(record: Option<&StateRecord>, r: &Requirement) -> String {
    let Some(record) = record else {
        return "none".into();
    };
    record
        .evidence
        .iter()
        .find(|e| e["subject"]["requirement"] == json!(r.member))
        .and_then(|e| e["summary"].as_str())
        .map(|s| match s {
            "failed" => "unverified".to_string(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "unverified".into())
}

/// A sealed work order.
#[allow(clippy::too_many_arguments)]
pub fn work_order(
    env: &Env,
    dispatch: &str,
    transaction: &str,
    base_commit: &str,
    input_snapshot: &str,
    attempt: (u32, u32, &[Value]),
    subjects: &[Subject],
    checks: &[Value],
    dir: &Path,
) -> AResult<Value> {
    let mut subject_values = Vec::new();
    let mut inherited_all: BTreeMap<String, Value> = BTreeMap::new();
    let mut references_all = Vec::new();
    let mut manifest: BTreeMap<String, Value> = BTreeMap::new();
    for s in subjects {
        let current = env.current_spec(s.address).unwrap_or_default();
        let previous = if s.entry["change"] == "create" {
            None
        } else {
            env.previous_spec(s.record)
        };
        let diff = unified_diff(s.address, previous.as_deref().unwrap_or(""), &current);
        let causes: Vec<Value> = s.entry["causes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|c| {
                [
                    "new",
                    "own_projection",
                    "consumed_incompatible",
                    "observed",
                    "context",
                    "convention",
                    "provider",
                    "repair",
                    "escalation",
                ]
                .contains(c)
            })
            .map(|c| json!({ "kind": c }))
            .collect();
        let envelope = envelope(env, s.address, s.record, dir)?;
        for w in envelope["write"].as_array().into_iter().flatten() {
            let path = w["path"].as_str().unwrap_or_default();
            if let Some(hash) = w["expected"].as_str().filter(|h| h.starts_with("sha256:")) {
                let size = std::fs::metadata(dir.join(path))
                    .map(|m| m.len())
                    .unwrap_or(0);
                let output = w["output"].as_str().unwrap_or_default();
                manifest.insert(
                    format!("code:{output}"),
                    json!({ "id": format!("code:{output}"), "class": "code", "required": true, "included": false,
                            "resource": format!("ctx:code:{}", &hash[7..19]), "hash": hash, "tokens": size.div_ceil(4) }),
                );
            }
        }
        subject_values.push(json!({
            "address": s.address,
            "kind": s.entry_kind(env),
            "change": if s.entry["change"] == "create" { "create" } else { "update" },
            "strategy": if s.entry["strategy"] == "replace" { "replace" } else { "edit" },
            "causes": causes,
            "spec": { "rendering": "source/1", "current": current, "previous": previous, "diff": diff, "resolution": env.resolution(s.address) },
            "envelope": envelope,
            "requirements": s.requirements.iter().map(|r| requirement_value(r, &baseline_label(s.record, r))).collect::<Vec<_>>(),
        }));
        for i in inherited(env, s.address, s.record)? {
            inherited_all
                .entry(format!("{}{}", i["source"], i["hash"]))
                .or_insert(i);
        }
        references_all.extend(references(env, s.address, s.record)?);
    }
    let order = json!({
        "schema": "intlang.work-order/1",
        "dispatch": dispatch,
        "binding": {
            "order_hash": "", "plan": env.plan["plan"], "plan_revision": env.plan["revision"], "plan_hash": env.plan_hash,
            "transaction": transaction, "base_commit": base_commit, "input_snapshot": input_snapshot,
            "graph_hash": env.graph_hash, "lock_hash": env.lock_hash,
        },
        "attempt": { "number": attempt.0, "max": attempt.1, "workspace": if attempt.0 == 1 { "new" } else { "continued" }, "previous": attempt.2 },
        "subjects": subject_values,
        "context": {
            "inherited": inherited_all.into_values().collect::<Vec<_>>(),
            "references": references_all,
            "manifest": manifest.into_values().collect::<Vec<_>>(),
            "budget": { "policy": "context-pack/1", "renderer": "markdown/1", "tokenizer": "bytes-div-4", "input_limit": 200000, "reserved_output": 32000, "reserved_feedback": 8000, "used_input": 0 },
        },
        "checks": checks,
        "result_path": format!(".spec/run/{dispatch}/result.json"),
        "tools": { "mcp": { "transport": "stdio", "command": ["spec", "mcp", "--dispatch", dispatch], "protocol": "2025-06-18" } },
        "limits": { "timeout_seconds": 1800, "result_max_bytes": 65536 },
    });
    let mut order = order;
    let used = render_work_order(&order).len().div_ceil(4) as u64;
    order["context"]["budget"]["used_input"] = json!(used);
    seal(order)
}

impl Subject<'_> {
    fn entry_kind(&self, env: &Env) -> String {
        env.graph["declarations"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|d| d["address"] == self.address)
            .and_then(|d| d["kind"].as_str())
            .unwrap_or_default()
            .to_string()
    }
}

/// Coverage candidate tests `(output, case, oracle)` and the run receipts behind them.
pub type Coverage = (Vec<(String, String, String)>, Vec<String>);

/// One subject a reviewer judges.
pub struct ReviewSubject {
    pub requirement: Requirement,
    pub context_hash: String,
    pub previous_text: Option<String>,
    pub code: Vec<String>,
    /// `(output, case, oracle)` and the run receipts, when coverage is assessed.
    pub coverage: Option<Coverage>,
}

/// A sealed review order.
pub fn review_order(
    env: &Env,
    dispatch: &str,
    snapshot: &str,
    subjects: &[ReviewSubject],
    context: (Vec<Value>, Vec<Value>),
) -> AResult<Value> {
    let values: Vec<Value> = subjects
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut assess = vec!["conformance"];
            let mut v = json!({
                "id": format!("s{}", i + 1), "subject": s.requirement.subject(), "text": s.requirement.text,
                "previous_text": s.previous_text, "contract_hash": s.requirement.contract_hash,
                "context_hash": s.context_hash, "policy": s.requirement.policy.to_value(), "code": s.code,
            });
            if let Some((tests, receipts)) = &s.coverage {
                assess.push("coverage");
                v["coverage_candidates"] = json!([{
                    "id": "c1",
                    "tests": tests.iter().map(|(o, c, oracle)| json!({ "test": { "output": o, "case": c }, "oracle": oracle })).collect::<Vec<_>>(),
                    "run_receipts": receipts,
                }]);
            }
            v["assess"] = json!(assess);
            v
        })
        .collect();
    seal(json!({
        "schema": "intlang.review-order/1",
        "dispatch": dispatch,
        "binding": { "order_hash": "", "plan_hash": env.plan_hash, "candidate_snapshot": snapshot, "graph_hash": env.graph_hash, "lock_hash": env.lock_hash },
        "subjects": values,
        "context": { "inherited": context.0, "references": context.1, "manifest": [] },
        "verdict_path": format!(".spec/run/{dispatch}/verdict.json"),
        "tools": { "mcp": { "transport": "stdio", "command": ["spec", "mcp", "--dispatch", dispatch], "protocol": "2025-06-18" } },
    }))
}

/// A generator input for the subjects one generator produces in a transaction. The
/// binding's `order_hash` is set by [`seal`].
#[allow(clippy::too_many_arguments)]
pub fn generator_input(
    env: &Env,
    dispatch: &str,
    snapshot: &str,
    subjects: &[(String, Option<&StateRecord>)],
    dir: &Path,
    staging: &str,
    generation_id: &str,
) -> AResult<Value> {
    let decls: BTreeMap<&str, &Value> = env.graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["address"].as_str()?, d)))
        .collect();
    let mut subject_values = Vec::new();
    let mut write = Vec::new();
    let mut retain = Vec::new();
    for (address, record) in subjects {
        let decl = decls.get(address.as_str()).copied();
        let before = match record.and_then(|r| r.entry.clone()) {
            Some(hash) => env.store.applied(&hash)?.unwrap_or(Value::Null),
            None => Value::Null,
        };
        let mut v = json!({ "address": address, "before": before, "after": decl.cloned() });
        if let Some(d) = decl {
            let members: Vec<Value> = d["edges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|e| e["relation"] == "collects")
                .filter_map(|e| decls.get(e["target"].as_str()?).map(|x| (*x).clone()))
                .collect();
            if !members.is_empty() {
                v["aggregate"] = json!(members);
            }
            if let Some(body) = kind_body(env.project, d) {
                for m in &body.members {
                    if let (true, MemberValue::Section(s)) = (m.name.text == "retains", &m.value) {
                        for r in &s.members {
                            let MemberValue::Section(rs) = &r.value else {
                                continue;
                            };
                            let path = rs.members.iter().find_map(|f| match &f.value {
                                MemberValue::Field(x) if f.name.text == "path" => match &x.value {
                                    Expr::String(s) => Some(intlang_graph::scope::lit_text(s)),
                                    _ => None,
                                },
                                _ => None,
                            });
                            if let Some(path) = path {
                                let mut existing: Vec<String> = std::fs::read_dir(dir.join(&path))
                                    .map(|it| {
                                        it.flatten()
                                            .filter_map(|e| {
                                                e.path()
                                                    .file_stem()
                                                    .map(|s| s.to_string_lossy().into_owned())
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                existing.sort();
                                retain.push(json!({ "root": r.name.text, "path": path, "existing": existing }));
                            }
                        }
                    }
                }
            }
        }
        subject_values.push(v);
        for w in envelope(env, address, *record, dir)?["write"]
            .as_array()
            .into_iter()
            .flatten()
        {
            write.push(w.clone());
        }
    }
    seal(json!({
        "protocol": "intlang.generator/1",
        "dispatch": dispatch,
        "binding": { "order_hash": "", "plan_hash": env.plan_hash, "input_snapshot": snapshot, "graph_hash": env.graph_hash, "lock_hash": env.lock_hash },
        "subjects": subject_values,
        "envelope": { "write": write, "delete": [], "move": [], "retain": retain },
        "decisions": [],
        "generation_id": generation_id,
        "staging": staging,
    }))
}

fn fence(s: &mut String, lang: &str, body: &str) {
    let _ = writeln!(s, "```{lang}\n{}\n```\n", body.trim_end());
}

/// The work order as Markdown, for the agent's stdin.
pub fn render_work_order(order: &Value) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Work order {}\n",
        order["dispatch"].as_str().unwrap_or("")
    );
    let _ = writeln!(
        s,
        "Make the code in the files listed under each subject's envelope satisfy that subject's spec and requirements. \
         Write only those files, or new files under its creation roots. Do not edit spec/ or .spec/ (other than your result). \
         Attempt {} of {}.\n",
        order["attempt"]["number"], order["attempt"]["max"]
    );
    let previous = order["attempt"]["previous"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !previous.is_empty() {
        s.push_str("## Earlier attempts failed\n\n");
        for f in &previous {
            let _ = writeln!(
                s,
                "- attempt {}: {} ({})",
                f["attempt"],
                f["kind"].as_str().unwrap_or(""),
                f["classification"].as_str().unwrap_or("")
            );
            fence(&mut s, "text", f["excerpt"].as_str().unwrap_or(""));
        }
    }
    for subj in order["subjects"].as_array().into_iter().flatten() {
        let _ = writeln!(
            s,
            "## {} `{}` ({}, {})\n",
            subj["kind"].as_str().unwrap_or(""),
            subj["address"].as_str().unwrap_or(""),
            subj["change"].as_str().unwrap_or(""),
            subj["strategy"].as_str().unwrap_or("")
        );
        s.push_str("### Spec\n\n");
        fence(
            &mut s,
            "spec",
            subj["spec"]["current"].as_str().unwrap_or(""),
        );
        if !subj["spec"]["previous"].is_null() {
            s.push_str("### Spec diff\n\n");
            fence(&mut s, "diff", subj["spec"]["diff"].as_str().unwrap_or(""));
        }
        s.push_str("### Names\n\n");
        for (k, v) in subj["spec"]["resolution"].as_object().into_iter().flatten() {
            let _ = writeln!(s, "- `{k}` is `{}`", v.as_str().unwrap_or(""));
        }
        s.push_str("\n### Envelope\n\n");
        for w in subj["envelope"]["write"].as_array().into_iter().flatten() {
            let _ = writeln!(
                s,
                "- write `{}` ({}; now {})",
                w["path"].as_str().unwrap_or(""),
                w["output"].as_str().unwrap_or(""),
                w["expected"].as_str().unwrap_or("")
            );
        }
        for c in subj["envelope"]["create_under"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let _ = writeln!(
                s,
                "- create new files under `{}` (root `{}`); register each in your result's outputs",
                c["path"].as_str().unwrap_or(""),
                c["root"].as_str().unwrap_or("")
            );
        }
        for i in subj["envelope"]["allowed_imports"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let _ = writeln!(s, "- may import `{}`", i["module"].as_str().unwrap_or(""));
        }
        s.push_str("\n### Requirements\n\n");
        for r in subj["requirements"].as_array().into_iter().flatten() {
            let p = &r["policy"];
            let _ = writeln!(
                s,
                "- `{}`: {} (tests: {}, review: {})",
                r["subject"]["requirement"].as_str().unwrap_or(""),
                r["text"].as_str().unwrap_or(""),
                p["tests"],
                p["review"]
            );
        }
        s.push('\n');
    }
    s.push_str("## Inherited context\n\n");
    for i in order["context"]["inherited"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let _ = writeln!(s, "From `{}`:\n", i["source"].as_str().unwrap_or(""));
        fence(&mut s, "text", i["text"].as_str().unwrap_or(""));
    }
    s.push_str("## Referenced declarations\n\n");
    for r in order["context"]["references"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let _ = writeln!(
            s,
            "`{}` {} `{}` ({}):\n",
            r["from"].as_str().unwrap_or(""),
            r["relation"].as_str().unwrap_or(""),
            r["target"].as_str().unwrap_or(""),
            r["projection"].as_str().unwrap_or("")
        );
        fence(&mut s, "json", r["text"].as_str().unwrap_or(""));
    }
    s.push_str("## Result\n\n");
    let _ = writeln!(
        s,
        "When done, write a JSON result to `{}` (also in $SPEC_RESULT) with: \"schema\": \"intlang.result/1\", \"dispatch\": \"{}\", \
         \"order_hash\": \"{}\", \"status\": \"submitted\" or \"blocked\", a one-paragraph \"summary\", \"outputs\" claiming every new \
         file you created under a creation root ({{\"op\": \"create\", \"owner\": <address>, \"key\": <snake_case key>, \"root\": <root>, \
         \"relative_path\": <path under the root>}}), and \"tests\" binding each requirement to the test cases that cover it \
         ({{\"subject\": {{\"target\", \"requirement\"}}, \"tests\": [{{\"output\", \"case\"}}]}}). Tag each test case with a stable snake_case \
         case name the test reporter reports. The full order is the JSON file in $SPEC_WORK_ORDER. MCP tools are not available in this version.",
        order["result_path"].as_str().unwrap_or(""),
        order["dispatch"].as_str().unwrap_or(""),
        order["binding"]["order_hash"].as_str().unwrap_or("")
    );
    s
}

/// The review order as Markdown, for the reviewer's stdin.
pub fn render_review_order(order: &Value) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Review order {}\n",
        order["dispatch"].as_str().unwrap_or("")
    );
    s.push_str("You did not write this code and may not change it. For each subject, judge the frozen candidate in the current directory.\n\n");
    for subj in order["subjects"].as_array().into_iter().flatten() {
        let _ = writeln!(
            s,
            "## {} `{}`\n\n{}\n",
            subj["id"].as_str().unwrap_or(""),
            subj["subject"]["requirement"].as_str().unwrap_or(""),
            subj["text"].as_str().unwrap_or("")
        );
        let _ = writeln!(s, "Assess: {}. Code: {}.\n", subj["assess"], subj["code"]);
        for c in subj["coverage_candidates"].as_array().into_iter().flatten() {
            let _ = writeln!(
                s,
                "Coverage candidate {}: {}\n",
                c["id"].as_str().unwrap_or(""),
                c["tests"]
            );
        }
    }
    for i in order["context"]["inherited"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let _ = writeln!(
            s,
            "Inherited from `{}`:\n",
            i["source"].as_str().unwrap_or("")
        );
        fence(&mut s, "text", i["text"].as_str().unwrap_or(""));
    }
    let _ = writeln!(
        s,
        "Write the verdict to `{}` (also $SPEC_VERDICT): {{\"schema\": \"intlang.review-verdict/1\", \"dispatch\": \"{}\", \"order_hash\": \"{}\", \
         \"subjects\": [{{\"id\", \"conformance\": {{\"verdict\": \"conforms\"|\"does_not_conform\"|\"cannot_determine\", \"reason\"}}, \
         \"coverage\": [{{\"candidate\", \"verdict\": \"sufficient\"|\"insufficient\"|\"cannot_determine\", \"reason\"}}]}}]}}. Answer every subject and every assessment exactly once.",
        order["verdict_path"].as_str().unwrap_or(""),
        order["dispatch"].as_str().unwrap_or(""),
        order["binding"]["order_hash"].as_str().unwrap_or("")
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs() {
        let d = unified_diff("x", "a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!(d, "--- a/x\n+++ b/x\n@@ -1,3 +1,4 @@\n a\n-b\n+B\n c\n+d\n");
        assert!(unified_diff("x", "", "a\n").contains("@@ -0,0 +1,1 @@"));
    }

    #[test]
    fn sealing() {
        let o = seal(json!({ "binding": { "order_hash": "", "plan": "p" }, "x": 1 })).unwrap();
        assert_eq!(
            order_hash(&o).unwrap(),
            o["binding"]["order_hash"].as_str().unwrap()
        );
    }
}
