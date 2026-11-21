# A scripted reviewer: REVIEW_VERDICT (default conforms) for every subject; coverage sufficient.
import json, os

order = json.load(open(os.environ["SPEC_REVIEW_ORDER"]))
subjects = []
for s in order["subjects"]:
    v = {"id": s["id"], "conformance": {"verdict": os.environ.get("REVIEW_VERDICT", "conforms"), "reason": "scripted"}}
    if "coverage" in s["assess"]:
        v["coverage"] = [{"candidate": c["id"], "verdict": "sufficient", "reason": "scripted"}
                         for c in s.get("coverage_candidates", [])]
    subjects.append(v)
json.dump({"schema": "intlang.review-verdict/1", "dispatch": order["dispatch"],
           "order_hash": order["binding"]["order_hash"], "subjects": subjects},
          open(os.environ["SPEC_VERDICT"], "w"))
