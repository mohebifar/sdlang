#!/usr/bin/env python3
"""Throwaway checker for GRAMMAR.ebnf.

A hand-written lexer and recursive-descent parser that mirrors GRAMMAR.ebnf
production by production. It checks that:

  * every ```spec block in SPEC.md parses as a complete File;
  * every ```json block in SPEC.md is valid JSON;
  * a set of built-in negative cases is rejected.

  * every protocol example in examples/ validates against schemas/, its binding
    hash is recomputable, and replies echo their orders (needs `pip install jsonschema`).

Usage: python3 tools/check_examples.py [SPEC.md ...]
Exit status is non-zero if anything fails.
"""
import copy
import json
import os
import re
import sys

KEYWORDS = {
    "import", "export", "extern", "use", "for", "as", "moved", "let", "type",
    "kind", "annotation", "implements", "project", "stack", "module",
    "package", "ref", "self", "select", "where", "and", "or", "not", "exists",
    "meta", "matches", "in", "contains", "true", "false", "null",
}
PUNCT = ["->", "==", "!=", "<=", ">=", "{", "}", "[", "]", "(", ")", ",", ":",
         "=", "|", "?", ".", "@", "*", "#", "<", ">"]
WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
NUM_RE = re.compile(r"-?[0-9]+(\.[0-9]+)?")
PATH_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)(\.[A-Za-z_][A-Za-z0-9_]*)*")


class ParseError(Exception):
    pass


class Tok:
    def __init__(self, kind, value, line):
        self.kind, self.value, self.line = kind, value, line

    def __repr__(self):
        return f"{self.kind}:{self.value!r}@{self.line}"


# ----------------------------------------------------------------- lexer

def scan_string_parts(body, line, prose):
    """Ordered string scanning per GRAMMAR.ebnf; returns list of parts."""
    parts, i, text = [], 0, []
    while i < len(body):
        if body.startswith("@@{", i) or body.startswith("$${", i):
            text.append(body[i + 1:i + 3]); i += 3; continue
        if body.startswith("@{", i) or body.startswith("${", i):
            close = body.find("}", i)
            inner = body[i + 2:close] if close != -1 else ""
            if body[i] == "@":
                ok = inner == "self" or (PATH_RE.fullmatch(inner) and inner.split(".")[0] not in KEYWORDS)
            else:
                ok = re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)?", inner) is not None
            if close == -1 or not ok:
                raise ParseError(f"line {line}: malformed {body[i:i+2]}...}} in string")
            if text:
                parts.append(("text", "".join(text))); text = []
            parts.append(("mention" if body[i] == "@" else "interp", inner))
            i = close + 1; continue
        if not prose and body[i] == "\\":
            nxt = body[i + 1:i + 2]
            if nxt in ('"', "\\"):
                text.append(nxt); i += 2; continue
            if nxt in ("n", "t"):
                text.append({"n": "\n", "t": "\t"}[nxt]); i += 2; continue
            m = re.match(r"u\{([0-9A-Fa-f]+)\}", body[i + 1:])
            if m:
                cp = int(m.group(1), 16)
                if cp > 0x10FFFF or 0xD800 <= cp <= 0xDFFF:
                    raise ParseError(f"line {line}: \\u{{{m.group(1)}}} is not a scalar value")
                text.append(chr(cp)); i += 1 + len(m.group(0)); continue
            raise ParseError(f"line {line}: bad escape \\{nxt}")
        if prose and body.startswith('\\"""', i):
            text.append('"""'); i += 4; continue
        text.append(body[i]); i += 1
    if text:
        parts.append(("text", "".join(text)))
    return parts


def dedent_prose(raw, line):
    if not raw.startswith("\n"):
        return raw  # inline prose: no dedent
    lines = raw[1:].split("\n")
    closing = lines[-1]
    if closing.strip(" \t") != "":
        raise ParseError(f"line {line}: closing \"\"\" of a block literal must be on its own line")
    content = lines[:-1]
    for ln in content + [closing]:
        lead = ln[:len(ln) - len(ln.lstrip(" \t"))]
        if "\t" in lead:
            raise ParseError(f"line {line}: tab in prose indentation")
    nonblank = [ln for ln in content if ln.strip()]
    n = min((len(ln) - len(ln.lstrip(" ")) for ln in nonblank), default=0)
    out = [(ln[n:] if ln.strip() else "") for ln in content]
    return "\n".join(out) + ("\n" if out else "")


