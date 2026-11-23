#!/usr/bin/env python3
"""Reference canonicaliser for spec graphs (schemas/graph.schema.json).

Computes the four declaration projections and their hashes exactly as docs/spec.md
"Projections and hashes" describes. Which fields are interface comes from the graph's
`kinds` table, which the frontend fills from kind definitions.

Also derives a plan's change set from two graphs (diff_graphs), which the checker
compares with examples/plan.json.
"""
import hashlib
import json
import math
import unicodedata

BEHAVIOUR_FIELDS = {"prompt"}
# Core declaration fields: never interface, even for a kind whose interface is "all".
CORE_FIELDS = {"summary", "prompt", "hint", "uses", "skills", "lifecycle"}
GUIDANCE_FIELDS = {"hint", "skills"}
SET_FIELDS = {"uses", "calls", "reads", "writes", "requires", "skills"}
CONSUMING = {"calls", "reads", "writes", "uses", "types", "requires"}
OBSERVING = {"calls", "reads", "writes"}


def _js_number(x):
    """A non-integral finite float as JavaScript's Number.prototype.toString writes it:
    shortest round-trip digits, plain decimal unless the exponent is below -6 or above 20."""
    from decimal import Decimal
    sign = "-" if x < 0 else ""
    t = Decimal(repr(abs(x))).as_tuple()
    digits, exp = "".join(map(str, t.digits)), t.exponent
    while len(digits) > 1 and digits.endswith("0"):
        digits, exp = digits[:-1], exp + 1
    digits = digits.lstrip("0") or "0"
    k, n = len(digits), len(digits) + exp
    if k <= n <= 21:
        out = digits + "0" * (n - k)
    elif 0 < n <= 21:
        out = digits[:n] + "." + digits[n:]
    elif -6 < n <= 0:
        out = "0." + "0" * (-n) + digits
    else:
        e = n - 1
        out = digits[0] + ("." + digits[1:] if k > 1 else "") + "e" + ("+" if e > 0 else "-") + str(abs(e))
    return sign + out


def _encode(obj, out):
    if isinstance(obj, str):
        out.append(json.dumps(unicodedata.normalize("NFC", obj), ensure_ascii=False))
    elif obj is None or isinstance(obj, bool):
        out.append(json.dumps(obj))
    elif isinstance(obj, int):
        out.append(str(obj))
    elif isinstance(obj, float):
        if not math.isfinite(obj):
            raise ValueError("non-finite number in canonical JSON")
        out.append(str(int(obj)) if obj == int(obj) else _js_number(obj))
    elif isinstance(obj, list):
        out.append("[")
        for i, v in enumerate(obj):
            if i:
                out.append(",")
            _encode(v, out)
        out.append("]")
    elif isinstance(obj, dict):
        items = {}
        for k, v in obj.items():
            nk = unicodedata.normalize("NFC", k)
            if nk in items:
                raise ValueError(f"duplicate key after NFC normalisation: {nk!r}")
            items[nk] = v
        out.append("{")
        for i, k in enumerate(sorted(items)):  # str order is Unicode code point order
            if i:
                out.append(",")
            out.append(json.dumps(k, ensure_ascii=False))
            out.append(":")
            _encode(items[k], out)
        out.append("}")
    else:
        raise TypeError(type(obj))


def canonical(obj):
    """Canonical JSON (docs/spec.md, Projections and hashes): NFC strings and keys, keys sorted
    by code point, no insignificant whitespace, UTF-8; integers and integral floats in
    plain decimal; other numbers as JavaScript writes them; non-finite numbers rejected."""
    out = []
    _encode(obj, out)
    return "".join(out)


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


def interface_fields(graph, decl):
    """The fields of a declaration that form its public projection, from the graph's
    kinds table; everything else that is not summary, behaviour or guidance is body."""
    kind = decl["kind"]
    if kind not in graph.get("kinds", {}):
        raise ValueError(f"{decl['address']}: kind {kind} is missing from the graph's kinds")
    listed = graph["kinds"][kind]["interface"]
    if listed == "all":
        return [f for f in decl["fields"] if f not in CORE_FIELDS]
    return listed


def _raw_public(graph, decl):
    kind = decl["kind"]
    fields = decl["fields"]
    iface = {f: _sorted_set(fields[f]) if f in SET_FIELDS else fields[f]
             for f in interface_fields(graph, decl) if f in fields}
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
    raw = {a: _raw_public(graph, d) for a, d in decls.items()}
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
        parts = d["module"].split(".") if d["module"] else []
        for i in range(1, len(parts) + 1):  # ancestor modules, outermost first
            path = ".".join(parts[:i])
            header = modules.get(path, {}).get("header", {})
            if "prompt" in header:
                inherited.append({"source": "module:" + path, "hash": sha(header["prompt"])})
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
        }
        context = {"normative": inherited}
        iface = set(interface_fields(graph, d))
        body = {
            "fields": {f: (_sorted_set(v) if f in SET_FIELDS else v) for f, v in fields.items()
                       if f not in iface and f not in BEHAVIOUR_FIELDS and f not in GUIDANCE_FIELDS
                       and f != "summary"},
            "policies": {k: v.get("verify") for k, v in members.get("expect", {}).items() if "verify" in v},
            "lifecycle": d["lifecycle"],
        }
        guidance = {f: (_sorted_set(fields[f]) if f in SET_FIELDS else fields[f])
                    for f in GUIDANCE_FIELDS if f in fields}
        out[a] = {"public": pub[a], "behaviour": sha(behaviour), "body": sha(body),
                  "guidance": sha(guidance), "context": sha(context)}
    return out


def diff_graphs(before, after):
    """Derive the plan's change set, following docs/spec.md's precedence: an authored change
    to a declaration's own public, behaviour or body projection is an update; so is a
    change to an aggregate owner's collected members. A change to inherited normative
    context only is a review (plus verify) obligation, never an update. Dependents whose
    consumed projection changed get an obligation (compatibility decides verify or
    update; not computed here), and observers of a target whose behaviour or normative
    context changed get verify and review. Guidance-only changes create nothing."""
    hb, ha = projections(before), projections(after)
    dafter = {d["address"]: d for d in after["declarations"]}
    managed = {a for a, d in dafter.items() if d["sort"] == "managed"}
    creates = sorted(a for a in ha if a not in hb and a in managed)
    deletes = sorted(a for a in hb if a not in ha)
    own = {a for a in ha if a in hb and any(ha[a][p] != hb[a][p] for p in ("public", "behaviour", "body"))}
    public_changed = {a for a in ha if a in hb and ha[a]["public"] != hb[a]["public"]}
    behaviour_changed = {a for a in ha if a in hb and (ha[a]["behaviour"] != hb[a]["behaviour"]
                                                       or ha[a]["context"] != hb[a]["context"])}
    context_changed = {a for a in ha if a in hb and ha[a]["context"] != hb[a]["context"]}
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
    for a in context_changed:
        consumers.setdefault(a, set()).add(("context", a))
    updates = sorted((own | aggregate) & managed)
    obligations = {a: sorted(v) for a, v in consumers.items()
                   if a in managed and a not in updates and a not in creates}
    return {"create": creates, "update": updates, "delete": deletes, "obligations": obligations}
