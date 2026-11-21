//! The MCP tool server of one dispatch (PROTOCOL.md, MCP tools): newline-delimited JSON-RPC
//! 2.0 on stdio. It reads the dispatch's context file, `.spec/run/<dispatch>/mcp.json`,
//! which the engine writes beside the order, and never writes state, widens the envelope
//! or records evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use intlang_canon::digest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::checks::{run_check, CheckDef};
use crate::error::{AResult, ApplyError};
use crate::imports::{check_imports, ImportPolicy};

/// One subject as the server sees it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpSubject {
    pub address: String,
    pub kind: String,
    pub obligations: Vec<String>,
    /// `(member, path)` of its outputs.
    pub outputs: Vec<(String, String)>,
    pub exports: Vec<String>,
    /// Creation root key -> path.
    pub roots: BTreeMap<String, String>,
    pub requirements: Vec<String>,
    pub imports: ImportPolicy,
    /// Expected test cases per test output member.
    pub expected_cases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// The dispatch context file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpContext {
    pub dispatch: String,
    /// Paths relative to the candidate.
    pub order: String,
    pub graph: String,
    pub logs: String,
    /// The commit the agent started from.
    pub base: String,
    pub resources: BTreeMap<String, (String, String)>,
    pub visible: BTreeSet<String>,
    pub subjects: Vec<McpSubject>,
    pub checks: Vec<(String, CheckDef)>,
    pub profile: String,
    pub check_timeout_seconds: u64,
}

const PAGE: usize = 32 * 1024;

/// One self-check run to make: the test output `(member, path)` and its expected cases.
type SelfRun = (Option<(String, String)>, BTreeMap<String, Vec<String>>);

fn page(text: &str, cursor: Option<&str>) -> (String, Option<String>) {
    let start = cursor
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0)
        .min(text.len());
    let mut end = (start + PAGE).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let next = (end < text.len()).then(|| end.to_string());
    (text[start..end].to_string(), next)
}

fn git(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The current tree as `git:<tree id>`, without touching the real index.
fn snapshot(dir: &Path, scratch: &Path) -> String {
    let index = scratch.join("snapshot.index");
    let index = index.to_string_lossy().into_owned();
    let env = [("GIT_INDEX_FILE", index.as_str())];
    let _ = git(dir, &["add", "-A"], &env);
    let tree = git(dir, &["write-tree"], &env).unwrap_or_else(|| "0".repeat(40));
    let _ = fs::remove_file(&index);
    format!("git:{tree}")
}

/// Paths changed since `base`, including untracked files (ignored ones aside).
fn changed(dir: &Path, base: &str) -> Vec<String> {
    let base = base.strip_prefix("git:").unwrap_or(base);
    let mut out: BTreeSet<String> = git(dir, &["diff", "--name-only", base], &[])
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect();
    out.extend(
        git(dir, &["ls-files", "--others", "--exclude-standard"], &[])
            .unwrap_or_default()
            .lines()
            .map(String::from),
    );
    out.into_iter().collect()
}

struct Server {
    dir: PathBuf,
    ctx: McpContext,
    order: Value,
    graph: Value,
    self_checks: u32,
}

fn tool_list() -> Value {
    let obj = |props: Value, required: &[&str]| json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false });
    json!([
        { "name": "get_task", "description": "The order's subjects: obligations, owned outputs and entry points.", "inputSchema": obj(json!({}), &[]) },
        { "name": "get_declaration", "description": "A projection of a declaration from the bound graph, if the subjects' edges or module visibility allow it, with its hash.",
          "inputSchema": obj(json!({ "address": { "type": "string" }, "projection": { "enum": ["public", "behaviour", "context", "catalog"] } }), &["address", "projection"]) },
        { "name": "read_context", "description": "A manifest resource, paged; its bytes match the manifest digest.",
          "inputSchema": obj(json!({ "resource": { "type": "string", "pattern": "^ctx:[a-z0-9_-]+:[0-9a-f]{12}$" }, "cursor": { "type": ["string", "null"] } }), &["resource"]) },
        { "name": "get_diagnostic", "description": "The full log of an earlier failure, by the diagnostics handle in attempt.previous.",
          "inputSchema": obj(json!({ "receipt": { "type": "string" }, "cursor": { "type": ["string", "null"] } }), &["receipt"]) },
        { "name": "run_self_check", "description": "Runs a check marked self_check on the current tree. Advisory: it never counts as evidence.",
          "inputSchema": obj(json!({ "check": { "type": "string" } }), &["check"]) },
        { "name": "validate_result", "description": "Schema, binding, envelope, ownership and import problems of a draft result against the current tree.",
          "inputSchema": obj(json!({ "draft": { "type": "object" } }), &["draft"]) },
    ])
}

