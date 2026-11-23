# Generator: writes every slot in its envelope through the staging area.
import hashlib, json, os, sys

inp = json.load(sys.stdin)
files = []
for w in inp["envelope"]["write"]:
    rel = w["path"]
    body = ("generated " + w["output"] + "\n").encode()
    dest = os.path.join(inp["staging"], rel)
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    open(dest, "wb").write(body)
    files.append({"op": "create" if w["expected"] == "absent" else "update", "target": w["output"],
                  "relative_path": rel, "digest": "sha256:" + hashlib.sha256(body).hexdigest()})
print(json.dumps({"protocol": "sdlang.generator/1", "dispatch": inp["dispatch"],
                  "order_hash": inp["binding"]["order_hash"], "status": "written", "files": files}))
