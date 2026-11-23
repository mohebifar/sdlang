# Test check: every "[case:<tag>]" in the test output is a case; it passes unless its
# line says FAILS.
import hashlib, json, os, re, sys

inp = json.load(sys.stdin)
out = inp["outputs"][0]
path = os.path.join(inp["workdir"], out["path"])
raw = open(path, "rb").read() if os.path.exists(path) else b""
oracle = "sha256:" + hashlib.sha256(raw).hexdigest()
cases = {}
for line in raw.decode().splitlines():
    for tag in re.findall(r"\[case:([a-z_][a-z0-9_]*)\]", line):
        cases[tag] = "failed" if "FAILS" in line else "passed"
result = [{"case": c, "oracle": oracle, "instances": [{"key": "default", "attempts": [s]}]}
          for c, s in sorted(cases.items())]
pairs = sorted([c, "default"] for c in cases)
inventory = "sha256:" + hashlib.sha256(json.dumps(pairs, separators=(",", ":")).encode()).hexdigest()
print(json.dumps({
    "protocol": "sdlang.check/1", "check": inp["check"], "snapshot": inp["snapshot"],
    "output": out["member"], "selection": "complete", "cases": result,
    "untagged": {"passed": 0, "failed": 0}, "inventory": inventory,
    "inputs": inp["inputs"], "profile": inp["profile"],
}))
