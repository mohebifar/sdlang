"""A deterministic reviewer for checking the language arm's runner: a subject conforms
exactly when every code path it names has the reference's bytes (ORACLE_REF); coverage is
always sufficient. Operator tool; it reads the reference."""
import json
import os

ref = os.environ["ORACLE_REF"]
order = json.load(open(os.environ["SPEC_REVIEW_ORDER"]))


def same(rel):
    a, b = rel, os.path.join(ref, rel)
    if os.path.exists(a) != os.path.exists(b):
        return False
    return not os.path.exists(a) or open(a, "rb").read() == open(b, "rb").read()


subjects = []
for s in order["subjects"]:
    differs = [p for p in s.get("code", []) if not same(p)]
    v = {"id": s["id"], "conformance": {
        "verdict": "does_not_conform" if differs else "conforms",
        "reason": ("differs from the reference: " + ", ".join(differs)) if differs else "matches the reference"}}
    if "coverage" in s["assess"]:
        v["coverage"] = [{"candidate": c["id"], "verdict": "sufficient", "reason": "oracle"}
                         for c in s.get("coverage_candidates", [])]
    subjects.append(v)
json.dump({"schema": "sdlang.review-verdict/1", "dispatch": order["dispatch"],
           "order_hash": order["binding"]["order_hash"], "subjects": subjects},
          open(os.environ["SPEC_VERDICT"], "w"))
