//! State records (`.spec/state/<address>.json`) and the applied store
//! (`.spec/applied/<hex>.json`), as SPEC.md's State record defines them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use intlang_canon::{canonical, digest, digest_bytes, GraphDocuments};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{PResult, PlanError};
use crate::materialise::Materialisation;

/// Relations that order work and consume the target's public projection.
pub const ORDERING: &[&str] = &["calls", "reads", "writes", "uses", "types", "requires"];
/// Relations that also observe the target's behaviour and normative context.
pub const OBSERVING: &[&str] = &["calls", "reads", "writes"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hashes {
    pub public: String,
    pub behaviour: String,
    pub body: String,
    pub context: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Consumed {
    pub relation: String,
    pub target: String,
    pub projection: String,
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    /// `slot` or `created`.
    pub origin: String,
    pub path: String,
    /// The exact digest of the accepted bytes.
    pub bytes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
}

/// One managed declaration's accepted baseline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateRecord {
    pub address: String,
    pub kind: String,
    /// The digest of the declaration's canonical graph entry as applied, kept in the
    /// applied store so generators receive the old entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// The digest of `{ "source": <the declaration's source text> }` as applied, kept in
    /// the applied store so the next work order can show the previous spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub hashes: Hashes,
    /// Inherited normative source -> hash.
    pub context_parts: BTreeMap<String, String>,
    pub consumed: Vec<Consumed>,
    /// `expect.<key>` / `errors.<key>` -> contract hash.
    pub members: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, Output>,
    #[serde(default)]
    pub evidence: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Value>,
}

/// A project's `.spec` directory.
pub struct Store {
    root: PathBuf,
}

fn io(path: &Path, e: std::io::Error) -> PlanError {
    PlanError::new(format!("{}: {e}", path.display()))
}

impl Store {
    pub fn new(project_root: &Path) -> Store {
        Store {
            root: project_root.to_path_buf(),
        }
    }

    fn state_dir(&self) -> PathBuf {
        self.root.join(".spec/state")
    }

    fn applied_dir(&self) -> PathBuf {
        self.root.join(".spec/applied")
    }

    /// Every record, keyed by address. A missing state directory is an empty state.
    pub fn records(&self) -> PResult<BTreeMap<String, StateRecord>> {
        let mut out = BTreeMap::new();
        for (name, record) in self.record_entries()? {
            if name != format!("{}.json", record.address) {
                return Err(PlanError::new(format!(
                    ".spec/state/{name}: holds the record of {}",
                    record.address
                )));
            }
            out.insert(record.address.clone(), record);
        }
        Ok(out)
    }

    /// Every record with its file name, in file name order, without checking that the
    /// name matches the address.
    pub fn record_entries(&self) -> PResult<Vec<(String, StateRecord)>> {
        let mut out = Vec::new();
        for (rel, path) in self.record_files()? {
            let text = fs::read_to_string(&path).map_err(|e| io(&path, e))?;
            let record: StateRecord = serde_json::from_str(&text)
                .map_err(|e| PlanError::new(format!("{}: {e}", path.display())))?;
            let name = rel.trim_start_matches(".spec/state/").to_string();
            out.push((name, record));
        }
        Ok(out)
    }

    /// Record files as (path relative to the project, absolute path), sorted.
    fn record_files(&self) -> PResult<Vec<(String, PathBuf)>> {
        let dir = self.state_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
            let path = entry.map_err(|e| io(&dir, e))?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                files.push((format!(".spec/state/{name}"), path));
            }
        }
        files.sort();
        Ok(files)
    }

    /// The digest of the whole state: every record file's path and exact digest.
    pub fn state_digest(&self) -> PResult<String> {
        let mut map = serde_json::Map::new();
        for (rel, path) in self.record_files()? {
            let bytes = fs::read(&path).map_err(|e| io(&path, e))?;
            map.insert(rel, Value::String(digest_bytes(&bytes)));
        }
        Ok(digest(&Value::Object(map))?)
    }

    pub fn remove_record(&self, address: &str) -> PResult<()> {
        let path = self.state_dir().join(format!("{address}.json"));
        match fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io(&path, e)),
            _ => Ok(()),
        }
    }

    pub fn write_record(&self, record: &StateRecord) -> PResult<()> {
        let dir = self.state_dir();
        fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
        let path = dir.join(format!("{}.json", record.address));
        let value = serde_json::to_value(record).expect("serialisable");
        let text = serde_json::to_string_pretty(&value).expect("serialisable") + "\n";
        fs::write(&path, text).map_err(|e| io(&path, e))
    }

    /// The canonical document whose digest is `hash`, if the store has it.
    pub fn applied(&self, hash: &str) -> PResult<Option<Value>> {
        let Some(hex) = hash.strip_prefix("sha256:") else {
            return Ok(None);
        };
        let path = self.applied_dir().join(format!("{hex}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|e| io(&path, e))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| PlanError::new(format!("{}: {e}", path.display())))?;
        if digest(&value)? != hash {
            return Err(PlanError::new(format!(
                "{}: content does not match its name",
                path.display()
            )));
        }
        Ok(Some(value))
    }

    /// Stores a canonical document under its digest and returns the digest.
    pub fn put_applied(&self, document: &Value) -> PResult<String> {
        let hash = digest(document)?;
        let dir = self.applied_dir();
        fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
        let path = dir.join(format!("{}.json", &hash["sha256:".len()..]));
        if !path.exists() {
            fs::write(&path, canonical(document)? + "\n").map_err(|e| io(&path, e))?;
        }
        Ok(hash)
    }
}

/// The digest of a file's exact bytes, or `None` if it does not exist.
pub fn file_digest(project_root: &Path, path: &str) -> PResult<Option<String>> {
    let full = project_root.join(path);
    match fs::read(&full) {
        Ok(bytes) => Ok(Some(digest_bytes(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io(&full, e)),
    }
}

/// The graph's declarations by address.
pub fn declarations(graph: &Value) -> BTreeMap<&str, &Value> {
    graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["address"].as_str()?, d)))
        .collect()
}

