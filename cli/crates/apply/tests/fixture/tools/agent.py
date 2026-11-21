# A scripted coding agent. AGENT_MODE: good | escape (writes rogue.txt for AGENT_TARGET) |
# fail_first (first attempt writes failing tests). AGENT_LOG records each dispatch.
import json, os

order = json.load(open(os.environ["SPEC_WORK_ORDER"]))
mode = os.environ.get("AGENT_MODE", "good")
target = os.environ.get("AGENT_TARGET", "")
tests = []
for s in order["subjects"]:
    addr = s["address"]
    keys = [r["subject"]["requirement"].split(".")[-1] for r in s["requirements"]]
    for w in s["envelope"]["write"]:
        path = w["path"]
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        if w["output"].endswith("#outputs.tests"):
            failing = mode == "fail_first" and order["attempt"]["number"] == 1
            json.dump({"cases": {k: "failed" if failing else "passed" for k in keys}}, open(path, "w"))
        else:
            open(path, "w").write("note " + addr + "\n")
    for r, k in zip(s["requirements"], keys):
        tests.append({"subject": r["subject"], "tests": [{"output": addr + "#outputs.tests", "case": k}]})
    if mode == "escape" and addr == target:
        open("rogue.txt", "w").write("not mine\n")
result = {"schema": "intlang.result/1", "dispatch": order["dispatch"],
          "order_hash": order["binding"]["order_hash"], "status": "submitted",
          "summary": "Wrote the note and its test cases.", "outputs": [], "tests": tests}
os.makedirs(os.path.dirname(os.environ["SPEC_RESULT"]), exist_ok=True)
json.dump(result, open(os.environ["SPEC_RESULT"], "w"))
log = os.environ.get("AGENT_LOG")
if log:
    with open(log, "a") as f:
        f.write(json.dumps({"dispatch": order["dispatch"], "attempt": order["attempt"]["number"],
                            "previous": len(order["attempt"]["previous"]),
                            "order": os.environ["SPEC_WORK_ORDER"]}) + "\n")