def lex(src):
    src = src.replace("\r\n", "\n")
    toks, i, line = [], 0, 1
    while i < len(src):
        c = src[i]
        if c in " \t":
            i += 1; continue
        if c == "\n":
            toks.append(Tok("NL", "\n", line)); line += 1; i += 1; continue
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = len(src) if j == -1 else j
            continue
        if src.startswith('"""', i):
            j, k = i + 3, None
            while j < len(src):
                if src.startswith('\\"""', j):
                    j += 4; continue
                if src.startswith('"""', j):
                    k = j; break
                j += 1
            if k is None:
                raise ParseError(f"line {line}: unterminated prose string")
            raw = src[i + 3:k]
            parts = scan_string_parts(dedent_prose(raw, line), line, prose=True)
            toks.append(Tok("STRING", ("prose", parts), line))
            line += raw.count("\n"); i = k + 3; continue
        if c == '"':
            j = i + 1
            while j < len(src) and src[j] != '"':
                if src[j] == "\n":
                    raise ParseError(f"line {line}: line break inside a line string")
                j += 2 if src[j] == "\\" else 1
            if j >= len(src):
                raise ParseError(f"line {line}: unterminated string")
            parts = scan_string_parts(src[i + 1:j], line, prose=False)
            toks.append(Tok("STRING", ("line", parts), line)); i = j + 1; continue
        m = NUM_RE.match(src, i)
        if m and (c.isdigit() or (c == "-" and src[i + 1:i + 2].isdigit())):
            toks.append(Tok("NUMBER", m.group(0), line)); i = m.end(); continue
        m = WORD_RE.match(src, i)
        if m:
            toks.append(Tok("WORD", m.group(0), line)); i = m.end(); continue
        for p in PUNCT:
            if src.startswith(p, i):
                toks.append(Tok("P", p, line)); i += len(p); break
        else:
            raise ParseError(f"line {line}: unexpected character {c!r}")
    toks.append(Tok("EOF", None, line))
    return toks


# ----------------------------------------------------------------- parser