/// A declaration's consumed entries: one per relation, target and projection, for
/// targets that are declarations of the graph.
pub fn consumed_for(decl: &Value, docs: &GraphDocuments) -> PResult<Vec<Consumed>> {
    let mut out = BTreeSet::new();
    for edge in decl["edges"].as_array().into_iter().flatten() {
        let (Some(relation), Some(target)) = (edge["relation"].as_str(), edge["target"].as_str())
        else {
            continue;
        };
        let Some(target_docs) = docs.declarations.get(target) else {
            continue;
        };
        let mut projections: Vec<(&str, &Value)> = Vec::new();
        if ORDERING.contains(&relation) || relation == "collects" {
            projections.push(("public", &target_docs.public));
        }
        if OBSERVING.contains(&relation) {
            projections.push(("behaviour", &target_docs.behaviour));
            projections.push(("context", &target_docs.context));
        }
        for (projection, document) in projections {
            out.insert(Consumed {
                relation: relation.into(),
                target: target.into(),
                projection: projection.into(),
                hash: digest(document)?,
            });
        }
    }
    Ok(out.into_iter().collect())
}

/// A declaration's requirement members (`expect.<key>`, `errors.<key>`) and their
/// contract hashes.
pub fn member_hashes(decl: &Value) -> PResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for collection in ["expect", "errors"] {
        for (key, value) in decl["members"][collection]
            .as_object()
            .into_iter()
            .flatten()
        {
            out.insert(format!("{collection}.{key}"), digest(value)?);
        }
    }
    Ok(out)
}

/// Every declaration address a document refers to with a hash (resolved type references).
fn hashed_refs(value: &Value, acc: &mut BTreeSet<String>) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| hashed_refs(v, acc)),
        Value::Object(map) => {
            if let (Some(Value::String(name)), Some(Value::String(_))) =
                (map.get("name"), map.get("hash"))
            {
                acc.insert(name.clone());
            }
            if let Some(Value::String(component)) = map.get("component") {
                acc.insert(format!("component:{component}"));
            }
            map.values().for_each(|v| hashed_refs(v, acc));
        }
        _ => {}
    }
}

/// What a publication writes besides what the graph and the files determine.
#[derive(Default)]
pub struct Publish<'a> {
    /// Only these declarations; all managed declarations when `None`.
    pub only: Option<&'a BTreeSet<String>>,
    /// Created outputs per address, as they stand after the transaction.
    pub created: BTreeMap<String, BTreeMap<String, Output>>,
    /// Evidence records per address.
    pub evidence: BTreeMap<String, Vec<Value>>,
    /// Source text per address, as applied.
    pub sources: BTreeMap<String, String>,
    pub provenance: Option<Value>,
}

