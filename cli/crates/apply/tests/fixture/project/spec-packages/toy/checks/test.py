# Test check: the test output is JSON {"cases": {case: "passed" | "failed"}}.
import hashlib, json, os, sys

inp = json.load(sys.stdin)
out = inp["outputs"][0]
path = os.path.join(inp["workdir"], out["path"])
raw = open(path, "rb").read() if os.path.exists(path) else b'{"cases": {}}'
data = json.loads(raw)
# A FAIL file in the package fails every case, so a rerun can differ from the record.
if os.path.exists(os.path.join(inp["package_root"], "FAIL")):
    data["cases"] = {c: "failed" for c in data["cases"]}
oracle = "sha256:" + hashlib.sha256(raw).hexdigest()
cases = [{"case": c, "oracle": oracle, "instances": [{"key": "default", "attempts": [s]}]}
         for c, s in sorted(data["cases"].items())]
pairs = sorted([c["case"], "default"] for c in cases)
inventory = "sha256:" + hashlib.sha256(json.dumps(pairs, separators=(",", ":")).encode()).hexdigest()
print(json.dumps({
    "protocol": "intlang.check/1", "check": inp["check"], "snapshot": inp["snapshot"],
    "output": out["member"], "selection": "complete", "cases": cases,
    "untagged": {"passed": 0, "failed": 0}, "inventory": inventory,
    "inputs": inp["inputs"], "profile": inp["profile"],
}))
