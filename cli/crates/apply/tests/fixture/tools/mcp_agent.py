# A scripted agent that uses the MCP server from $SPEC_MCP_CONFIG like an MCP client,
# then does agent.py's work, validates its result through the server and logs every
# answer to $MCP_LOG.
import json, os, subprocess, sys

config = json.load(open(os.environ["SPEC_MCP_CONFIG"]))["mcpServers"]["spec"]
server = subprocess.Popen([config["command"]] + config["args"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
counter = [0]

def rpc(method, params=None, notify=False):
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        counter[0] += 1
        msg["id"] = counter[0]
    server.stdin.write(json.dumps(msg) + "\n")
    server.stdin.flush()
    return None if notify else json.loads(server.stdout.readline())

def tool(name, args):
    r = rpc("tools/call", {"name": name, "arguments": args})["result"]
    return r.get("structuredContent", {"error_text": r["content"][0]["text"]}) if not r["isError"] else {"tool_error": r["content"][0]["text"]}

log = {}
log["initialize"] = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "scripted", "version": "0"}})["result"]
rpc("notifications/initialized", notify=True)
log["tools"] = [t["name"] for t in rpc("tools/list")["result"]["tools"]]
task = tool("get_task", {})
log["get_task"] = task
address = task["subjects"][0]["address"]
log["own_public"] = tool("get_declaration", {"address": address, "projection": "public"})
log["missing"] = tool("get_declaration", {"address": "nowhere.at_all", "projection": "public"})
order = json.load(open(os.environ["SPEC_WORK_ORDER"]))
resources = [m["resource"] for m in order["context"]["manifest"]]
if resources:
    log["read_context"] = tool("read_context", {"resource": resources[0]})

# Do the work.
here = os.path.dirname(os.path.abspath(__file__))
subprocess.run([sys.executable, os.path.join(here, "agent.py")], check=True)
result = json.load(open(os.environ["SPEC_RESULT"]))
open("rogue.txt", "w").write("x\n")
log["validate_with_rogue"] = tool("validate_result", {"draft": result})
os.remove("rogue.txt")
log["validate"] = tool("validate_result", {"draft": result})
log["self_check"] = tool("run_self_check", {"check": "toy.test"})
log["not_a_check"] = tool("run_self_check", {"check": "toy.nothing"})
server.stdin.close()
server.wait()
with open(os.environ["MCP_LOG"], "a") as f:
    f.write(json.dumps(log) + "\n")
