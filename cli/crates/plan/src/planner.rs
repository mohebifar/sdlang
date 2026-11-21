//! The plan: identity, drift, the change set, propagation, transactions and the binding
//! (SPEC.md, State, plan and apply). Reads the new graph, the state records, the applied
//! store and the owned files; never an old commit and never package code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use intlang_canon::{digest, documents, GraphDocuments};
use intlang_graph::loader::Project;
use serde_json::{json, Map, Value};

use crate::compat::{classify, Compatibility};
use crate::error::{PResult, PlanError};
use crate::materialise::{materialise, Materialisation};
use crate::state::{
    applied_graph_digest, declarations, file_digest, member_hashes, StateRecord, Store, OBSERVING,
    ORDERING,
};

/// What `spec plan` reads besides the files under the project root.
pub struct PlanInputs<'a> {
    pub root: &'a Path,
    pub project: &'a Project,
    pub graph: &'a Value,
    pub plan_id: &'a str,
    /// `git:<object id>` of the commit the plan starts from.
    pub baseline_commit: &'a str,
    /// New declarations `spec import --accept` adopts, by address (strategy `acquire`).
    pub acquire: Option<&'a BTreeMap<String, Acquisition>>,
}

/// What an accepted import claims for one new declaration: existing files only. Its JSON
/// form is the plan entry's `acquire` object; `.spec/import.json` holds one per address
/// until the declaration publishes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Acquisition {
    /// Slot keys whose existing files become the baseline.
    pub slots: BTreeSet<String>,
    /// Created outputs: key -> (creation root key, path relative to the root).
    pub created: BTreeMap<String, (String, String)>,
    /// Proposed test bindings: requirement member -> `(test output member, case)`.
    pub tests: BTreeMap<String, Vec<(String, String)>>,
}

impl Acquisition {
    pub fn to_value(&self) -> Value {
        let created: Map<String, Value> = self
            .created
            .iter()
            .map(|(k, (root, rel))| (k.clone(), json!({ "root": root, "relative_path": rel })))
            .collect();
        let tests: Map<String, Value> = self
            .tests
            .iter()
            .map(|(member, ts)| {
                let list = ts
                    .iter()
                    .map(|(o, c)| json!({ "output": o, "case": c }))
                    .collect();
                (member.clone(), Value::Array(list))
            })
            .collect();
        json!({ "slots": self.slots, "created": created, "tests": tests })
    }

    pub fn from_value(v: &Value) -> Acquisition {
        let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
        Acquisition {
            slots: v["slots"]
                .as_array()
                .into_iter()
                .flatten()
                .map(text)
                .collect(),
            created: v["created"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, o)| (k.clone(), (text(&o["root"]), text(&o["relative_path"]))))
                .collect(),
            tests: v["tests"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(m, ts)| {
                    let list = ts
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|t| (text(&t["output"]), text(&t["case"])))
                        .collect();
                    (m.clone(), list)
                })
                .collect(),
        }
    }
}

/// Where accepted imports wait for their acquire transaction.
pub const PENDING_IMPORTS: &str = ".spec/import.json";

/// The accepted imports not yet published, by address; empty when there are none.
pub fn pending_imports(root: &Path) -> PResult<BTreeMap<String, Acquisition>> {
    let path = root.join(PENDING_IMPORTS);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(BTreeMap::new());
    };
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| PlanError::new(format!("{PENDING_IMPORTS}: {e}")))?;
    if v["schema"] != "intlang.import/1" {
        return Err(PlanError::new(format!(
            "{PENDING_IMPORTS}: not an import document (schema intlang.import/1)"
        )));
    }
    Ok(v["acquire"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(a, v)| (a.clone(), Acquisition::from_value(v)))
        .collect())
}