class Parser:
    def __init__(self, toks):
        self.t, self.i = toks, 0

    # helpers
    def peek(self, k=0):
        return self.t[self.i + k]

    def at(self, kind, value=None, k=0):
        tk = self.peek(k)
        return tk.kind == kind and (value is None or tk.value == value)

    def atp(self, v, k=0):
        return self.at("P", v, k)

    def atkw(self, v, k=0):
        return self.at("WORD", v, k)

    def err(self, msg):
        tk = self.peek()
        raise ParseError(f"line {tk.line}: {msg}, found {tk.kind} {tk.value!r}")

    def take(self):
        tk = self.t[self.i]; self.i += 1; return tk

    def expectp(self, v):
        if not self.atp(v):
            self.err(f"expected '{v}'")
        return self.take()

    def expectkw(self, v):
        if not self.atkw(v):
            self.err(f"expected '{v}'")
        return self.take()

    def gap(self):
        while self.at("NL"):
            self.take()

    def word(self):
        if not self.at("WORD"):
            self.err("expected a name")
        return self.take().value

    def ident(self):
        if not self.at("WORD") or self.peek().value in KEYWORDS:
            self.err("expected an identifier (not a keyword)")
        return self.take().value

    def line_string(self):
        if not (self.at("STRING") and self.peek().value[0] == "line"):
            self.err("expected a line string")
        return self.take()

    def path(self):
        segs = [self.ident()]
        while self.atp(".") and self.at("WORD", k=1):
            self.take(); segs.append(self.word())
        return ".".join(segs)

    def sep_or_close(self, close):
        """After an item: consume a Sep; return True if the list closed."""
        had = False
        if self.atp(","):
            self.take(); self.gap(); had = True
        elif self.at("NL"):
            self.gap(); had = True
        if self.atp(close):
            self.take(); return True
        if not had:
            self.err(f"expected ',' or a line break or '{close}'")
        return False

    def seq(self, open_, close, item):
        self.expectp(open_); self.gap()
        if self.atp(close):
            self.take(); return
        while True:
            item()
            if self.sep_or_close(close):
                return

    # File
    def file(self):
        self.gap()
        while not self.at("EOF"):
            self.statement()
            if self.at("EOF"):
                break
            if not self.at("NL"):
                self.err("expected a line break between statements")
            self.gap()

    def statement(self):
        if self.atkw("import"):
            self.take()
            if self.atkw("module"):
                self.take(); self.path()
            elif self.atkw("package"):
                self.take(); self.line_string()
            else:
                self.path()
            if self.atkw("as"):
                self.take(); self.ident()
        elif self.atkw("use"):
            self.take(); self.line_string(); self.expectkw("for"); self.ident()
            if self.atp("{"):
                self.record()
        elif self.atkw("moved"):
            self.take()
            if self.atkw("module"):
                self.take()
            self.address(); self.expectp("->"); self.address()
        elif self.atkw("project"):
            self.take(); self.ident(); self.block()
        elif self.atkw("stack"):
            self.take(); self.block()
        elif self.atkw("module"):
            self.take(); self.block()
        else:
            if self.atkw("export"):
                self.take()
            self.exportable()

    def address(self):
        self.word()
        while self.atp("."):
            self.take(); self.word()
        if self.atp("#"):
            self.take(); self.word(); self.expectp("."); self.word()
            while self.atp("."):
                self.take(); self.word()

    def exportable(self):
        if self.atkw("let"):
            self.take(); self.ident(); self.expectp("="); self.gap(); self.expr()
        elif self.atkw("type"):
            self.take(); self.ident(); self.expectp("="); self.gap(); self.expr()
        elif self.atkw("kind"):
            self.take(); self.ident(); self.block()
        elif self.atkw("annotation"):
            self.take(); self.ident(); self.block()
        elif self.atkw("implements"):
            self.take(); self.word(); self.block()
        else:
            self.named_decl()

    def named_decl(self):
        if self.atkw("extern"):
            self.take()
        self.path(); self.ident()
        while self.atp("@"):
            self.annotation()
        self.block()

    # Bodies
    def block(self):
        self.seq("{", "}", self.member)

    def member(self):
        self.word()
        if self.atp(":"):
            self.take(); self.gap(); self.field_rest()
        elif self.atp("{"):
            self.block()
        else:
            self.err("expected ':' or '{' after a member name")

    def field_rest(self):
        self.expr()
        if self.atp("="):
            self.take(); self.gap(); self.expr()
        while self.atp("@"):
            self.annotation()

    def annotation(self):
        self.expectp("@"); self.path()
        if self.atp("("):
            self.seq("(", ")", self.expr)

    def record(self):
        def rf():
            self.word(); self.expectp(":"); self.gap(); self.field_rest()
        self.seq("{", "}", rf)

    # Expressions
    def expr(self):
        if self.atkw("select"):
            self.selector()
        else:
            self.union()

    def union(self):
        self.postfix()
        while self.atp("|"):
            self.take(); self.gap(); self.postfix()

    def postfix(self):
        if self.atkw("ref"):
            self.take()
        self.primary()
        if self.atp("?"):
            self.take()

    def primary(self):
        tk = self.peek()
        if tk.kind in ("STRING", "NUMBER"):
            self.take()
        elif tk.kind == "WORD" and tk.value in ("true", "false", "null", "self"):
            self.take()
        elif tk.kind == "WORD":
            self.path()
            if self.atp("{"):
                self.record()
        elif self.atp("["):
            self.seq("[", "]", self.expr)
        elif self.atp("{"):
            self.record()
        elif self.atp("("):
            self.take(); self.gap(); self.expr(); self.gap(); self.expectp(")")
        else:
            self.err("expected an expression")

    def selector(self):
        self.expectkw("select")
        if self.atp("*"):
            self.take()
        elif self.atp("["):
            k = 1
            while self.at("NL", k=k):
                k += 1
            if self.atp("]", k=k):
                self.err("a selector kind list may not be empty")
            self.seq("[", "]", self.path)
        else:
            self.path()
        if self.atkw("where"):
            self.take(); self.gap(); self.predicate()

    def predicate(self):
        self.conjunction()
        while self.atkw("or"):
            self.take(); self.gap(); self.conjunction()

    def conjunction(self):
        self.negation()
        while self.atkw("and"):
            self.take(); self.gap(); self.negation()

    def negation(self):
        if self.atkw("not"):
            self.take(); self.gap(); self.negation()
        elif self.atp("("):
            self.take(); self.gap(); self.predicate(); self.gap(); self.expectp(")")
        else:
            self.test()

    def test(self):
        if self.atkw("exists"):
            self.take(); self.access(); return
        self.access()
        tk = self.peek()
        ops = {"==", "!=", "<", "<=", ">", ">="}
        if (tk.kind == "P" and tk.value in ops) or (tk.kind == "WORD" and tk.value in ("matches", "in", "contains")):
            self.take(); self.gap(); self.operand()
        else:
            self.err("expected a comparison operator")

    def access(self):
        if self.atkw("meta"):
            self.take(); self.expectp(".")
            if self.peek().value not in ("module", "name", "kind"):
                self.err("meta.<module|name|kind> expected")
            self.take(); return
        self.word()
        while self.atp("."):
            self.take(); self.word()

    def operand(self):
        tk = self.peek()
        if tk.kind in ("STRING", "NUMBER") or (tk.kind == "WORD" and tk.value in ("true", "false", "null")):
            self.take()
        elif self.atp("["):
            self.seq("[", "]", self.operand)
        else:
            self.path()


