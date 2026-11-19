#!/usr/bin/env python3
"""Generate the protocol examples in examples/ for tx-4 (auth.login update) of the
worked example in SPEC.md. Digests are sha256 of stable labels, so the files are
reproducible; order_hash is computed exactly as the schema describes.

Usage: python3 tools/make_examples.py
"""
import difflib
import hashlib
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import canon  # noqa: E402
from worked_graph import GRAPH_BEFORE, GRAPH_AFTER, plan_fixture  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def d(label):
    return "sha256:" + hashlib.sha256(label.encode()).hexdigest()


def git(label):
    return "git:" + hashlib.sha1(label.encode()).hexdigest()


def res(kind, label):
    return f"ctx:{kind}:{hashlib.sha256(label.encode()).hexdigest()[:12]}"


def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def order_hash(order, field="binding"):
    clone = json.loads(json.dumps(order))
    del clone[field]["order_hash"]
    return canon.sha(clone)


PREVIOUS = '''export nextjs.page login {
  route: "/login"
  summary: "A page where you can sign in"
  uses: [ui.form, ui.input, ui.button]
  calls: [sign_in]
  prompt: """
    Email and password form in the shared auth card.
    On success go to @{dashboard.home}.
  """
  expect {
    redirects: "valid credentials redirect to @{dashboard.home}"
    generic_error: "invalid credentials show one generic error and keep the email filled in"
    busy_button: "the submit button is disabled while the request is in flight"
  }
}
'''

CURRENT = '''export nextjs.page login {
  route: "/login"
  summary: "A page where you can sign in"
  uses: [ui.form, ui.input, ui.button, ui.link]
  calls: [sign_in]
  prompt: """
    Email and password form in the shared auth card.
    On success go to @{dashboard.home}.
    Link to @{forgot_password} under the password field.
  """
  expect {
    redirects: "valid credentials redirect to @{dashboard.home}"
    generic_error: "invalid credentials show one generic error and keep the email filled in"
    busy_button: "the submit button is disabled while the request is in flight"
    forgot_link: "the forgot-password link goes to @{forgot_password}"
  }
}
'''

PROJECT_PROMPT = "Server-render by default. All copy in plain English, sentence case.\n"
AUTH_PROMPT = "All auth screens use one centered card layout.\nNever reveal whether an email address is registered.\n"
DISPATCH = "p-2026-11-18-01/r1/tx-4/1"
TESTS = "auth.login#outputs.tests"


def requirement(key, text, baseline, cases):
    return {
        "subject": {"target": "auth.login", "requirement": f"auth.login#expect.{key}"},
        "text": text,
        "contract_hash": contract(text.replace("@{forgot_password}", "@{auth.forgot_password}")),
        "policy": {"tests": True, "review": True, "checks": [], "approval": False},
        "policy_source": "project",
        "baseline": baseline,
        "tests": [{"output": TESTS, "case": c} for c in cases],
    }


HB = canon.projections(GRAPH_BEFORE)
HA = canon.projections(GRAPH_AFTER)
PROJECT_HASH = canon.sha({"$prose": PROJECT_PROMPT})
AUTH_HASH = canon.sha({"$prose": AUTH_PROMPT})


def contract(text):
    """A requirement's contract hash: the canonical member, as in the graph."""
    return canon.sha({"text": {"$prose": text}})


