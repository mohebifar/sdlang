"""The worked example of docs/spec.md as canonical graphs (before and after the change) and the
plan expected from them. Used by make_examples.py; checked by check_examples.py."""
import copy
import hashlib

NEXT = "npm:@sdlang/nextjs::page"


def d(label):
    return "sha256:" + hashlib.sha256(label.encode()).hexdigest()


def b(name):
    return {"$type": "builtin", "name": name}


def prose(text):
    return {"$prose": text}


def ref(target):
    return {"$ref": target}


def decl(address, kind, sort, fields, *, members=None, edges=(), exported=False, applies=(),
         annotations=(), file="spec/app.sd", line=1):
    module = address.rsplit(".", 1)[0] if "." in address else ""
    out = {
        "address": address, "module": module, "kind": kind, "sort": sort, "exported": exported,
        "fields": fields, "members": members or {}, "lifecycle": "managed",
        "edges": sorted(edges, key=lambda e: (e["relation"], e["target"], e["via"])),
        "source": {"file": file, "line": line},
    }
    if applies:
        out["applies"] = sorted(applies)
    if annotations:
        out["annotations"] = list(annotations)
    return out


def edge(relation, target, via):
    return {"relation": relation, "target": target, "via": via}


def expect(**items):
    return {"expect": {k: {"text": prose(v)} for k, v in items.items()}}


PROJECT_PROMPT = "Server-render by default. All copy in plain English, sentence case.\n"
AUTH_PROMPT = "All auth screens use one centered card layout.\nNever reveal whether an email address is registered.\n"
LOGIN_PROMPT = "Email and password form in the shared auth card.\nOn success go to @{dashboard.home}.\n"
LOGIN_PROMPT_AFTER = LOGIN_PROMPT + "Link to @{auth.forgot_password} under the password field.\n"