def parse(src):
    Parser(lex(src)).file()


# ----------------------------------------------------------------- driver

NEGATIVE = {
    "union continued on next line": 'type Plan = "free"\n  | "team"\n',
    "two statements on one line": "import a import b\n",
    "keyword as declaration name": "entity type {\n}\n",
    "unnamed named declaration": "nextjs.page {\n}\n",
    "named stack": "stack main {\n}\n",
    "mention outside a string": "action a {\n  to: @{login}\n}\n",
    "malformed mention": 'action a {\n  prompt: "go to @{ login }"\n}\n',
    "missing separator": "action a {\n  input: { a: string b: string }\n}\n",
    "closing prose not on own line": 'action a {\n  prompt: """\n    text"""\n}\n',
    "tab indentation in prose": 'action a {\n  prompt: """\n\ttext\n  """\n}\n',
    "raw newline in line string": 'action a {\n  summary: "a\nb"\n}\n',
    "repeated export": "export export action a {\n}\n",
    "member address without collection": "moved a.b#redirects -> a.b#expect.x\n",
    "template is not v0": "template crud(e: entity) {\n}\n",
    "record inside a selector operand list": "rule r {\n  applies_to: select * where config in [{ mode: \"a\" }]\n}\n",
    "empty selector kind list": "rule r {\n  applies_to: select []\n}\n",
    "meta with unknown field": "rule r {\n  applies_to: select * where meta.path == \"x\"\n}\n",
}

POSITIVE = {
    "keyword field names": 'ui.component Button {\n  type: "button"\n  module { kind: "x" }\n}\n',
    "union continued after bar": 'type Plan = "free" |\n  "team"\n',
    "literal openers": 'action a {\n  prompt: "write @@{x} and $${y} and a@b.com"\n}\n',
    "selector across lines": 'rule r {\n  applies_to: select [nextjs.page, nextjs.endpoint] where\n    route matches\n      "/app/**" and not exists layout\n  require: "x"\n}\n',
    "empty file": "",
    "comment before statement boundary": "import a // note\nimport b\n",
}


def check_spec_md(path):
    text = open(path).read()
    failures, count = [], {"spec": 0, "json": 0}
    for m in re.finditer(r"^```(\w[\w-]*)?\n(.*?)^```", text, re.S | re.M):
        lang, body = m.group(1) or "", m.group(2)
        line = text[:m.start()].count("\n") + 1
        if lang == "spec":
            count["spec"] += 1
            try:
                parse(body)
            except ParseError as e:
                failures.append(f"{path}:{line} spec block: {e}")
        elif lang == "json":
            count["json"] += 1
            try:
                json.loads(body)
            except json.JSONDecodeError as e:
                failures.append(f"{path}:{line} json block: {e}")
        elif lang == "":
            failures.append(f"{path}:{line} untagged code block (tag it spec, json, console, text...)")
    return count, failures


def adapter_rules(path, check):
    """{rule: [kinds]} from the `rules { }` section of `export check <check>` in an adapter."""
    text = open(path).read()
    block = re.search(r"export check " + check + r" \{(.*?)\n\}", text, re.S).group(1)
    section = re.search(r"rules \{(.*?)\n  \}", block, re.S).group(1)
    return {m.group(1): [k.strip() for k in m.group(2).split(",")]
            for m in re.finditer(r"(\w+): \[([^\]]*)\]", section)}


def _short_kind(kind):
    return kind.split("::")[-1].split(":")[-1]


def conformance_pair_problems(ci, cr, rules, graph):
    out = []
    if ci["check"]["id"] != "nextjs.conformance" or ci["purpose"] != "conformance":
        out.append("check input: not the adapter's conformance check")
    if (cr["check"], cr["snapshot"]) != (ci["check"], ci["snapshot"]):
        out.append("check result: check or snapshot does not echo the input")
    kind = _short_kind(graph[ci["subject"]]["kind"])
    applicable = sorted(r for r, kinds in rules.items() if kind in kinds)
    if sorted(ci["rules"]) != applicable:
        out.append(f"check input: rules {sorted(ci['rules'])} are not the adapter's rules for {kind}: {applicable}")
    pairs = sorted((r["subject"], r["rule"]) for r in cr["results"])
    if pairs != sorted((ci["subject"], r) for r in applicable):
        out.append(f"check result: reported {pairs}, expected each applicable rule once for {ci['subject']}")
    return out