impl Server {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "get_task" => Ok(json!({
                "dispatch": self.ctx.dispatch,
                "subjects": self.ctx.subjects.iter().map(|s| json!({
                    "address": s.address, "obligations": s.obligations,
                    "outputs": s.outputs.iter().map(|(m, _)| m).collect::<Vec<_>>(), "exports": s.exports,
                })).collect::<Vec<_>>(),
            })),
            "get_declaration" => {
                let address = args["address"].as_str().ok_or("address is required")?;
                let projection = args["projection"]
                    .as_str()
                    .ok_or("projection is required")?;
                if !self.ctx.visible.contains(address) {
                    let exists = self.graph["declarations"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|d| d["address"] == address);
                    return Ok(
                        json!({ "error": if exists { "not_visible" } else { "not_found" } }),
                    );
                }
                let docs = intlang_canon::documents(&self.graph).map_err(|e| e.to_string())?;
                let Some(d) = docs.declarations.get(address) else {
                    return Ok(json!({ "error": "not_found" }));
                };
                let doc = match projection {
                    "public" => &d.public,
                    "behaviour" => &d.behaviour,
                    "context" => &d.context,
                    _ => return Ok(json!({ "error": "not_found" })),
                };
                Ok(
                    json!({ "address": address, "projection": projection, "hash": digest(doc).map_err(|e| e.to_string())?, "text": serde_json::to_string_pretty(doc).unwrap_or_default() }),
                )
            }
            "read_context" => {
                let resource = args["resource"].as_str().ok_or("resource is required")?;
                let (path, want) = self
                    .ctx
                    .resources
                    .get(resource)
                    .ok_or_else(|| format!("unknown resource {resource}"))?;
                let bytes = fs::read(self.dir.join(path)).map_err(|e| e.to_string())?;
                if intlang_canon::digest_bytes(&bytes) != *want {
                    return Err(format!("{resource} no longer matches its digest"));
                }
                let text = String::from_utf8_lossy(&bytes);
                let (content, next) = page(&text, args["cursor"].as_str());
                Ok(
                    json!({ "resource": resource, "digest": want, "content": content, "next": next }),
                )
            }
            "get_diagnostic" => {
                let receipt = args["receipt"].as_str().ok_or("receipt is required")?;
                let hex = receipt
                    .strip_prefix("ctx:log:")
                    .ok_or("a diagnostics handle looks like ctx:log:<hex>")?;
                if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err("bad handle".into());
                }
                let text =
                    fs::read_to_string(self.dir.join(&self.ctx.logs).join(format!("{hex}.log")))
                        .map_err(|_| format!("no log for {receipt}"))?;
                let (content, next) = page(&text, args["cursor"].as_str());
                Ok(json!({ "receipt": receipt, "content": content, "next": next }))
            }
            "run_self_check" => self.self_check(args["check"].as_str().ok_or("check is required")?),
            "validate_result" => Ok(self.validate(&args["draft"])),
            other => Err(format!("unknown tool {other}")),
        }
    }

    fn scratch(&self) -> PathBuf {
        let s = self
            .dir
            .join(format!(".spec/run/{}/scratch", self.ctx.dispatch));
        let _ = fs::create_dir_all(&s);
        s
    }

    fn self_check(&mut self, id: &str) -> Result<Value, String> {
        let allowed = self.order["checks"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| c["id"] == id && c["self_check"] == true);
        if !allowed {
            return Err(format!("{id} is not a self_check check of this order"));
        }
        self.self_checks += 1;
        let receipt = format!("self:{}:{}", self.ctx.dispatch, self.self_checks);
        let scratch = self.scratch();
        let snap = snapshot(&self.dir, &scratch);
        let graph = self.dir.join(&self.ctx.graph);
        let mut result = "passed";
        let mut excerpts = Vec::new();
        for (address, def) in self.ctx.checks.iter().filter(|(_, d)| d.id == id) {
            let subject = self.ctx.subjects.iter().find(|s| &s.address == address);
            let project_scope = def.scope == "project";
            let mut runs: Vec<SelfRun> = vec![(None, BTreeMap::new())];
            if def.purpose == "test" {
                runs = subject
                    .map(|s| {
                        s.outputs
                            .iter()
                            .filter(|(m, _)| {
                                m.rsplit('.').next().is_some_and(|k| k.contains("test"))
                            })
                            .map(|(m, p)| {
                                (
                                    Some((m.clone(), p.clone())),
                                    s.expected_cases.get(m).cloned().unwrap_or_default(),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            }
            for (test_output, expected_cases) in runs {
                let ctx = crate::checks::RunContext {
                    receipt: receipt.clone(),
                    workdir: &self.dir,
                    scratch: &scratch,
                    graph_path: &graph,
                    snapshot: &snap,
                    profile: &self.ctx.profile,
                    subject: (!project_scope).then_some(address.as_str()),
                    subject_kind: subject.map(|s| s.kind.as_str()),
                    outputs: if project_scope {
                        vec![]
                    } else {
                        subject.map(|s| s.outputs.clone()).unwrap_or_default()
                    },
                    test_output,
                    expected_cases,
                    inputs: digest(&json!(snap)).unwrap_or_default(),
                    timeout: Duration::from_secs(self.ctx.check_timeout_seconds),
                };
                let run = run_check(def, &ctx).map_err(|e| e.message)?;
                if !run.passed {
                    result = if run.classification == Some("infrastructure") && result == "passed" {
                        "error"
                    } else {
                        "failed"
                    };
                    excerpts.push(run.excerpt);
                }
            }
        }
        let mut excerpt = excerpts.join("\n");
        excerpt.truncate(4000);
        Ok(
            json!({ "receipt": receipt, "snapshot": snap, "result": result, "excerpt": excerpt, "advisory": true }),
        )
    }

    fn validate(&self, draft: &Value) -> Value {
        let mut problems = Vec::new();
        let mut problem = |stage: &str, path: Option<&str>, message: String| {
            let mut p = json!({ "stage": stage, "message": message });
            if let Some(path) = path {
                p["path"] = json!(path);
            }
            problems.push(p);
        };
        if draft["schema"] != "intlang.result/1" {
            problem(
                "schema",
                Some("schema"),
                "must be \"intlang.result/1\"".into(),
            );
        }
        if !matches!(draft["status"].as_str(), Some("submitted" | "blocked")) {
            problem(
                "schema",
                Some("status"),
                "must be \"submitted\" or \"blocked\"".into(),
            );
        }
        if !draft["summary"].is_string() {
            problem(
                "schema",
                Some("summary"),
                "a summary string is required".into(),
            );
        }
        if draft["dispatch"] != json!(self.ctx.dispatch) {
            problem(
                "binding",
                Some("dispatch"),
                format!("must be {}", self.ctx.dispatch),
            );
        }
        if draft["order_hash"] != self.order["binding"]["order_hash"] {
            problem(
                "binding",
                Some("order_hash"),
                format!("must be {}", self.order["binding"]["order_hash"]),
            );
        }
        let members: BTreeSet<&str> = self
            .ctx
            .subjects
            .iter()
            .flat_map(|s| s.requirements.iter().map(String::as_str))
            .collect();
        for t in draft["tests"].as_array().into_iter().flatten() {
            let m = t["subject"]["requirement"].as_str().unwrap_or_default();
            if !members.contains(m) {
                problem(
                    "schema",
                    Some("tests"),
                    format!("{m} is not a requirement of this order"),
                );
            }
        }
        let mut claimed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for o in draft["outputs"].as_array().into_iter().flatten() {
            let owner = o["owner"].as_str().unwrap_or_default();
            let Some(s) = self.ctx.subjects.iter().find(|s| s.address == owner) else {
                problem(
                    "ownership",
                    Some("outputs"),
                    format!("{owner} is not a subject of this order"),
                );
                continue;
            };
            let root = o["root"].as_str().unwrap_or_default();
            let rel = o["relative_path"].as_str().unwrap_or_default();
            match s.roots.get(root) {
                None => problem(
                    "ownership",
                    Some("outputs"),
                    format!("{owner} has no creation root {root:?}"),
                ),
                Some(_) if rel.is_empty() || rel.contains("..") || rel.starts_with('/') => problem(
                    "ownership",
                    Some("outputs"),
                    format!("{rel:?} is not a path under its root"),
                ),
                Some(path) => {
                    let full = format!("{path}{rel}");
                    if o["op"] != "remove" && !self.dir.join(&full).is_file() {
                        problem(
                            "ownership",
                            Some("outputs"),
                            format!("{full} does not exist"),
                        );
                    }
                    claimed.entry(owner.to_string()).or_default().insert(full);
                }
            }
        }
        let changed = changed(&self.dir, &self.ctx.base);
        let writable: BTreeSet<String> = self
            .ctx
            .subjects
            .iter()
            .flat_map(|s| s.outputs.iter().map(|(_, p)| p.clone()))
            .chain(claimed.values().flatten().cloned())
            .collect();
        for path in &changed {
            if !writable.contains(path) {
                let under_root = self
                    .ctx
                    .subjects
                    .iter()
                    .any(|s| s.roots.values().any(|r| path.starts_with(r.as_str())));
                let stage = if under_root { "ownership" } else { "envelope" };
                problem(
                    stage,
                    Some(path),
                    format!(
                        "{path} changed but is {}",
                        if under_root {
                            "not claimed in outputs"
                        } else {
                            "outside the envelope"
                        }
                    ),
                );
            }
        }
        for s in &self.ctx.subjects {
            let mut policy = s.imports.clone();
            policy
                .own
                .extend(claimed.get(&s.address).into_iter().flatten().cloned());
            let files: Vec<String> = changed
                .iter()
                .filter(|p| policy.own.contains(*p))
                .cloned()
                .collect();
            for v in check_imports(&policy, &self.dir, &files) {
                problem("imports", None, v);
            }
        }
        let snap = snapshot(&self.dir, &self.scratch());
        json!({ "snapshot": snap, "valid": problems.is_empty(), "problems": problems })
    }
}

/// Serves the tools of `dispatch` for the candidate at `dir` until `input` closes.
pub fn serve(
    dir: &Path,
    dispatch: &str,
    input: impl BufRead,
    mut output: impl Write,
) -> AResult<()> {
    let ctx_path = dir.join(format!(".spec/run/{dispatch}/mcp.json"));
    let text = fs::read_to_string(&ctx_path)
        .map_err(|e| ApplyError::new(format!("no protocol area for {dispatch}: {e}")))?;
    let ctx: McpContext = serde_json::from_str(&text)
        .map_err(|e| ApplyError::new(format!("{}: {e}", ctx_path.display())))?;
    let read = |rel: &str| -> AResult<Value> {
        serde_json::from_str(
            &fs::read_to_string(dir.join(rel))
                .map_err(|e| ApplyError::new(format!("{rel}: {e}")))?,
        )
        .map_err(|e| ApplyError::new(format!("{rel}: {e}")))
    };
    let order = read(&ctx.order)?;
    let graph = read(&ctx.graph)?;
    let mut server = Server {
        dir: dir.to_path_buf(),
        ctx,
        order,
        graph,
        self_checks: 0,
    };
    for line in input.lines() {
        let line = line.map_err(|e| ApplyError::new(e.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Err(e) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": e.to_string() } }),
            ),
            Ok(msg) => {
                let id = msg.get("id").cloned();
                let method = msg["method"].as_str().unwrap_or_default();
                let result: Result<Value, (i64, String)> = match method {
                    "initialize" => Ok(json!({
                        "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "spec", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": format!("Tools for dispatch {}. They read the bound graph and the order; none of them writes state or widens your envelope.", server.ctx.dispatch),
                    })),
                    "ping" => Ok(json!({})),
                    "tools/list" => Ok(json!({ "tools": tool_list() })),
                    "tools/call" => {
                        let name = msg["params"]["name"].as_str().unwrap_or_default();
                        let args = msg["params"].get("arguments").cloned().unwrap_or(json!({}));
                        let answer = server.call(name, &args);
                        // An audit trail of which tools the agent used.
                        let log = dir.join(format!(".spec/run/{dispatch}/mcp-calls.jsonl"));
                        if let Ok(mut f) =
                            fs::OpenOptions::new().create(true).append(true).open(log)
                        {
                            let _ =
                                writeln!(f, "{}", json!({ "tool": name, "ok": answer.is_ok() }));
                        }
                        Ok(match answer {
                            Ok(v) => {
                                json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() }], "structuredContent": v, "isError": false })
                            }
                            Err(e) => {
                                json!({ "content": [{ "type": "text", "text": e }], "isError": true })
                            }
                        })
                    }
                    m if m.starts_with("notifications/") => {
                        continue;
                    }
                    other => Err((-32601, format!("method not found: {other}"))),
                };
                id.map(|id| match result {
                    Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
                    Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
                })
            }
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}").map_err(|e| ApplyError::new(e.to_string()))?;
            output.flush().map_err(|e| ApplyError::new(e.to_string()))?;
        }
    }
    Ok(())
}
