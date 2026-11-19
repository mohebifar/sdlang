#!/usr/bin/env python3
"""Reference canonicaliser for spec graphs (schemas/graph.schema.json).

Computes the four declaration projections and their hashes exactly as SPEC.md
"Projections and hashes" describes, for the kinds used by the worked example. A real
implementation reads interface fields from kind definitions; this reference keeps them
in KIND_INTERFACE.

Also derives a plan's change set from two graphs (diff_graphs), which the checker
compares with examples/plan.json.
"""
import hashlib
import json

# Fields that form the public projection, per kind. Everything else that is not
# summary, behaviour or guidance is body.
KIND_INTERFACE = {
    "core:project": [],
    "core:stack": ["language", "runtime", "framework", "package_manager"],
    "core:use": ["package", "capability", "options"],
    "core:rule": ["applies_to"],
    "core:entity": [],
    "core:action": ["input", "output"],
    "npm:@intentlang/nextjs::page": ["route"],
}
BEHAVIOUR_FIELDS = {"prompt"}
GUIDANCE_FIELDS = {"hint", "skills"}
SET_FIELDS = {"uses", "calls", "reads", "writes", "requires", "skills"}
CONSUMING = {"calls", "reads", "writes", "uses", "types", "requires"}
OBSERVING = {"calls", "reads", "writes"}


def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def sha(obj):
    return "sha256:" + hashlib.sha256(canonical(obj).encode()).hexdigest()


def _sorted_set(value):
    if isinstance(value, list):
        return sorted(value, key=canonical)
    return value


def _resolve_types(value, public_hashes):
    """Replace named type references to graph declarations by address + public hash."""
    if isinstance(value, list):
        return [_resolve_types(v, public_hashes) for v in value]
    if isinstance(value, dict):
        if value.get("$type") == "named" and value.get("name") in public_hashes:
            return {"$type": "named", "name": value["name"], "hash": public_hashes[value["name"]]}
        return {k: _resolve_types(v, public_hashes) for k, v in value.items()}
    return value


def _named_refs(value, acc):
    if isinstance(value, list):
        for v in value:
            _named_refs(v, acc)
    elif isinstance(value, dict):
        if value.get("$type") == "named":
            acc.add(value.get("name"))
        for v in value.values():
            _named_refs(v, acc)
    return acc


def _raw_public(decl):
    kind = decl["kind"]
    fields = decl["fields"]
    iface = {f: _sorted_set(fields[f]) if f in SET_FIELDS else fields[f]
             for f in KIND_INTERFACE.get(kind, []) if f in fields}
    members = decl.get("members", {})
    pub_members = {}
    if kind == "core:action" and "errors" in members:
        pub_members["errors"] = {k: v.get("code", k) for k, v in members["errors"].items()}
    if kind == "core:entity" and "fields" in members:
        pub_members["fields"] = members["fields"]
    annotations = sorted((a for a in decl.get("annotations", [])), key=canonical)
    return {"kind": kind, "summary": fields.get("summary"), "interface": iface,
            "members": pub_members, "annotations": annotations}


def public_hashes(graph):
    """Public hashes for every declaration; mutually recursive type references are
    hashed together as one strongly connected component (Tarjan)."""
    decls = {d["address"]: d for d in graph["declarations"]}
    raw = {a: _raw_public(d) for a, d in decls.items()}
    deps = {a: sorted(x for x in _named_refs(raw[a], set()) if x in decls) for a in decls}
    index, low, stack, on, sccs, counter = {}, {}, [], set(), [], [0]

    def strong(v):
        index[v] = low[v] = counter[0]; counter[0] += 1
        stack.append(v); on.add(v)
        for w in deps[v]:
            if w not in index:
                strong(w); low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop(); on.discard(w); comp.append(w)
                if w == v:
                    break
            sccs.append(sorted(comp))

    for a in sorted(decls):
        if a not in index:
            strong(a)
    hashes = {}
    for comp in sccs:  # Tarjan emits components in reverse topological order
        if len(comp) == 1 and comp[0] not in deps[comp[0]]:
            a = comp[0]
            hashes[a] = sha(_resolve_types(raw[a], hashes))
        else:
            members = {a: _resolve_types(raw[a], {k: v for k, v in hashes.items() if k not in comp})
                       for a in comp}
            comp_hash = sha(members)
            for a in comp:
                hashes[a] = sha({"component": comp_hash, "address": a})
    return hashes