def _canon():
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import canon
    return canon


def test_pair_problems(ti, tr):
    out = []
    if (tr["check"], tr["snapshot"], tr["profile"], tr["inputs"]) != (
            ti["check"], ti["snapshot"], ti["profile"], ti.get("inputs")):
        out.append("test result: check, snapshot, profile or inputs do not echo the input")
    if tr["output"] not in [o["member"] for o in ti["outputs"]]:
        out.append("test result: output is not the input's test output")
    tags = [c["case"] for c in tr["cases"]]
    pairs = [[c["case"], i["key"]] for c in tr["cases"] for i in c["instances"]]
    if len(set(tags)) != len(tags) or len({tuple(p) for p in pairs}) != len(pairs):
        out.append("test result: duplicate case tag or instance key")
    if tr["selection"] == "complete" and tr["inventory"] != _canon().sha(sorted(pairs)):
        out.append("test result: inventory does not match the reported instances")
    reported = {tuple(p) for p in pairs}
    missing = sorted((e["case"], k) for e in ti.get("expected_cases", []) for k in e["keys"]
                     if (e["case"], k) not in reported)
    if missing:
        out.append(f"test result: expected instances missing: {missing}")
    return out


SCHEMA_EXAMPLES = [
    ("work-order", "examples/work-order.json"),
    ("result", "examples/result.json"),
    ("review-order", "examples/review-order.json"),
    ("review-verdict", "examples/review-verdict.json"),
    ("graph", "examples/graph.before.json"),
    ("graph", "examples/graph.after.json"),
    ("plan", "examples/plan.json"),
    ("package-manifest", "examples/package.json"),
    ("generator#input", "examples/generator.input.json"),
    ("generator#output", "examples/generator.output.json"),
    ("check#input", "examples/check.input.json"),
    ("check#conformance_result", "examples/check.conformance-result.json"),
    ("check#input", "examples/check.test-input.json"),
    ("check#test_result", "examples/check.test-result.json"),
]

_R = {"schema": "intlang.result/1", "dispatch": "p-1/r1/tx-1/1",
      "order_hash": "sha256:" + "0" * 64, "summary": "s"}
_EVID = {"references": ["ctx:code:000000000000"], "attempted": "read the stack and lock"}

# Result files the result schema must accept.
GOOD_RESULTS = {
    "blocked spec question": dict(_R, status="blocked", blocked={
        "kind": "spec_question", "question": "Should the link open in a new tab?",
        "subjects": [{"target": "auth.login", "requirement": "auth.login#expect.forgot_link"}],
        "evidence": _EVID}),
    "blocked dependency request": dict(_R, status="blocked", blocked={
        "kind": "authority_request",
        "request": {"type": "dependency", "package": "zod", "version": "^3.23.0", "capability": "validation"},
        "evidence": _EVID}),
    "repair with no automated reproduction": dict(_R, status="submitted", regression=[{
        "subject": {"target": "auth.login", "requirement": "auth.login#expect.busy_button"},
        "kind": "not_automatable", "reason": "layout overlap only visible at 320px in Safari",
        "artifacts": ["src/app/login/_parts/evidence/before.png"]}]),
}

# Result files the result schema must reject.
BAD_RESULTS = {
    "blocked without blocked object": dict(_R, status="blocked"),
    "submitted with blocked object": dict(_R, status="submitted", blocked={
        "kind": "environment", "problem": "registry down", "evidence": _EVID}),
    "positional requirement id": dict(_R, status="submitted", tests=[{
        "subject": {"target": "auth.login", "requirement": "auth.login#e1"},
        "tests": [{"output": "auth.login#outputs.tests", "case": "c"}]}]),
    "regression with test and reason": dict(_R, status="submitted", regression=[{
        "subject": {"target": "a.b", "requirement": "a.b#expect.c"},
        "kind": "automated", "tests": [{"output": "a.b#outputs.tests", "case": "c"}], "reason": "x"}]),
    "regression without subject": dict(_R, status="submitted", regression=[{
        "kind": "automated", "tests": [{"output": "a.b#outputs.tests", "case": "c"}]}]),
    "short digest": dict(_R, status="submitted", order_hash="sha256:abcd"),
    "blocker without evidence": dict(_R, status="blocked", blocked={
        "kind": "environment", "problem": "registry down"}),
}


