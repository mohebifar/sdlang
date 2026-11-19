#!/usr/bin/env python3
"""Throwaway checker for GRAMMAR.ebnf.

A hand-written lexer and recursive-descent parser that mirrors GRAMMAR.ebnf
production by production. It checks that:

  * every ```spec block in SPEC.md parses as a complete File;
  * every ```json block in SPEC.md is valid JSON;
  * a set of built-in negative cases is rejected.

Usage: python3 tools/check_examples.py [SPEC.md ...]
Exit status is non-zero if anything fails.
"""
import json
import re
import sys

KEYWORDS = {
    "import", "export", "extern", "use", "for", "as", "moved", "let", "type",
    "kind", "annotation", "template", "expand", "project", "stack", "module",
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
                ok = WORD_RE.fullmatch(inner) is not None
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
            self.take(); self.address(); self.expectp("->"); self.address()
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
            self.take(); self.word()

    def exportable(self, in_template=False):
        if self.atkw("let"):
            self.take(); self.ident(); self.expectp("="); self.gap(); self.expr()
        elif not in_template and self.atkw("type"):
            self.take(); self.ident(); self.expectp("="); self.gap(); self.expr()
        elif not in_template and self.atkw("kind"):
            self.take(); self.ident(); self.block()
        elif not in_template and self.atkw("annotation"):
            self.take(); self.ident(); self.block()
        elif not in_template and self.atkw("template"):
            self.template_def()
        elif not in_template and self.atkw("expand"):
            self.take(); self.path(); self.ident()
            self.seq("(", ")", self.arg)
        else:
            self.named_decl()

    def named_decl(self):
        if self.atkw("extern"):
            self.take()
        self.path(); self.ident()
        while self.atp("@"):
            self.annotation()
        self.block()

    def template_def(self):
        self.expectkw("template"); self.ident()
        self.expectp("("); self.gap()
        if not self.atp(")"):
            while True:
                self.ident(); self.expectp(":"); self.gap(); self.expr()
                if self.atp(","):
                    self.take(); self.gap()
                    if self.atp(")"):
                        break
                    continue
                break
        self.gap(); self.expectp(")")
        self.expectp("{"); self.gap()
        while not self.atp("}"):
            if self.atkw("export"):
                self.take()
            self.exportable(in_template=True)
            if self.atp("}"):
                break
            if not self.at("NL"):
                self.err("expected a line break between template statements")
            self.gap()
        self.expectp("}")

    def arg(self):
        self.word(); self.expectp(":"); self.gap(); self.expr()

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


def main(argv):
    failures = []
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
    for path in argv or ["SPEC.md"]:
        count, f = check_spec_md(path)
        failures += f
        for k in total:
            total[k] += count[k]
    print(f"checked {total['spec']} spec blocks, {total['json']} json blocks, "
          f"{len(NEGATIVE)} negative and {len(POSITIVE)} positive grammar cases")
    for f in failures:
        print("FAIL", f)
    print("OK" if not failures else f"{len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
