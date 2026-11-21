"""A deterministic coding agent for checking the language arm's runner: it makes each
subject's outputs equal to the reference implementation at ORACLE_REF, staying inside the
order's envelope, registers created outputs and binds tests by their case tags.
Operator tool; it reads the reference, which no trial agent may see."""
import json
import os
import re
import shutil

SLOTS = {"page", "tests", "server", "main", "manifest", "lockfile", "tsconfig", "next_config"}
ref = os.environ["ORACLE_REF"]
order = json.load(open(os.environ["SPEC_WORK_ORDER"]))


def ident(stem):
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", stem)
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")


def put(rel):
    src = os.path.join(ref, rel)
    if os.path.exists(src):
        os.makedirs(os.path.dirname(rel) or ".", exist_ok=True)
        shutil.copyfile(src, rel)
        return True
    if os.path.exists(rel):
        os.remove(rel)
    return False


outputs, tests = [], []
for s in order["subjects"]:
    addr = s["address"]
    env = s["envelope"]
    roots = {c["root"]: c["path"] for c in env["create_under"]}
    known = set()
    for w in env["write"]:
        key = w["output"].split("#outputs.")[-1]
        path = w["path"]
        known.add(path)
        if put(path) or key in SLOTS:
            continue
        # A created output the reference keeps elsewhere: relocate it by its file name.
        name = os.path.basename(path)
        for root, rpath in roots.items():
            if os.path.exists(os.path.join(ref, rpath, name)):
                put(rpath + name)
                known.add(rpath + name)
                outputs.append({"op": "relocate", "owner": addr, "key": key, "root": root, "relative_path": name})
                break
        else:
            outputs.append({"op": "remove", "owner": addr, "key": key})
    for root, rpath in roots.items():
        base = os.path.join(ref, rpath)
        for dirpath, _, files in os.walk(base):
            for f in sorted(files):
                rel = os.path.relpath(os.path.join(dirpath, f), ref)
                if rel in known:
                    continue
                put(rel)
                outputs.append({"op": "create", "owner": addr, "key": ident(f.split(".")[0]),
                                "root": root, "relative_path": os.path.relpath(rel, rpath)})
    test_paths = [(w["output"], w["path"]) for w in env["write"] if w["output"].endswith("#outputs.tests")]
    for r in s["requirements"]:
        key = r["subject"]["requirement"].split(".")[-1]
        bound = [{"output": o, "case": key} for o, p in test_paths
                 if os.path.exists(p) and f"[case:{key}]" in open(p).read()]
        if bound:
            tests.append({"subject": r["subject"], "tests": bound})

result = {"schema": "intlang.result/1", "dispatch": order["dispatch"],
          "order_hash": order["binding"]["order_hash"], "status": "submitted",
          "summary": "Made the subjects' outputs equal to the reference.", "outputs": outputs, "tests": tests}
os.makedirs(os.path.dirname(os.environ["SPEC_RESULT"]), exist_ok=True)
json.dump(result, open(os.environ["SPEC_RESULT"], "w"))