def _canonical_hash(obj, field):
    """order_hash: the same canonical JSON as graph projections (tools/canon.py), NFC included."""
    import sys as _sys
    _sys.path.insert(0, "tools")
    import canon
    clone = json.loads(json.dumps(obj))
    del clone[field]["order_hash"]
    return canon.sha(clone)


def check_graphs(before, after, plan):
    """Graphs are closed (no dangling edges), their projection hashes match
    examples/hashes.json as recomputed by tools/canon.py, and plan.json's change set and
    ordering follow from the two graphs."""
    import sys as _sys
    _sys.path.insert(0, "tools")
    import canon
    failures = []
    for label, g in (("before", before), ("after", after)):
        addrs = {x["address"] for x in g["declarations"]}
        for x in g["declarations"]:
            for e in x["edges"]:
                if "::" not in e["target"] and e["target"] not in addrs:
                    failures.append(f"graph.{label}: {x['address']} has a dangling {e['relation']} edge to {e['target']}")
            for r in x.get("applies", []):
                if r not in addrs:
                    failures.append(f"graph.{label}: {x['address']} applies unknown rule {r}")
    expected = json.load(open("examples/hashes.json"))
    if expected != {"before": canon.projections(before), "after": canon.projections(after)}:
        failures.append("examples/hashes.json does not match tools/canon.py over the graphs")
    vectors = json.load(open("examples/hash-vectors.json"))["vectors"]
    for v in vectors:
        if canon.canonical(v["value"]) != v["canonical"] or canon.sha(v["value"]) != v["sha256"]:
            failures.append(f"hash vector mismatch: {v['name']}")
    if vectors[1]["sha256"] != vectors[2]["sha256"]:
        failures.append("hash vectors: NFC-equivalent strings must hash equally")
    # Inherited normative context changes are review obligations, never updates.
    import copy
    variant = copy.deepcopy(before)
    for m in variant["modules"]:
        if m["path"] == "auth":
            m["header"]["prompt"] = {"$prose": "All auth screens use a split layout.\n"}
    dv = canon.diff_graphs(before, variant)
    if dv["update"] or dv["create"] or sorted(dv["obligations"]) != ["auth.login", "auth.sign_in", "auth.signup"]:
        failures.append(f"module-prompt-only change derived {dv}, expected review obligations only")
    nested = copy.deepcopy(before)
    nested["modules"].append({"path": "auth.child", "imports": []})
    nested["declarations"].append({"address": "auth.child.page", "module": "auth.child",
        "kind": "npm:@intlang/nextjs::page", "sort": "managed", "exported": False,
        "fields": {"route": "/child"}, "members": {}, "edges": [], "lifecycle": "managed",
        "source": {"file": "spec/auth/child/page.int", "line": 1}})
    nested_variant = copy.deepcopy(nested)
    for m in nested_variant["modules"]:
        if m["path"] == "auth":
            m["header"]["prompt"] = {"$prose": "All auth screens use a split layout.\n"}
    if "auth.child.page" not in canon.diff_graphs(nested, nested_variant)["obligations"]:
        failures.append("an ancestor module's prompt change must reach declarations in child modules")
    derived = canon.diff_graphs(before, after)
    by_change = {}
    for e in plan["entries"]:
        by_change.setdefault(e["change"], set()).add(e["address"])
    for change in ("create", "update", "delete"):
        if set(derived[change]) != by_change.get(change, set()):
            failures.append(f"plan.json {change} set {sorted(by_change.get(change, set()))} "
                            f"!= derived {derived[change]}")
    if set(derived["obligations"]) != by_change.get("none", set()):
        failures.append(f"plan.json obligation-only entries {sorted(by_change.get('none', set()))} "
                        f"!= derived {sorted(derived['obligations'])}")
    tx_of = {e["address"]: e["transaction"] for e in plan["entries"]}
    after_of = {t["id"]: set(t["after"]) for t in plan["transactions"]}
    decls = {x["address"]: x for x in after["declarations"]}
    for a, x in decls.items():
        for e in x["edges"]:
            if e["relation"] in canon.CONSUMING and e["target"] in tx_of and a in tx_of:
                ta, tb = tx_of[a], tx_of[e["target"]]
                if ta != tb and e["target"] in derived["create"] and tb not in after_of.get(ta, set()):
                    failures.append(f"plan.json: {ta} ({a}) must run after {tb} ({e['target']})")
    return failures