def work_order():
    diff = "".join(difflib.unified_diff(PREVIOUS.splitlines(True), CURRENT.splitlines(True),
                                        "a/auth.login", "b/auth.login"))
    catalog = [("ui.form", "Form"), ("ui.input", "Input"), ("ui.button", "Button"), ("ui.link", "Link")]
    refs = [
        {"from": "auth.login", "target": "auth.sign_in", "relation": "calls", "projection": "public",
         "viewpoint": "applied", "hash": HB["auth.sign_in"]["public"],
         "text": 'action sign_in {\n  summary: ""\n  input: { email: email, password: string }\n'
                 '  output: authentication.session\n  errors { invalid_credentials, rate_limited }\n}\n'},
        {"from": "auth.login", "target": "auth.sign_in", "relation": "calls", "projection": "behaviour",
         "viewpoint": "applied", "hash": HB["auth.sign_in"]["behaviour"],
         "text": "errors:\n  invalid_credentials: Email or password is wrong\n"
                 "  rate_limited: Too many attempts; try again later\n"},
        {"from": "auth.login", "target": "auth.sign_in", "relation": "calls", "projection": "context",
         "viewpoint": "applied", "hash": HB["auth.sign_in"]["context"],
         "text": "inherited normative context:\n  project: " + PROJECT_PROMPT + "  module auth: " + AUTH_PROMPT},
        {"from": "auth.login", "target": "dashboard.home", "relation": "mentions", "projection": "public",
         "viewpoint": "applied", "hash": HB["dashboard.home"]["public"],
         "text": 'nextjs.page home { route: "/app" summary: "Lists the signed-in user\'s workspaces" }\n'},
        {"from": "auth.login", "target": "auth.forgot_password", "relation": "mentions", "projection": "public",
         "viewpoint": "desired", "hash": HA["auth.forgot_password"]["public"],
         "text": 'nextjs.page forgot_password { route: "/forgot-password" summary: "Request a password reset email" }\n'},
    ]
    for alias, export in catalog:
        refs.append({"from": "auth.login", "target": f"npm:ui-sdk::{export}", "relation": "uses",
                     "projection": "catalog", "viewpoint": "applied", "hash": d(f"catalog:ui-sdk:{export}"),
                     "text": f"{export}: component contract from ui-sdk ({alias})\n"})
    manifest = [
        {"id": "skill:authentication.skills", "class": "skill", "required": False, "included": False,
         "resource": res("skill", "better-auth"), "hash": d("skill:better-auth"), "tokens": 6200},
        {"id": "catalog_docs:ui-sdk", "class": "catalog_docs", "required": False, "included": False,
         "resource": res("docs", "ui-sdk"), "hash": d("docs:ui-sdk"), "tokens": 9400},
        {"id": "code:auth.login#outputs.page", "class": "code", "required": True, "included": False,
         "resource": res("code", "src/app/login/page.tsx"), "hash": d("bytes:page.tsx"), "tokens": 1300},
        {"id": "code:auth.login#outputs.login_form", "class": "code", "required": True, "included": False,
         "resource": res("code", "src/app/login/_parts/LoginForm.tsx"), "hash": d("bytes:LoginForm.tsx"),
         "tokens": 2100},
        {"id": "code:auth.login#outputs.tests", "class": "code", "required": True, "included": False,
         "resource": res("code", "tests/app/login/page.test.ts"), "hash": d("bytes:page.test.ts"), "tokens": 1800},
    ]
    order = {
        "schema": "intentlang.work-order/1",
        "dispatch": DISPATCH,
        "binding": {
            "order_hash": "",
            "plan": "p-2026-11-18-01",
            "plan_revision": 1,
            "plan_hash": d("plan:p-2026-11-18-01:r1"),
            "transaction": "tx-4",
            "base_commit": git("commit:after-tx-3"),
            "input_snapshot": d("snapshot:after-tx-3"),
            "graph_hash": d("graph:77d0"),
            "lock_hash": d("lock:5f3a"),
        },
        "attempt": {"number": 1, "max": 3, "workspace": "new", "previous": []},
        "subjects": [{
            "address": "auth.login",
            "kind": "nextjs.page",
            "change": "update",
            "strategy": "edit",
            "causes": [{"kind": "own_projection", "via": "body,behaviour"}],
            "spec": {
                "rendering": "source/1",
                "current": CURRENT,
                "previous": PREVIOUS,
                "diff": diff,
                "resolution": {
                    "nextjs.page": "npm:@intentlang/nextjs::page",
                    "ui.form": "npm:ui-sdk::Form", "ui.input": "npm:ui-sdk::Input",
                    "ui.button": "npm:ui-sdk::Button", "ui.link": "npm:ui-sdk::Link",
                    "sign_in": "auth.sign_in", "dashboard.home": "dashboard.home",
                    "forgot_password": "auth.forgot_password",
                },
            },
            "envelope": {
                "write": [
                    {"output": "auth.login#outputs.page", "path": "src/app/login/page.tsx",
                     "expected": d("bytes:page.tsx")},
                    {"output": "auth.login#outputs.tests", "path": "tests/app/login/page.test.ts",
                     "expected": d("bytes:page.test.ts")},
                    {"output": "auth.login#outputs.login_form", "path": "src/app/login/_parts/LoginForm.tsx",
                     "expected": d("bytes:LoginForm.tsx")},
                ],
                "create_under": [{"root": "parts", "path": "src/app/login/_parts/"}],
                "delete": [],
                "allowed_imports": [
                    {"module": "next/link", "exports": ["*"], "mode": "runtime", "via": "ambient:nextjs.page"},
                    {"module": "next/navigation", "exports": ["*"], "mode": "runtime", "via": "ambient:nextjs.page"},
                    {"module": "react", "exports": ["*"], "mode": "runtime", "via": "ambient:nextjs.page"},
                    {"module": "react/jsx-runtime", "exports": ["*"], "mode": "runtime", "via": "ambient:nextjs.page"},
                    {"module": "ui-sdk", "exports": ["Form", "Input", "Button", "Link"], "mode": "runtime",
                     "via": "uses:ui.form,ui.input,ui.button,ui.link"},
                    {"module": "src/server/auth/sign_in.ts", "exports": ["default"], "mode": "runtime",
                     "via": "calls:auth.sign_in"},
                    {"module": "src/app/login/_parts/*", "exports": ["*"], "mode": "runtime", "via": "own outputs"},
                ],
            },
            "requirements": [
                requirement("redirects", "valid credentials redirect to @{dashboard.home}", "tested",
                            ["valid_credentials"]),
                requirement("generic_error",
                            "invalid credentials show one generic error and keep the email filled in",
                            "tested", ["invalid_credentials"]),
                requirement("busy_button", "the submit button is disabled while the request is in flight",
                            "tested", ["busy_button"]),
                requirement("forgot_link", "the forgot-password link goes to @{forgot_password}", "none", []),
            ],
        }],
        "context": {
            "inherited": [
                {"source": "project", "normative": True, "hash": PROJECT_HASH, "text": PROJECT_PROMPT},
                {"source": "module:auth", "normative": True, "hash": AUTH_HASH, "text": AUTH_PROMPT},
            ],
            "references": refs,
            "manifest": manifest,
            "budget": {"policy": "context-pack/1", "renderer": "work-order-markdown/1",
                       "tokenizer": "o200k_base", "input_limit": 60000, "reserved_output": 8000,
                       "reserved_feedback": 4000, "used_input": 3870},
        },
        "checks": [
            {"id": "nextjs.typecheck", "definition_hash": d("check:nextjs.typecheck"), "self_check": True},
            {"id": "nextjs.build", "definition_hash": d("check:nextjs.build"), "self_check": True},
            {"id": "nextjs.conformance", "definition_hash": d("check:nextjs.conformance"), "self_check": True},
            {"id": "nextjs.test:auth.login#outputs.tests", "definition_hash": d("check:nextjs.test"),
             "self_check": True},
        ],
        "result_path": ".spec/run/p-2026-11-18-01/r1/tx-4/1/result.json",
        "tools": {"mcp": {"transport": "stdio", "command": ["spec", "mcp", "--dispatch", DISPATCH],
                          "protocol": "2025-06-18"}},
        "limits": {"timeout_seconds": 1800, "result_max_bytes": 65536},
    }
    order["binding"]["order_hash"] = order_hash(order)
    return order


