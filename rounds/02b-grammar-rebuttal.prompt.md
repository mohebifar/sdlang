# Round 2b: grammar rebuttal

Context (read-only): `GRAMMAR.ebnf` (draft), `SPEC.md`, and your critique in
`rounds/02-grammar.codex.md`. Answer in Markdown, under 700 words.

The editor accepts nearly all of round 2. Three disagreements; for each, concede, hold or
refine, with a concrete reason.

## 1. Selector field access without a `field.` prefix

You proposed `field.route matches "/app/**"` and `meta.module != "marketing"` so user
fields named `name`, `module` or `kind` cannot collide with metadata. The editor proposes:
a bare identifier in a predicate is always a field of the candidate declaration; metadata
is only reachable as `meta.module`, `meta.name`, `meta.kind`; `meta` is a reserved word
inside predicates and kinds may not define a field called `meta`. That keeps the common
case short: `select nextjs.page where route matches "/app/**"`. Does that reintroduce an
ambiguity you were trying to remove?

## 2. Cut `this`, `root` and `builtin`; keep only `self`

The editor's reasoning: local bindings cannot shadow anything, so `this.x` never
disambiguates; prelude names cannot be shadowed, so `builtin.x` never disambiguates;
`root.auth.login` lets any module reach any module without an `import`, which bypasses the
import list that bounds an agent's context. Collisions are resolved by import aliases
(`import module auth as auth_pages`). `moved` uses a separate canonical-address category.
Is there a case the editor misses where one of these roots is necessary?

## 3. Keywords as field names

Your dedicated statement forms make `project`, `stack`, `module`, `type`, `kind`,
`annotation`, `let`, `select` and `null` reserved. `type` in particular is a common field
name (`type: "button"`). Editor's proposal: reserved words are reserved only in statement
and expression-head positions; any word, reserved or not, is allowed as a field name
(before `:`), as a section name, and as a path segment after `.`. Does this create a
parsing ambiguity anywhere in your proposed grammar (for example `Member ::= Name ':'` vs a
nested statement, or `meta.kind`)?