def check_schemas():
    try:
        import jsonschema
        from referencing import Registry, Resource
    except ImportError:
        return ["jsonschema not installed: cannot validate schemas (pip install jsonschema)"], 0
    schemas = {}
    for name in ["common", "work-order", "result", "review-order", "review-verdict", "graph", "plan",
                 "package-manifest", "generator", "mcp-tools", "check"]:
        schemas[name] = json.load(open(f"schemas/{name}.schema.json"))
        jsonschema.Draft202012Validator.check_schema(schemas[name])
    registry = Registry().with_resources(
        [(s["$id"], Resource.from_contents(s)) for s in schemas.values()])

    def validator(name):
        if "#" in name:  # a sub-schema such as generator#input
            base, part = name.split("#")
            ref = {"$ref": schemas[base]["$id"] + "#/$defs/" + part}
            return jsonschema.Draft202012Validator(ref, registry=registry)
        return jsonschema.Draft202012Validator(schemas[name], registry=registry)

    failures, docs = [], {}
    for name, path in SCHEMA_EXAMPLES:
        doc = json.load(open(path))
        docs[path] = docs[name] = doc
        for err in validator(name).iter_errors(doc):
            failures.append(f"{path}: {err.json_path}: {err.message[:200]}")
    wo, rs, ro, rv = (docs[k] for k in ("work-order", "result", "review-order", "review-verdict"))
    # Check protocol: pairs of input and result, checked against the adapter's declared rules.
    graph = {x["address"]: x for x in docs["examples/graph.after.json"]["declarations"]}
    rules = adapter_rules("examples/nextjs-adapter.int", "conformance")
    ci, cr = docs["examples/check.input.json"], docs["examples/check.conformance-result.json"]
    ti, tr = docs["examples/check.test-input.json"], docs["examples/check.test-result.json"]
    failures += conformance_pair_problems(ci, cr, rules, graph)
    failures += test_pair_problems(ti, tr)
    # Mutations that must be caught: an extra subject, an invented rule, a dropped case.
    extra = copy.deepcopy(cr)
    extra["results"].append(dict(cr["results"][0], subject="unrequested.page"))
    invented_in, invented_out = copy.deepcopy(ci), copy.deepcopy(cr)
    invented_in["rules"] = ["invented_rule"]
    invented_out["results"] = [dict(cr["results"][0], rule="invented_rule")]
    dropped = copy.deepcopy(tr)
    dropped["cases"] = dropped["cases"][1:]
    no_instance = copy.deepcopy(tr)
    no_instance["cases"][0]["instances"] = no_instance["cases"][0]["instances"][:1]
    duplicate = copy.deepcopy(tr)
    duplicate["cases"].append(copy.deepcopy(duplicate["cases"][0]))
    other_profile = dict(tr, profile="integration-v4")
    for label, problems in [("extra subject", conformance_pair_problems(ci, extra, rules, graph)),
                            ("invented rule", conformance_pair_problems(invented_in, invented_out, rules, graph)),
                            ("dropped case", test_pair_problems(ti, dropped)),
                            ("dropped instance", test_pair_problems(ti, no_instance)),
                            ("duplicate case", test_pair_problems(ti, duplicate)),
                            ("changed profile", test_pair_problems(ti, other_profile))]:
        if not problems:
            failures.append(f"check protocol: mutation not caught: {label}")
    # Inputs the check schema must reject: purpose-specific fields missing or misplaced.
    bad_inputs = {
        "test input without expected cases": {k: v for k, v in ti.items() if k != "expected_cases"},
        "test input without run inputs": {k: v for k, v in ti.items() if k != "inputs"},
        "conformance input without rules": {k: v for k, v in ci.items() if k != "rules"},
        "conformance input with expected cases": dict(ci, expected_cases=ti["expected_cases"]),
        "declaration scope with null subject": dict(ci, subject=None),
    }
    for label, doc in bad_inputs.items():
        if not list(validator("check#input").iter_errors(doc)):
            failures.append(f"check schema accepted a bad input: {label}")
    # Bindings: order hashes are recomputable; replies echo the dispatch and hash.
    gi, go = docs["generator#input"], docs["generator#output"]
    if _canonical_hash(gi, "binding") != gi["binding"]["order_hash"]:
        failures.append("generator input: binding.order_hash does not match")
    if (go["dispatch"], go["order_hash"]) != (gi["dispatch"], gi["binding"]["order_hash"]):
        failures.append("generator output: dispatch/order_hash do not echo the input")
    for name, order in (("work-order", wo), ("review-order", ro)):
        if _canonical_hash(order, "binding") != order["binding"]["order_hash"]:
            failures.append(f"{name}: binding.order_hash does not match the canonical order")
    for reply, order, label in ((rs, wo, "result"), (rv, ro, "review-verdict")):
        if (reply["dispatch"], reply["order_hash"]) != (order["dispatch"], order["binding"]["order_hash"]):
            failures.append(f"{label}: dispatch/order_hash do not echo the order")
    # Cross-document rules the schema cannot express.
    order_subjects = {(r["subject"]["target"], r["subject"]["requirement"])
                      for s in wo["subjects"] for r in s["requirements"]}
    for t in rs.get("tests", []):
        if (t["subject"]["target"], t["subject"]["requirement"]) not in order_subjects:
            failures.append(f"result: test binding for unknown subject {t['subject']}")
    stale = set(m["resource"] for m in wo["context"]["manifest"]) & set(ro["context"]["manifest"])
    if stale:
        failures.append(f"review-order: reuses pre-edit resource handles {sorted(stale)}")
    failures += check_graphs(docs["examples/graph.before.json"], docs["examples/graph.after.json"],
                             docs["plan"])
    # Hashes in the work order must name the same canonical objects as the graph fixtures.
    import sys as _sys
    _sys.path.insert(0, "tools")
    import canon
    hb = canon.projections(docs["examples/graph.before.json"])
    ha = canon.projections(docs["examples/graph.after.json"])
    after_decls = {x["address"]: x for x in docs["examples/graph.after.json"]["declarations"]}
    for ref in wo["context"]["references"]:
        table = ha if ref["viewpoint"] == "desired" else hb
        if ref["target"] in table and table[ref["target"]][ref["projection"]] != ref["hash"]:
            failures.append(f"work-order: {ref['target']} {ref['projection']} hash differs from hashes.json")
    for s in wo["subjects"]:
        for r in s["requirements"]:
            target, member = r["subject"]["requirement"].split("#")
            coll, key = member.split(".", 1)
            want = canon.sha(after_decls[target]["members"][coll][key])
            if r["contract_hash"] != want:
                failures.append(f"work-order: contract hash of {r['subject']['requirement']} differs from the graph member")
    if [s["id"] for s in rv["subjects"]] != [s["id"] for s in ro["subjects"]]:
        failures.append("review-verdict: subjects must match the review order exactly")
    if wo["attempt"]["number"] > wo["attempt"]["max"]:
        failures.append("work-order: attempt.number exceeds attempt.max")
    for s in wo["subjects"]:
        for key in ("current", "previous"):
            text = s["spec"].get(key)
            if text:
                try:
                    parse(text)
                except ParseError as e:
                    failures.append(f"work-order: {s['address']} spec.{key} does not parse: {e}")
    rv_ok = validator("result")
    for name, good in GOOD_RESULTS.items():
        for err in rv_ok.iter_errors(good):
            failures.append(f"result schema rejected good case {name}: {err.message[:160]}")
    for name, bad in BAD_RESULTS.items():
        if rv_ok.is_valid(bad):
            failures.append(f"result schema accepted bad case: {name}")
    return failures, len(SCHEMA_EXAMPLES)


def main(argv):
    failures = []
    schema_failures, n_schemas = check_schemas()
    failures += schema_failures
    for name, src in NEGATIVE.items():
        try:
            parse(src)
            failures.append(f"negative case accepted: {name}")
        except ParseError:
            pass
    for name, src in POSITIVE.items():
        try:
            parse(src)
        except ParseError as e:
            failures.append(f"positive case rejected: {name}: {e}")
    total = {"spec": 0, "json": 0}
    import glob
    for path in sorted(glob.glob("examples/*.int")):
        total["spec"] += 1
        try:
            parse(open(path).read())
        except ParseError as e:
            failures.append(f"{path}: {e}")
    for path in argv or ["SPEC.md", "PROTOCOL.md", "EXTENSIONS.md"]:
        count, f = check_spec_md(path)
        failures += f
        for k in total:
            total[k] += count[k]
    print(f"checked {total['spec']} spec blocks, {total['json']} json blocks, "
          f"{len(NEGATIVE)} negative and {len(POSITIVE)} positive grammar cases, "
          f"{n_schemas} protocol examples, {len(GOOD_RESULTS)} good and {len(BAD_RESULTS)} bad result files")
    for f in failures:
        print("FAIL", f)
    print("OK" if not failures else f"{len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