def result(order):
    tests = [("forgot_link", "forgot_link"), ("redirects", "valid_credentials"),
             ("generic_error", "invalid_credentials"), ("busy_button", "busy_button")]
    return {
        "schema": "intentlang.result/1",
        "dispatch": order["dispatch"],
        "order_hash": order["binding"]["order_hash"],
        "status": "submitted",
        "summary": "Added a 'Forgot password?' link below the password field in LoginForm, using ui-sdk Link "
                   "with the /forgot-password route. Added test case forgot_link. No other behaviour changed.",
        "outputs": [],
        "tests": [{"subject": {"target": "auth.login", "requirement": f"auth.login#expect.{k}"},
                   "tests": [{"output": TESTS, "case": c}]} for k, c in tests],
        "notes": ["The route string comes from a local constant; nothing is imported from forgot_password's outputs."],
    }


def review_order():
    order = {
        "schema": "intentlang.review-order/1",
        "dispatch": "p-2026-11-18-01/r1/tx-4/2",
        "binding": {
            "order_hash": "",
            "plan_hash": d("plan:p-2026-11-18-01:r1"),
            "candidate_snapshot": d("snapshot:tx-4:attempt-1:frozen"),
            "graph_hash": d("graph:77d0"),
            "lock_hash": d("lock:5f3a"),
        },
        "subjects": [{
            "id": "s1",
            "subject": {"target": "auth.login", "requirement": "auth.login#expect.forgot_link"},
            "text": "the forgot-password link goes to @{forgot_password}",
            "previous_text": None,
            "contract_hash": contract("the forgot-password link goes to @{auth.forgot_password}"),
            "context_hash": HA["auth.login"]["context"],
            "policy": {"tests": True, "review": True, "checks": [], "approval": False},
            "assess": ["conformance", "coverage"],
            "code": ["src/app/login/_parts/LoginForm.tsx", "tests/app/login/page.test.ts"],
            "coverage_candidates": [{
                "id": "c1",
                "tests": [{"test": {"output": TESTS, "case": "forgot_link"},
                           "oracle": d("oracle:candidate:forgot_link")}],
                "run_receipts": ["run:tx-4:1:nextjs.test:auth.login#outputs.tests"],
            }],
        }],
        "context": {
            "inherited": [
                {"source": "project", "normative": True, "hash": PROJECT_HASH, "text": PROJECT_PROMPT},
                {"source": "module:auth", "normative": True, "hash": AUTH_HASH, "text": AUTH_PROMPT},
            ],
            "references": [
                {"from": "auth.login", "target": "auth.forgot_password", "relation": "mentions",
                 "projection": "public", "viewpoint": "desired", "hash": HA["auth.forgot_password"]["public"],
                 "text": 'nextjs.page forgot_password { route: "/forgot-password" summary: "Request a password reset email" }\n'},
            ],
            "manifest": [res("code", "candidate:tx-4:1:src/app/login/_parts/LoginForm.tsx"),
                         res("code", "candidate:tx-4:1:tests/app/login/page.test.ts")]},
        "verdict_path": ".spec/run/p-2026-11-18-01/r1/tx-4/2/verdict.json",
        "tools": {"mcp": {"transport": "stdio",
                          "command": ["spec", "mcp", "--dispatch", "p-2026-11-18-01/r1/tx-4/2"],
                          "protocol": "2025-06-18"}},
    }
    order["binding"]["order_hash"] = order_hash(order)
    return order


