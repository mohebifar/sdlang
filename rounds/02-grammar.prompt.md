# Round 2: grammar

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read:

- `SPEC.md` (now v0.1), especially "Surface syntax", "Projects, files and modules",
  "Definition kinds", "Packages" and every code example.
- `REVIEW.md` flaw 4 (constructs used but never defined) and flaw 7 (scope collisions).
- `GRAMMAR.ebnf`, the editor's first draft grammar. It is a draft for you to attack, not
  a settled decision.

Key choices in the draft, all open to challenge:

1. **One expression grammar** for values, type expressions and selectors; the kind's
   field schema decides how an expression is interpreted ("parse uniformly, check by
   schema").
2. **Field syntax** `name: expr (= default)? @annotation*`. `@default(...)` is dropped in
   favour of `= expr`, with `now` a core constant.
3. **Annotations** are `@name` or `@name(args)` after a field. Not yet decided: where they
   are defined, how packages add them, and which are core.
4. **Selectors** are `expr where predicate`, with `==`, `!=`, `matches` (glob), `in`,
   `and`, `or`, `not` over a declaration's fields plus pseudo-fields `module`, `name`,
   `kind`.
5. **`@{path}` appears only inside strings.** In value position a reference is a bare path.
   `@@{` escapes it. `${name}` interpolation is legal only in `template`-typed fields of
   package kind definitions.
6. **Enum-like values are string literals**: `check: "tested"`, `lifecycle: "frozen"`.
   The `enum` kind is cut in favour of `type Plan = "free" | "team"`.
7. **Free-text `use` roles are cut**: `use "pdf-lib" for pdf` always binds an identifier.
8. **Separators**: newline or comma, with newlines dropped after openers and binary
   operators.
9. **Module header** is a declaration; `stack { }` and `module { }` take no name.
10. **Kind definitions** use sections: `export kind page { fields { ... } outputs { ... } }`
    to separate schema from metadata (REVIEW.md flaw 5).
11. **Named expectations** are a record: `expect: { redirects: "..." }`. (Identity details
    such as renames are round 3; only the syntax matters here.)

## Questions

A. **Grammar defects.** Find ambiguities, places the draft cannot parse an example in
   SPEC.md, LL(1)/lookahead problems, and lexical traps (the newline rule, prose
   dedent, `@{` inside prose that legitimately contains `@`, `|` in prose, etc.).

B. **Scoping and reference resolution.** Specify an exact resolution algorithm for a path
   such as `a.b.c` given: local declarations and `let`s, local module imports, packages,
   capability names bound by `use`, core kinds and builtin types, nested submodules, and
   self-reference. Address REVIEW.md flaw 7 (a capability `auth` versus a module `auth`).
   Say what is an error and what shadows.

C. **Annotations.** Where are they declared, how do packages add them, how are they
   type-checked, and which belong in core? Is there a case for annotations on
   declarations, not only fields?

D. **The selector language.** Is `expr where predicate` enough for real rules? What
   predicates are missing, and what must be forbidden so rule attachment stays cheap and
   deterministic?

E. **Versions in `stack`.** `"nextjs@15"` is currently a string. Propose a concrete form.

F. **Anything to cut or simplify** in the surface syntax.

For each point give a concrete fix with corrected EBNF or example syntax. End with a
list of the changes you would make to `GRAMMAR.ebnf`, in priority order.