/// Writes the pending imports, removing the file when there are none.
pub fn write_pending_imports(root: &Path, pending: &BTreeMap<String, Acquisition>) -> PResult<()> {
    let path = root.join(PENDING_IMPORTS);
    if pending.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let acquire: Map<String, Value> = pending
        .iter()
        .map(|(a, q)| (a.clone(), q.to_value()))
        .collect();
    let doc = json!({ "schema": "intlang.import/1", "acquire": acquire });
    let text = serde_json::to_string_pretty(&doc).expect("serialisable") + "\n";
    std::fs::create_dir_all(path.parent().expect("under .spec"))
        .and_then(|_| std::fs::write(&path, text))
        .map_err(|e| PlanError::new(format!("{PENDING_IMPORTS}: {e}")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Change {
    None,
    Create,
    Update,
    Delete,
}

impl Change {
    fn as_str(self) -> &'static str {
        match self {
            Change::None => "none",
            Change::Create => "create",
            Change::Update => "update",
            Change::Delete => "delete",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Obligation {
    because: String,
    kind: &'static str,
    compatibility: Option<String>,
    /// The changed declaration that caused it; empty for inherited context.
    source: String,
}

#[derive(Clone, Debug)]
struct Entry {
    kind: String,
    identity: &'static str,
    moved_from: Option<String>,
    lifecycle: String,
    change: Change,
    causes: BTreeSet<&'static str>,
    replace: bool,
    obligations: BTreeSet<Obligation>,
    /// Changed declarations whose transaction this update joins.
    joins: BTreeSet<String>,
    acquire: Option<Acquisition>,
}

impl Entry {
    fn new(kind: &str, lifecycle: &str) -> Entry {
        Entry {
            kind: kind.to_string(),
            identity: "same",
            moved_from: None,
            lifecycle: lifecycle.to_string(),
            change: Change::None,
            causes: BTreeSet::new(),
            replace: false,
            obligations: BTreeSet::new(),
            joins: BTreeSet::new(),
            acquire: None,
        }
    }

    fn update(&mut self, cause: &'static str) {
        if self.change == Change::None {
            self.change = Change::Update;
        }
        self.causes.insert(cause);
    }

    fn oblige(
        &mut self,
        kind: &'static str,
        because: String,
        source: &str,
        compatibility: Option<String>,
    ) {
        self.obligations.insert(Obligation {
            because,
            kind,
            compatibility,
            source: source.to_string(),
        });
    }
}

/// Tie-break rank for ordering transactions: data, then logic, then the rest.
fn rank(kind: &str) -> u8 {
    match kind {
        "core:entity" | "core:use" => 0,
        "core:type" | "core:action" | "core:lib" => 1,
        _ => 2,
    }
}

fn projection<'a>(docs: &'a GraphDocuments, address: &str, projection: &str) -> Option<&'a Value> {
    let d = docs.declarations.get(address)?;
    Some(match projection {
        "public" => &d.public,
        "behaviour" => &d.behaviour,
        "body" => &d.body,
        "context" => &d.context,
        _ => return None,
    })
}

struct UnionFind(BTreeMap<String, String>);

impl UnionFind {
    fn find(&mut self, a: &str) -> String {
        let parent = self.0.get(a).cloned().unwrap_or_else(|| a.to_string());
        if parent == a {
            return parent;
        }
        let root = self.find(&parent);
        self.0.insert(a.to_string(), root.clone());
        root
    }

    fn union(&mut self, a: &str, b: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.0.insert(hi, lo);
        }
    }
}

/// Strongly connected components of a small directed graph (Tarjan, iterative enough
/// for plan sizes).
fn components(nodes: &[String], edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    struct S<'e> {
        edges: &'e BTreeMap<String, BTreeSet<String>>,
        index: BTreeMap<String, usize>,
        low: BTreeMap<String, usize>,
        stack: Vec<String>,
        on: BTreeSet<String>,
        n: usize,
        out: Vec<Vec<String>>,
    }
    fn visit(s: &mut S, v: &str) {
        s.index.insert(v.to_string(), s.n);
        s.low.insert(v.to_string(), s.n);
        s.n += 1;
        s.stack.push(v.to_string());
        s.on.insert(v.to_string());
        let next: Vec<String> = s.edges.get(v).into_iter().flatten().cloned().collect();
        for w in next {
            if !s.index.contains_key(&w) {
                visit(s, &w);
                let lw = s.low[&w];
                let lv = s.low.get_mut(v).unwrap();
                *lv = (*lv).min(lw);
            } else if s.on.contains(&w) {
                let iw = s.index[&w];
                let lv = s.low.get_mut(v).unwrap();
                *lv = (*lv).min(iw);
            }
        }
        if s.low[v] == s.index[v] {
            let mut c = Vec::new();
            loop {
                let w = s.stack.pop().unwrap();
                s.on.remove(&w);
                c.push(w.clone());
                if w == v {
                    break;
                }
            }
            c.sort();
            s.out.push(c);
        }
    }
    let mut s = S {
        edges,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on: BTreeSet::new(),
        n: 0,
        out: Vec::new(),
    };
    for v in nodes {
        if !s.index.contains_key(v) {
            visit(&mut s, v);
        }
    }
    s.out
}

/// Plans the project's change and returns the plan document (`schemas/plan.schema.json`).
pub fn plan(inputs: &PlanInputs) -> PResult<Value> {
    let graph = inputs.graph;
    let docs = documents(graph)?;
    let mats = materialise(inputs.project, graph)?;
    let store = Store::new(inputs.root);
    let records = store.records()?;
    let decls = declarations(graph);
    let managed: Vec<&str> = mats.keys().map(String::as_str).collect();
    for (address, a) in inputs.acquire.into_iter().flatten() {
        let Some(m) = mats.get(address) else {
            return Err(PlanError::new(format!(
                "cannot acquire {address}: not a managed declaration"
            )));
        };
        if records.contains_key(address) {
            return Err(PlanError::new(format!(
                "cannot acquire {address}: it already has a state record"
            )));
        }
        for slot in &a.slots {
            if !m.slots.contains_key(slot) {
                return Err(PlanError::new(format!(
                    "cannot acquire {address}#outputs.{slot}: no such slot"
                )));
            }
        }
        for (key, (root, _)) in &a.created {
            if !m.roots.contains_key(root) || m.slots.contains_key(key) {
                return Err(PlanError::new(format!(
                    "cannot acquire {address}#outputs.{key} under root {root}"
                )));
            }
        }
    }

    // Identity: moved statements map old addresses to new ones.
    let mut moved_from: BTreeMap<String, String> = BTreeMap::new();
    for mv in graph["moves"].as_array().into_iter().flatten() {
        let (Some(from), Some(to), Some(scope)) =
            (mv["from"].as_str(), mv["to"].as_str(), mv["scope"].as_str())
        else {
            continue;
        };
        match scope {
            "declaration" if records.contains_key(from) => {
                moved_from.insert(to.to_string(), from.to_string());
            }
            "module" => {
                for old in records.keys() {
                    if let Some(rest) = old.strip_prefix(&format!("{from}.")) {
                        moved_from.insert(format!("{to}.{rest}"), old.clone());
                    }
                }
            }
            _ => {}
        }
    }
    let new_of: BTreeMap<String, String> = moved_from
        .iter()
        .map(|(n, o)| (o.clone(), n.clone()))
        .collect();
    let current = |old: &str| new_of.get(old).cloned().unwrap_or_else(|| old.to_string());
    let record_of = |address: &str| -> Option<&StateRecord> {
        records.get(
            moved_from
                .get(address)
                .map(String::as_str)
                .unwrap_or(address),
        )
    };

    // Drift: an owned output whose bytes differ from the record blocks a managed
    // declaration.
    let mut blocked: BTreeMap<String, Value> = BTreeMap::new();
    let mut fingerprints: BTreeMap<String, String> = BTreeMap::new();
    for &address in &managed {
        let decl = decls[address];
        let Some(record) = record_of(address) else {
            continue;
        };
        if decl["lifecycle"] == "frozen" {
            continue;
        }
        let mut outputs: Vec<_> = record.outputs.values().collect();
        outputs.sort_by(|a, b| a.path.cmp(&b.path));
        for out in outputs {
            let actual = file_digest(inputs.root, &out.path)?;
            if actual.as_deref() == Some(out.bytes.as_str()) {
                continue;
            }
            let mut b = json!({
                "address": address, "reason": "drift", "path": out.path, "expected": out.bytes,
                "resolutions": ["adopt", "absorb", "overwrite", "repair"],
            });
            if let Some(actual) = actual {
                b["actual"] = json!(actual);
                fingerprints.insert(out.path.clone(), actual);
            }
            blocked.insert(address.to_string(), b);
            break;
        }
    }

    // Own changes, identity and paths.
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    for &address in &managed {
        if blocked.contains_key(address) {
            continue;
        }
        let decl = decls[address];
        let kind = decl["kind"].as_str().unwrap_or_default();
        let mut e = Entry::new(kind, decl["lifecycle"].as_str().unwrap_or("managed"));
        match record_of(address) {
            None => {
                e.identity = "new";
                e.change = Change::Create;
                e.causes.insert("new");
                e.acquire = inputs.acquire.and_then(|a| a.get(address)).cloned();
            }
            Some(r) => {
                if let Some(from) = moved_from.get(address) {
                    e.identity = "moved";
                    e.moved_from = Some(from.clone());
                }
                let now = docs.declarations[address].hashes()?;
                if r.kind != kind {
                    e.update("own_projection");
                    e.replace = true;
                } else if now.public != r.hashes.public
                    || now.behaviour != r.hashes.behaviour
                    || now.body != r.hashes.body
                {
                    e.update("own_projection");
                }
                let slots = &mats[address].slots;
                let recorded: BTreeMap<&String, &String> = r
                    .outputs
                    .iter()
                    .filter(|(_, o)| o.origin == "slot")
                    .map(|(k, o)| (k, &o.path))
                    .collect();
                let wanted: BTreeMap<&String, &String> = slots.iter().collect();
                if recorded != wanted {
                    e.update("convention");
                }
            }
        }
        entries.insert(address.to_string(), e);
    }
    let gone: Vec<&StateRecord> = records
        .values()
        .filter(|r| !decls.contains_key(r.address.as_str()) && !new_of.contains_key(&r.address))
        .collect();
    for r in &gone {
        let mut e = Entry::new(&r.kind, "managed");
        e.identity = "gone";
        e.change = Change::Delete;
        entries.insert(r.address.clone(), e);
    }
    // A frozen declaration that needs an update blocks rather than changes.
    let freeze = |entries: &mut BTreeMap<String, Entry>, blocked: &mut BTreeMap<String, Value>| {
        let frozen: Vec<String> = entries
            .iter()
            .filter(|(_, e)| e.lifecycle == "frozen" && e.change == Change::Update)
            .map(|(a, _)| a.clone())
            .collect();
        for a in frozen {
            entries.remove(&a);
            blocked.insert(a.clone(), json!({ "address": a, "reason": "frozen" }));
        }
    };
    freeze(&mut entries, &mut blocked);

    // Propagation from what each record consumed.
    for &address in &managed {
        if blocked.contains_key(address) {
            continue;
        }
        let Some(r) = record_of(address) else {
            continue;
        };
        let mut e = entries
            .remove(address)
            .expect("every managed declaration has an entry");
        let mut collected_then = BTreeSet::new();
        for c in &r.consumed {
            let target = current(&c.target);
            if c.relation == "collects" {
                collected_then.insert(target.clone());
            }
            if blocked.contains_key(&target) {
                continue;
            }
            let Some(now) = projection(&docs, &target, &c.projection) else {
                continue;
            };
            if digest(now)? == c.hash {
                continue;
            }
            match (c.relation.as_str(), c.projection.as_str()) {
                ("collects", _) => {
                    e.update("aggregate");
                    e.joins.insert(target.clone());
                }
                (relation, "public") if ORDERING.contains(&relation) => {
                    let verdict = match store.applied(&c.hash)? {
                        Some(old) => classify(&old, now, relation),
                        None => Compatibility::Incompatible,
                    };
                    match verdict {
                        Compatibility::Incompatible => {
                            e.update("consumed_incompatible");
                            e.joins.insert(target.clone());
                        }
                        Compatibility::Compatible(label) => e.oblige(
                            "verify",
                            format!("consumed:{target}"),
                            &target,
                            Some(label.to_string()),
                        ),
                    }
                }
                (relation, _) if OBSERVING.contains(&relation) => {
                    e.oblige("verify", format!("observed:{target}"), &target, None);
                    e.oblige("review", format!("observed:{target}"), &target, None);
                }
                _ => {}
            }
        }
        let collected_now: BTreeSet<String> = decls[address]["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|edge| edge["relation"] == "collects")
            .filter_map(|edge| edge["target"].as_str().map(String::from))
            .collect();
        for changed in collected_now.symmetric_difference(&collected_then) {
            e.update("aggregate");
            e.joins.insert(changed.clone());
        }
        // Inherited normative context, part by part.
        let now_parts: BTreeMap<&String, String> = docs.declarations[address]
            .context_parts
            .iter()
            .map(|(s, d)| Ok((s, digest(d)?)))
            .collect::<PResult<_>>()?;
        let sources: BTreeSet<&String> = now_parts
            .keys()
            .copied()
            .chain(r.context_parts.keys())
            .collect();
        for source in sources {
            if now_parts.get(source) != r.context_parts.get(source) {
                e.oblige("review", format!("context:{source}"), "", None);
                e.oblige("verify", format!("context:{source}"), "", None);
            }
        }
        // Evidence that never met its policy (an acquired declaration's, say). Approval
        // needs a human, so a plan cannot discharge it.
        for ev in &r.evidence {
            let policy = &ev["policy"];
            if ev["summary"] != "unverified" || policy["approval"] == true {
                continue;
            }
            let member = ev["subject"]["requirement"].as_str().unwrap_or_default();
            let because = format!("unverified:{member}");
            let tests = policy["tests"] == true;
            if tests || policy["checks"].as_array().is_some_and(|c| !c.is_empty()) {
                e.oblige("verify", because.clone(), "", None);
            }
            if tests || policy["review"] == true {
                e.oblige("review", because, "", None);
            }
        }
        entries.insert(address.to_string(), e);
    }
    freeze(&mut entries, &mut blocked);

    // Implementation impact: every create or update gives verify to its transitive
    // dependents over ordering relations.
    let mut dependents: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (&address, decl) in &decls {
        for edge in decl["edges"].as_array().into_iter().flatten() {
            let relation = edge["relation"].as_str().unwrap_or_default();
            if let Some(target) = edge["target"].as_str() {
                if ORDERING.contains(&relation) && decls.contains_key(target) {
                    dependents.entry(target).or_default().insert(address);
                }
            }
        }
    }
    let changed_now: Vec<String> = entries
        .iter()
        .filter(|(_, e)| matches!(e.change, Change::Create | Change::Update))
        .map(|(a, _)| a.clone())
        .collect();
    for source in &changed_now {
        let mut seen = BTreeSet::new();
        let mut todo: Vec<&str> = dependents
            .get(source.as_str())
            .into_iter()
            .flatten()
            .copied()
            .collect();
        while let Some(a) = todo.pop() {
            if !seen.insert(a) {
                continue;
            }
            todo.extend(dependents.get(a).into_iter().flatten().copied());
            if !mats.contains_key(a) || blocked.contains_key(a) {
                continue;
            }
            let Some(e) = entries.get_mut(a) else {
                continue;
            };
            let already = e
                .obligations
                .iter()
                .any(|o| o.kind == "verify" && o.source == *source);
            if !already {
                e.oblige("verify", format!("impact:{source}"), source, None);
            }
        }
    }

    // Precedence: an update absorbs obligations; entries with nothing to do go.
    entries.retain(|_, e| {
        if e.change != Change::None {
            e.obligations.clear();
        }
        e.change != Change::None || !e.obligations.is_empty() || e.identity == "moved"
    });

    // Transactions: each change roots one; updates join what caused them; cycles merge.
    // A pure move changes nothing but still publishes its re-keyed record.
    let changed: BTreeSet<String> = entries
        .iter()
        .filter(|(_, e)| e.change != Change::None || e.identity == "moved")
        .map(|(a, _)| a.clone())
        .collect();
    let mut uf = UnionFind(BTreeMap::new());
    for a in &changed {
        for j in &entries[a].joins {
            if changed.contains(j) {
                uf.union(a, j);
            }
        }
    }
    // Former consumers of a deleted declaration, from the old edges in state.
    let mut former: Vec<(String, String)> = Vec::new(); // (consumer, deleted)
    for r in records.values() {
        for c in &r.consumed {
            if entries
                .get(&c.target)
                .is_some_and(|e| e.change == Change::Delete)
            {
                let consumer = current(&r.address);
                if changed.contains(&consumer) && consumer != c.target {
                    former.push((consumer, c.target.clone()));
                }
            }
        }
    }
    former.sort();
    former.dedup();
    for (consumer, deleted) in &former {
        uf.union(consumer, deleted);
    }
    let order_edges = |uf: &mut UnionFind| -> BTreeMap<String, BTreeSet<String>> {
        let mut after: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for a in &changed {
            if entries[a].change == Change::Delete {
                continue;
            }
            for edge in decls[a.as_str()]["edges"].as_array().into_iter().flatten() {
                let relation = edge["relation"].as_str().unwrap_or_default();
                let Some(target) = edge["target"].as_str() else {
                    continue;
                };
                if ORDERING.contains(&relation) && changed.contains(target) {
                    let (ga, gt) = (uf.find(a), uf.find(target));
                    if ga != gt {
                        after.entry(ga).or_default().insert(gt);
                    }
                }
            }
        }
        // Deletions run in reverse: after their former consumers.
        for (consumer, deleted) in &former {
            let (gd, gc) = (uf.find(deleted), uf.find(consumer));
            if gd != gc {
                after.entry(gd).or_default().insert(gc);
            }
        }
        after
    };
    let roots: Vec<String> = {
        let mut r: BTreeSet<String> = BTreeSet::new();
        for a in &changed {
            r.insert(uf.find(a));
        }
        r.into_iter().collect()
    };
    for component in components(&roots, &order_edges(&mut uf)) {
        for w in component.windows(2) {
            uf.union(&w[0], &w[1]);
        }
    }
    let after = order_edges(&mut uf);
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in &changed {
        groups.entry(uf.find(a)).or_default().push(a.clone());
    }
    let key = |a: &String| (rank(&entries[a].kind), a.clone());
    // Within a transaction, members follow ordering edges too (deletions in reverse,
    // after their former consumers); cycles fall back to the tie-break key.
    let depends_on = |a: &String, b: &String| -> bool {
        if entries[a].change == Change::Delete {
            return former.iter().any(|(c, d)| d == a && c == b);
        }
        decls.get(a.as_str()).is_some_and(|d| {
            d["edges"].as_array().into_iter().flatten().any(|e| {
                ORDERING.contains(&e["relation"].as_str().unwrap_or_default())
                    && e["target"] == json!(b)
            })
        })
    };
    for members in groups.values_mut() {
        members.sort_by_key(key);
        let mut left: Vec<String> = members.clone();
        let mut sorted = Vec::new();
        while !left.is_empty() {
            let i = left
                .iter()
                .position(|a| !left.iter().any(|b| b != a && depends_on(a, b)))
                .unwrap_or(0);
            sorted.push(left.remove(i));
        }
        *members = sorted;
    }
    // Kahn's algorithm with the tie-break key of each group's first member.
    let mut ordered: Vec<String> = Vec::new();
    let mut remaining: BTreeSet<String> = groups.keys().cloned().collect();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .filter(|g| {
                after
                    .get(*g)
                    .into_iter()
                    .flatten()
                    .all(|dep| !remaining.contains(dep))
            })
            .min_by_key(|g| key(&groups[*g][0]))
            .cloned()
            .expect("cycles were merged");
        remaining.remove(&next);
        ordered.push(next);
    }
    let tx_id: BTreeMap<String, String> = ordered
        .iter()
        .enumerate()
        .map(|(i, g)| (g.clone(), format!("tx-{}", i + 1)))
        .collect();

    // Obligations attach to every transaction that caused them; obligations with no
    // changed source (inherited context, or an observed target whose own context
    // changed) share one obligation-only transaction, run last.
    let mut tx_obligations: BTreeMap<String, BTreeMap<String, BTreeSet<&'static str>>> =
        BTreeMap::new();
    let mut primary: BTreeMap<String, String> = BTreeMap::new();
    let mut obligation_only: Vec<String> = Vec::new();
    for (address, e) in entries.iter().filter(|(a, _)| !changed.contains(*a)) {
        let mut attached: BTreeSet<usize> = BTreeSet::new();
        for o in &e.obligations {
            if changed.contains(&o.source) {
                let g = uf.find(&o.source);
                let index = ordered.iter().position(|x| *x == g).expect("ordered");
                attached.insert(index);
                tx_obligations
                    .entry(tx_id[&g].clone())
                    .or_default()
                    .entry(address.clone())
                    .or_default()
                    .insert(o.kind);
            }
        }
        match attached.first() {
            Some(&first) => {
                primary.insert(address.clone(), tx_id[&ordered[first]].clone());
            }
            None => obligation_only.push(address.clone()),
        }
    }
    let mut transactions: Vec<Value> = Vec::new();
    let mut entry_order: Vec<String> = Vec::new();
    let contingent = |address: &str| -> Vec<String> {
        let mut paths: Vec<String> = record_of(address)
            .map(|r| r.outputs.values().map(|o| o.path.clone()).collect())
            .unwrap_or_default();
        paths.sort();
        paths
    };
    for g in &ordered {
        let id = tx_id[g].clone();
        let members = &groups[g];
        entry_order.extend(members.iter().cloned());
        let mut deps: Vec<String> = after
            .get(g)
            .into_iter()
            .flatten()
            .map(|d| tx_id[d].clone())
            .collect();
        deps.sort_by_key(|t| t["tx-".len()..].parse::<usize>().unwrap_or(0));
        let mut obligations = Map::new();
        for (address, kinds) in tx_obligations.get(&id).into_iter().flatten() {
            obligations.insert(
                address.clone(),
                json!({ "kinds": kinds.iter().collect::<Vec<_>>(), "contingent": contingent(address) }),
            );
            if primary[address] == id {
                entry_order.push(address.clone());
            }
        }
        transactions.push(
            json!({ "id": id, "entries": members, "after": deps, "obligations": obligations }),
        );
    }
    if !obligation_only.is_empty() {
        let id = format!("tx-{}", transactions.len() + 1);
        let mut obligations = Map::new();
        for address in &obligation_only {
            let kinds: BTreeSet<&str> = entries[address]
                .obligations
                .iter()
                .map(|o| o.kind)
                .collect();
            obligations.insert(
                address.clone(),
                json!({ "kinds": kinds, "contingent": contingent(address) }),
            );
            primary.insert(address.clone(), id.clone());
            entry_order.push(address.clone());
        }
        transactions
            .push(json!({ "id": id, "entries": [], "after": [], "obligations": obligations }));
    }
    let tx_of_change: BTreeMap<&String, String> = changed
        .iter()
        .map(|a| (a, tx_id[&uf.find(a)].clone()))
        .collect();

    // Entries, coverage reviews and fingerprints.
    let mut entries_out = Vec::new();
    let mut coverage: BTreeSet<String> = BTreeSet::new();
    for address in &entry_order {
        let e = &entries[address];
        let mut out = json!({
            "address": address,
            "identity": e.identity,
            "change": e.change.as_str(),
            "transaction": tx_of_change.get(address).cloned().unwrap_or_else(|| primary[address].clone()),
        });
        if let Some(from) = &e.moved_from {
            out["moved_from"] = json!(from);
        }
        if e.lifecycle != "managed" {
            out["lifecycle"] = json!(e.lifecycle);
        }
        if !e.causes.is_empty() {
            out["causes"] = json!(e.causes);
        }
        match e.change {
            Change::None => {}
            Change::Delete => out["strategy"] = json!("none"),
            Change::Create | Change::Update => {
                let strategy = if e.acquire.is_some() {
                    "acquire"
                } else if e.replace {
                    "replace"
                } else if mats
                    .get(address)
                    .and_then(|m: &Materialisation| m.generator.as_ref())
                    .is_some()
                {
                    "generator"
                } else {
                    "agent"
                };
                out["strategy"] = json!(strategy);
                if let Some(a) = &e.acquire {
                    // Acquisition claims existing files: bind their bytes; nothing is
                    // judged until the next plan's obligations.
                    let m = &mats[address];
                    for (root, rel) in a.created.values() {
                        let path = format!("{}{rel}", m.roots[root]);
                        if let Some(d) = file_digest(inputs.root, &path)? {
                            fingerprints.insert(path, d);
                        }
                    }
                    for slot in &a.slots {
                        let path = &m.slots[slot];
                        if let Some(d) = file_digest(inputs.root, path)? {
                            fingerprints.insert(path.clone(), d);
                        }
                    }
                    out["acquire"] = a.to_value();
                } else {
                    let before = record_of(address)
                        .map(|r| r.members.clone())
                        .unwrap_or_default();
                    for (member, hash) in member_hashes(decls[address.as_str()])? {
                        if before.get(&member) != Some(&hash) {
                            coverage.insert(format!("{address}#{member}"));
                        }
                    }
                }
            }
        }
        if !e.obligations.is_empty() {
            out["obligations"] = Value::Array(
                e.obligations
                    .iter()
                    .map(|o| {
                        let mut v = json!({ "kind": o.kind, "because": o.because });
                        if let Some(c) = &o.compatibility {
                            v["compatibility"] = json!(c);
                        }
                        v
                    })
                    .collect(),
            );
        }
        if let Some(r) = records.get(e.moved_from.as_deref().unwrap_or(address)) {
            for o in r.outputs.values() {
                if let Some(d) = file_digest(inputs.root, &o.path)? {
                    fingerprints.insert(o.path.clone(), d);
                }
            }
        }
        entries_out.push(out);
    }

    if inputs.acquire.is_some_and(|a| !a.is_empty()) {
        if let Some(d) = file_digest(inputs.root, PENDING_IMPORTS)? {
            fingerprints.insert(PENDING_IMPORTS.to_string(), d);
        }
    }
    let binding = json!({
        "baseline_commit": inputs.baseline_commit,
        "graph_before": applied_graph_digest(&records)?,
        "graph_after": digest(graph)?,
        "lock_hash": digest(&inputs.project.lock)?,
        "state_digest": store.state_digest()?,
        "fingerprints": fingerprints,
    });
    Ok(json!({
        "schema": "intlang.plan/1",
        "plan": inputs.plan_id,
        "revision": 1,
        "binding": binding,
        "entries": entries_out,
        "blocked": blocked.into_values().collect::<Vec<_>>(),
        "transactions": transactions,
        "reviews": { "coverage": coverage },
    }))
}