_before = {
    "schema": "sdlang.graph/1",
    "lock": d("lock:5f3a"),
    "modules": [
        {"path": "", "imports": [{"alias": "nextjs", "source": "package:npm:@sdlang/nextjs"},
                                 {"alias": "auth", "source": "module:auth"}]},
        {"path": "core", "imports": []},
        {"path": "auth", "header": {"summary": "Sign-in, sign-up and session handling",
                                    "prompt": prose(AUTH_PROMPT),
                                    "skills": [ref("npm:better-auth::skills")]},
         "imports": [{"alias": "nextjs", "source": "package:npm:@sdlang/nextjs"},
                     {"alias": "dashboard", "source": "module:dashboard"}]},
        {"path": "dashboard", "imports": [{"alias": "nextjs", "source": "package:npm:@sdlang/nextjs"},
                                          {"alias": "core", "source": "module:core"}]},
    ],
    "declarations": [
        decl("acme_web", "core:project", "context",
             {"summary": "Customer portal for Acme", "verify": "tested", "prompt": prose(PROJECT_PROMPT)},
             line=4),
        decl("stack", "core:stack", "managed", {
            "language": {"$record": {"name": "typescript"}},
            "runtime": {"$record": {"name": "nodejs", "version": "22"}},
            "framework": {"$record": {"name": "nextjs", "version": "^15.0.0"}},
            "package_manager": {"$record": {"name": "pnpm"}}}, line=12),
        decl("authentication", "core:use", "context",
             {"package": "better-auth", "capability": "authentication"}, line=19),
        decl("database", "core:use", "managed",
             {"package": "drizzle", "capability": "database", "options": {"$record": {"dialect": "postgres"}}},
             edges=[edge("collects", "core.Workspace", "collects")], line=20),
        decl("ui", "core:use", "context", {"package": "ui-sdk", "capability": "ui"}, line=21),
        decl("app_requires_session", "core:rule", "context",
             {"applies_to": {"$select": {"kinds": [NEXT], "where": 'route matches "/app/**"'}}},
             members={"expect": {"redirects": {
                 "text": prose("unauthenticated visitors are redirected to @{auth.login}"), "verify": "tested"}}},
             edges=[edge("mentions", "auth.login", "expect.redirects.text")], line=23),
        decl("core.Workspace", "core:entity", "managed", {"summary": "A team's shared space"},
             members={"fields": {
                 "id": {"type": b("uuid")},
                 "name": {"type": b("string")},
                 "slug": {"type": b("string")},
                 "owner": {"type": {"$type": "ref", "of": {"$type": "named", "name": "npm:better-auth::User"}}},
                 "created_at": {"type": b("timestamp"), "default": {"$init": "now"}}}},
             annotations=[{"name": "core:id", "on": "fields.id", "args": []},
                          {"name": "core:max_length", "on": "fields.name", "args": [80]},
                          {"name": "core:unique", "on": "fields.slug", "args": []}],
             edges=[edge("types", "npm:better-auth::User", "fields.owner")],
             exported=True, file="spec/core/workspace.sd"),
        decl("auth.login", NEXT, "managed", {
            "route": "/login", "summary": "A page where you can sign in",
            "uses": [ref("npm:ui-sdk::Button"), ref("npm:ui-sdk::Form"), ref("npm:ui-sdk::Input")],
            "calls": [ref("auth.sign_in")], "prompt": prose(LOGIN_PROMPT)},
            members=expect(
                redirects="valid credentials redirect to @{dashboard.home}",
                generic_error="invalid credentials show one generic error and keep the email filled in",
                busy_button="the submit button is disabled while the request is in flight"),
            edges=[edge("calls", "auth.sign_in", "calls"),
                   edge("mentions", "dashboard.home", "prompt"),
                   edge("mentions", "dashboard.home", "expect.redirects.text"),
                   edge("uses", "npm:ui-sdk::Button", "uses"), edge("uses", "npm:ui-sdk::Form", "uses"),
                   edge("uses", "npm:ui-sdk::Input", "uses")],
            exported=True, file="spec/auth/login.sd", line=4),
        decl("auth.sign_in", "core:action", "managed", {
            "input": {"$type": "record", "fields": {"email": b("email"), "password": b("string")}},
            "output": {"$type": "named", "name": "npm:better-auth::session"},
            "hint": prose("Use the library's email and password flow rather than custom hashing.")},
            members={"errors": {
                "invalid_credentials": {"summary": "Email or password is wrong", "code": "invalid_credentials"},
                "rate_limited": {"summary": "Too many attempts; try again later", "code": "rate_limited"}}},
            edges=[edge("types", "npm:better-auth::session", "output")],
            file="spec/auth/login.sd", line=20),
        decl("auth.signup", NEXT, "managed", {"route": "/signup", "summary": "Create an account"},
             exported=True, file="spec/auth/signup.sd", line=3),
        decl("dashboard.home", NEXT, "managed", {
            "route": "/app", "summary": "Lists the signed-in user's workspaces",
            "requires": [ref("npm:better-auth::session")], "reads": [ref("core.Workspace")],
            "uses": [ref("npm:ui-sdk::Button"), ref("npm:ui-sdk::Card")],
            "prompt": prose('One card per workspace, newest first.\nThe empty state offers "Create workspace".\n')},
            edges=[edge("reads", "core.Workspace", "reads"), edge("requires", "npm:better-auth::session", "requires"),
                   edge("uses", "npm:ui-sdk::Button", "uses"), edge("uses", "npm:ui-sdk::Card", "uses")],
            applies=["app_requires_session"], exported=True, file="spec/dashboard/home.sd", line=4),
    ],
    "moves": [],
}

_before["modules"].sort(key=lambda m: m["path"])
GRAPH_BEFORE = _before

# Interface fields per kind, as the frontend writes them into the graph: core kinds fix
# theirs (docs/spec.md, Projections and hashes); the nextjs page kind marks `route` @interface.
KIND_INTERFACE = {
    "core:project": [], "core:stack": "all", "core:use": "all", "core:rule": ["applies_to"],
    "core:entity": [], "core:action": ["input", "output"], "core:lib": ["exports"],
    "core:type": [], NEXT: ["route"],
}


def _with_kinds(g):
    g["kinds"] = {k: {"interface": KIND_INTERFACE[k]}
                  for k in sorted({x["kind"] for x in g["declarations"]})}
    return g


