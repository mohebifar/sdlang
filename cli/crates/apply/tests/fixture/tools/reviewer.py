# A scripted reviewer: REVIEW_VERDICT (default conforms) for every subject; coverage sufficient.
import json, os

order = json.load(open(os.environ["SPEC_REVIEW_ORDER"]))
# REVIEW_BAD=always answers with requirement names instead of subject ids, like the first
# real reviewer did; REVIEW_BAD=once does so only on its first review (REVIEW_MARK records it).
bad = os.environ.get("REVIEW_BAD", "")
if bad == "once":
    mark = os.environ["REVIEW_MARK"]
    bad = "" if os.path.exists(mark) else "always"
    open(mark, "a").close()
# REVIEW_ESCALATE=<address> finds that address nonconforming in its first review only
# (REVIEW_MARK records it), so the obligation escalates and the next review conforms.
escalate = os.environ.get("REVIEW_ESCALATE", "")
first = False
if escalate and any(s["subject"]["target"] == escalate for s in order["subjects"]):
    mark = os.environ["REVIEW_MARK"]
    first = not os.path.exists(mark)
    open(mark, "a").close()
# REVIEW_INSUFFICIENT=<address> finds that address's tests insufficient in its first review
# only (REVIEW_MARK records it), so the agent is asked again with the reason.
insufficient = os.environ.get("REVIEW_INSUFFICIENT", "")
gap = False
if insufficient and any(s["subject"]["target"] == insufficient for s in order["subjects"]):
    mark = os.environ["REVIEW_MARK"]
    gap = not os.path.exists(mark)
    open(mark, "a").close()
subjects = []
for s in order["subjects"]:
    verdict = os.environ.get("REVIEW_VERDICT", "conforms")
    if first and s["subject"]["target"] == escalate:
        verdict = "does_not_conform"
    v = {"id": s["subject"]["requirement"] if bad == "always" else s["id"], "conformance": {"verdict": verdict, "reason": "scripted"}}
    if "coverage" in s["assess"]:
        short = gap and s["subject"]["target"] == insufficient
        v["coverage"] = [{"candidate": c["id"], "verdict": "insufficient" if short else "sufficient",
                          "reason": "the test never checks the greeting text" if short else "scripted"}
                         for c in s.get("coverage_candidates", [])]
    subjects.append(v)
json.dump({"schema": "sdlang.review-verdict/1", "dispatch": order["dispatch"],
           "order_hash": order["binding"]["order_hash"], "subjects": subjects},
          open(os.environ["SPEC_VERDICT"], "w"))