def review_verdict(order):
    return {
        "schema": "intentlang.review-verdict/1",
        "dispatch": order["dispatch"],
        "order_hash": order["binding"]["order_hash"],
        "subjects": [{
            "id": "s1",
            "conformance": {"verdict": "conforms",
                            "reason": "LoginForm renders a Link to /forgot-password directly under the password input.",
                            "locations": ["src/app/login/_parts/LoginForm.tsx:41"]},
            "coverage": [{"candidate": "c1", "verdict": "sufficient",
                          "reason": "The test renders the page, finds the link by its accessible name and asserts its href."}],
        }],
    }


def generator_io():
    """Generator input and output for tx-1: the database binding's migration generator."""
    decls_b = {x["address"]: x for x in GRAPH_BEFORE["declarations"]}
    decls_a = {x["address"]: x for x in GRAPH_AFTER["declarations"]}
    dispatch = "p-2026-11-18-01/r1/tx-1/1"
    inp = {
        "protocol": "intentlang.generator/1",
        "dispatch": dispatch,
        "binding": {"order_hash": "", "plan_hash": d("plan:p-2026-11-18-01:r1"),
                    "input_snapshot": d("snapshot:baseline"), "graph_hash": d("graph:77d0"),
                    "lock_hash": d("lock:5f3a")},
        "subjects": [
            {"address": "core.Workspace", "before": decls_b["core.Workspace"], "after": decls_a["core.Workspace"]},
            {"address": "database", "before": decls_b["database"], "after": decls_a["database"],
             "aggregate": [decls_a["core.Workspace"]]},
        ],
        "envelope": {
            "write": [
                {"output": "core.Workspace#outputs.schema", "path": "src/db/schema/Workspace.ts",
                 "expected": d("bytes:schema/Workspace.ts")},
                {"output": "database#outputs.schema_index", "path": "src/db/schema/index.ts",
                 "expected": d("bytes:schema/index.ts")},
            ],
            "delete": [],
            "move": [],
            "retain": [{"root": "migrations", "path": "drizzle/", "existing": ["0000_init", "0001_workspace"]}],
        },
        "decisions": [],
        "generation_id": "p-2026-11-18-01-tx-1",
        "staging": ".spec/run/p-2026-11-18-01/r1/tx-1/1/staging/",
    }
    inp["binding"]["order_hash"] = order_hash(inp)
    out = {
        "protocol": "intentlang.generator/1",
        "dispatch": dispatch,
        "order_hash": inp["binding"]["order_hash"],
        "status": "written",
        "files": [
            {"op": "update", "target": "core.Workspace#outputs.schema", "relative_path": "src/db/schema/Workspace.ts",
             "digest": d("bytes:schema/Workspace.ts:after")},
            {"op": "create", "target": "retain:migrations", "relative_path": "drizzle/0002_workspace_archived_at.sql",
             "digest": d("bytes:0002.sql")},
        ],
        "retained": [{"id": "0002_workspace_archived_at", "root": "migrations",
                      "depends_on": ["0001_workspace"], "checksum": d("bytes:0002.sql")}],
    }
    return inp, out


def hashes_fixture():
    return {"before": canon.projections(GRAPH_BEFORE), "after": canon.projections(GRAPH_AFTER)}


def main():
    wo = work_order()
    ro = review_order()
    files = {
        "examples/work-order.json": wo,
        "examples/result.json": result(wo),
        "examples/review-order.json": ro,
        "examples/review-verdict.json": review_verdict(ro),
        "examples/graph.before.json": GRAPH_BEFORE,
        "examples/graph.after.json": GRAPH_AFTER,
        "examples/hashes.json": hashes_fixture(),
        "examples/plan.json": plan_fixture(),
        "examples/generator.input.json": generator_io()[0],
        "examples/generator.output.json": generator_io()[1],
    }
    for path, obj in files.items():
        with open(os.path.join(ROOT, path), "w") as f:
            json.dump(obj, f, indent=2, ensure_ascii=False)
            f.write("\n")
        print("wrote", path)


if __name__ == "__main__":
    main()