def _after():
    g = copy.deepcopy(_before)
    decls = {x["address"]: x for x in g["declarations"]}
    ws = decls["core.Workspace"]
    ws["members"]["fields"]["archived_at"] = {"type": {"$type": "optional", "of": b("timestamp")}}
    decls["auth.sign_in"]["source"]["line"] = 22  # two lines added to login above it
    login = decls["auth.login"]
    login["fields"]["prompt"] = prose(LOGIN_PROMPT_AFTER)
    login["fields"]["uses"] = sorted(login["fields"]["uses"] + [ref("npm:ui-sdk::Link")], key=lambda r: r["$ref"])
    login["members"]["expect"]["forgot_link"] = {"text": prose("the forgot-password link goes to @{auth.forgot_password}")}
    login["edges"] = sorted(login["edges"] + [
        edge("mentions", "auth.forgot_password", "prompt"),
        edge("mentions", "auth.forgot_password", "expect.forgot_link.text"),
        edge("uses", "npm:ui-sdk::Link", "uses")], key=lambda e: (e["relation"], e["target"], e["via"]))
    g["declarations"] += [
        decl("auth.forgot_password", NEXT, "managed", {
            "route": "/forgot-password", "summary": "Request a password reset email",
            "uses": [ref("npm:ui-sdk::Button"), ref("npm:ui-sdk::Form"), ref("npm:ui-sdk::Input")],
            "calls": [ref("auth.request_reset")]},
            members=expect(same_message="submitting any email shows the same confirmation message"),
            edges=[edge("calls", "auth.request_reset", "calls"),
                   edge("uses", "npm:ui-sdk::Button", "uses"), edge("uses", "npm:ui-sdk::Form", "uses"),
                   edge("uses", "npm:ui-sdk::Input", "uses")],
            exported=True, file="spec/auth/forgot_password.sd", line=3),
        decl("auth.request_reset", "core:action", "managed", {
            "input": {"$type": "record", "fields": {"email": b("email")}},
            "output": {"$type": "record", "fields": {"sent": b("boolean")}},
            "uses": [ref("npm:better-auth::password_reset")],
            "hint": prose("Use the library's reset flow.")},
            members={"errors": {"rate_limited": {"summary": "Too many requests; try again later",
                                                 "code": "rate_limited"}}},
            edges=[edge("uses", "npm:better-auth::password_reset", "uses")],
            file="spec/auth/forgot_password.sd", line=13),
    ]
    g["declarations"].sort(key=lambda x: x["address"])
    return g


GRAPH_AFTER = _with_kinds(_after())
GRAPH_BEFORE["declarations"].sort(key=lambda x: x["address"])
_with_kinds(GRAPH_BEFORE)


def plan_fixture():
    tx = lambda n: f"tx-{n}"
    return {
        "schema": "sdlang.plan/1",
        "plan": "p-2026-11-18-01",
        "revision": 1,
        "binding": {
            "baseline_commit": "git:" + hashlib.sha1(b"commit:baseline").hexdigest(),
            "graph_before": d("graph:before"),
            "graph_after": d("graph:77d0"),
            "lock_hash": d("lock:5f3a"),
            "state_digest": d("state:c9e1"),
            "fingerprints": {"src/app/login/page.tsx": d("bytes:page.tsx"),
                             "src/app/signup/page.tsx": d("bytes:signup.tsx:hand-edited")},
        },
        "entries": [
            {"address": "core.Workspace", "identity": "same", "change": "update", "causes": ["own_projection"],
             "strategy": "generator", "transaction": tx(1)},
            {"address": "database", "identity": "same", "change": "update", "causes": ["aggregate"],
             "strategy": "generator", "transaction": tx(1)},
            {"address": "dashboard.home", "identity": "same", "change": "none",
             "obligations": [{"kind": "verify", "because": "consumed:core.Workspace", "compatibility": "reader-compatible"}],
             "transaction": tx(1)},
            {"address": "auth.request_reset", "identity": "new", "change": "create", "causes": ["new"],
             "strategy": "agent", "transaction": tx(2)},
            {"address": "auth.forgot_password", "identity": "new", "change": "create", "causes": ["new"],
             "strategy": "agent", "transaction": tx(3)},
            {"address": "auth.login", "identity": "same", "change": "update", "causes": ["own_projection"],
             "strategy": "agent", "transaction": tx(4)},
        ],
        "blocked": [
            {"address": "auth.signup", "reason": "drift", "path": "src/app/signup/page.tsx",
             "expected": d("bytes:signup.tsx:baseline"), "actual": d("bytes:signup.tsx:hand-edited"),
             "resolutions": ["adopt", "absorb", "overwrite", "repair"]},
        ],
        "transactions": [
            {"id": tx(1), "entries": ["core.Workspace", "database"], "after": [],
             "obligations": {"dashboard.home": {"kinds": ["verify"],
                                                "contingent": ["src/app/app/page.tsx", "tests/app/app/page.test.ts"]}}},
            {"id": tx(2), "entries": ["auth.request_reset"], "after": [], "obligations": {}},
            {"id": tx(3), "entries": ["auth.forgot_password"], "after": [tx(2)], "obligations": {}},
            {"id": tx(4), "entries": ["auth.login"], "after": [], "obligations": {}},
        ],
        "reviews": {"coverage": ["auth.forgot_password#expect.same_message", "auth.login#expect.forgot_link",
                               "auth.request_reset#errors.rate_limited"]},
        "rollout": {"prerequisites": [{"id": "migration:workspace_archived_at", "before": "code reading core.Workspace#fields.archived_at"}],
                    "readiness": "unknown"},
    }