def projections(graph):
    """Return {address: {public, behaviour, body, guidance}} hashes for a graph."""
    decls = {d["address"]: d for d in graph["declarations"]}
    modules = {m["path"]: m for m in graph["modules"]}
    pub = public_hashes(graph)
    project = next((d for d in decls.values() if d["kind"] == "core:project"), None)
    out = {}
    for a, d in decls.items():
        fields, members = d["fields"], d.get("members", {})
        inherited = []
        if project is not None and a != project["address"] and "prompt" in project["fields"]:
            inherited.append({"source": "project", "hash": sha(project["fields"]["prompt"])})
        header = modules.get(d["module"], {}).get("header", {})
        if "prompt" in header:
            inherited.append({"source": "module:" + d["module"], "hash": sha(header["prompt"])})
        for r in d.get("applies", []):
            rule = decls[r]
            inherited.append({"source": "rule:" + r,
                              "hash": sha({"prompt": rule["fields"].get("prompt"),
                                           "expect": {k: v.get("text") for k, v in
                                                      rule.get("members", {}).get("expect", {}).items()}})})
        behaviour = {
            "prompt": fields.get("prompt"),
            "expect": {k: v.get("text") for k, v in members.get("expect", {}).items()},
            "errors": {k: v.get("summary") for k, v in members.get("errors", {}).items()},
            "inherited": inherited,
        }
        iface = set(KIND_INTERFACE.get(d["kind"], []))
        body = {
            "fields": {f: (_sorted_set(v) if f in SET_FIELDS else v) for f, v in fields.items()
                       if f not in iface and f not in BEHAVIOUR_FIELDS and f not in GUIDANCE_FIELDS
                       and f != "summary"},
            "policies": {k: v.get("verify") for k, v in members.get("expect", {}).items() if "verify" in v},
            "lifecycle": d["lifecycle"],
        }
        guidance = {f: (_sorted_set(fields[f]) if f in SET_FIELDS else fields[f])
                    for f in GUIDANCE_FIELDS if f in fields}
        out[a] = {"public": pub[a], "behaviour": sha(behaviour), "body": sha(body), "guidance": sha(guidance)}
    return out


def diff_graphs(before, after):
    """Derive the plan's change set: creates, updates (own projection changed or an
    aggregate owner whose collected members changed), deletes, and dependents whose
    consumed or observed projection changed. Guidance-only changes create nothing."""
    hb, ha = projections(before), projections(after)
    dafter = {d["address"]: d for d in after["declarations"]}
    managed = {a for a, d in dafter.items() if d["sort"] == "managed"}
    creates = sorted(a for a in ha if a not in hb and a in managed)
    deletes = sorted(a for a in hb if a not in ha)
    own = {a for a in ha if a in hb and any(ha[a][p] != hb[a][p] for p in ("public", "behaviour", "body"))}
    public_changed = {a for a in ha if a in hb and ha[a]["public"] != hb[a]["public"]}
    behaviour_changed = {a for a in ha if a in hb and ha[a]["behaviour"] != hb[a]["behaviour"]}
    aggregate = set()
    consumers = {}
    for a, d in dafter.items():
        for e in d["edges"]:
            t = e["target"]
            if e["relation"] == "collects" and (t in public_changed or t in creates):
                aggregate.add(a)
            if e["relation"] in CONSUMING and t in public_changed:
                consumers.setdefault(a, set()).add(("consumed", t))
            if e["relation"] in OBSERVING and t in behaviour_changed:
                consumers.setdefault(a, set()).add(("observed", t))
    updates = sorted((own | aggregate) & managed)
    obligations = {a: sorted(v) for a, v in consumers.items()
                   if a in managed and a not in updates and a not in creates}
    return {"create": creates, "update": updates, "delete": deletes, "obligations": obligations}