/// Records every managed declaration of `graph` as accepted, with the files on disk as
/// its baseline bytes (a slot with no file is left without an output), and stores every
/// projection the records name. Returns the records written.
pub fn record_baseline(
    store: &Store,
    project_root: &Path,
    graph: &Value,
    docs: &GraphDocuments,
    materialisations: &BTreeMap<String, Materialisation>,
    provenance: Option<Value>,
) -> PResult<Vec<StateRecord>> {
    let publish = Publish {
        provenance,
        ..Publish::default()
    };
    publish_records(store, project_root, graph, docs, materialisations, &publish)
}

/// Writes the records of the published declarations (see [`Publish`]) and the applied
/// store entries they name.
pub fn publish_records(
    store: &Store,
    project_root: &Path,
    graph: &Value,
    docs: &GraphDocuments,
    materialisations: &BTreeMap<String, Materialisation>,
    publish: &Publish,
) -> PResult<Vec<StateRecord>> {
    let decls = declarations(graph);
    let mut written = Vec::new();
    let mut to_store: BTreeSet<String> = BTreeSet::new();
    for (address, m) in materialisations {
        if publish.only.is_some_and(|only| !only.contains(address)) {
            continue;
        }
        let decl = decls[address.as_str()];
        let d = &docs.declarations[address];
        let hashes = d.hashes()?;
        let mut outputs = BTreeMap::new();
        for (slot, path) in &m.slots {
            if let Some(bytes) = file_digest(project_root, path)? {
                outputs.insert(
                    slot.clone(),
                    Output {
                        origin: "slot".into(),
                        path: path.clone(),
                        bytes,
                        root: None,
                        relative_path: None,
                    },
                );
            }
        }
        for (key, out) in publish.created.get(address).into_iter().flatten() {
            if let Some(bytes) = file_digest(project_root, &out.path)? {
                let mut out = out.clone();
                out.bytes = bytes;
                outputs.insert(key.clone(), out);
            }
        }
        let entry = store.put_applied(decl)?;
        let source = match publish.sources.get(address) {
            Some(text) => Some(store.put_applied(&json!({ "source": text }))?),
            None => None,
        };
        let mut context_parts = BTreeMap::new();
        for (source, document) in &d.context_parts {
            context_parts.insert(source.clone(), store.put_applied(document)?);
        }
        for document in [&d.public, &d.behaviour, &d.body, &d.context] {
            store.put_applied(document)?;
            hashed_refs(document, &mut to_store);
        }
        let consumed = consumed_for(decl, docs)?;
        for c in &consumed {
            to_store.insert(format!("{}#{}", c.target, c.projection));
        }
        let record = StateRecord {
            address: address.clone(),
            kind: decl["kind"].as_str().unwrap_or_default().to_string(),
            entry: Some(entry),
            source,
            hashes: Hashes {
                public: hashes.public,
                behaviour: hashes.behaviour,
                body: hashes.body,
                context: hashes.context,
            },
            context_parts,
            consumed,
            members: member_hashes(decl)?,
            outputs,
            evidence: publish.evidence.get(address).cloned().unwrap_or_default(),
            provenance: publish.provenance.clone(),
        };
        store.write_record(&record)?;
        written.push(record);
    }
    // The closure: consumed targets' projections, and everything referenced by hash.
    let mut done = BTreeSet::new();
    while let Some(item) = to_store.pop_first() {
        if !done.insert(item.clone()) {
            continue;
        }
        if let Some(component) = item.strip_prefix("component:") {
            if let Some(doc) = docs.components.get(component) {
                store.put_applied(doc)?;
                hashed_refs(doc, &mut to_store);
            }
            continue;
        }
        let (address, projection) = item.split_once('#').unwrap_or((item.as_str(), "public"));
        let Some(d) = docs.declarations.get(address) else {
            continue;
        };
        let document = match projection {
            "behaviour" => &d.behaviour,
            "context" => &d.context,
            _ => &d.public,
        };
        store.put_applied(document)?;
        let mut refs = BTreeSet::new();
        hashed_refs(document, &mut refs);
        to_store.extend(refs.into_iter().filter(|r| !done.contains(r)));
        if projection == "context" {
            for (_, part) in &d.context_parts {
                store.put_applied(part)?;
            }
        }
    }
    Ok(written)
}

/// The digest standing for the applied graph: every record's address and hashes.
pub fn applied_graph_digest(records: &BTreeMap<String, StateRecord>) -> PResult<String> {
    let map: serde_json::Map<String, Value> = records
        .iter()
        .map(|(a, r)| (a.clone(), json!(r.hashes)))
        .collect();
    Ok(digest(&Value::Object(map))?)
}
