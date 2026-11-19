# Round 2 decisions: grammar

Inputs: `02-grammar.codex.md`, rebuttal `02b-grammar-rebuttal.codex.md`. The editor drafted
`GRAMMAR.ebnf` first; Codex attacked the draft.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| 1 | Several examples fail the proposed rules (`to: @{login}`, `@default`, `check: tested`, list `expect`, flat kind def, unbound error names) | **Accept** | All examples migrated. Code blocks are tagged (`spec`, `json`, `console`, `text`, `mermaid`) so the checker knows what to parse. |
| 2 | Generic `Modifier* Path Ident? Body` accepts nonsense | **Accept** | Dedicated forms: `project`, `stack`, `module`, `type`, `let`, `kind`, `annotation`, and `extern? Path Ident Annotation* Block`. Placement rules (root-only, once per directory) are compilation-unit checks. |
| 3 | Entity columns collide with declaration metadata (`summary`, `lifecycle`) | **Accept** | Entity schema moves into a `fields { }` section, mirroring kind definitions. |
| 4 | Newline deletion is incomplete; preserve NL tokens | **Accept** | NL is always a token; productions accept it explicitly (`Gap`). Continuation only after openers, commas, `:`, `=`, `|`, `where`, `and`, `or`, `not`, comparison operators. Comments stop before the NL. |
| 5 | Left-factoring and trailing separators | **Accept** | Documented in the grammar; the checker's parser uses the factored forms. |
| 6 | String scanning must be a state machine; `@@{` had no production | **Accept** | Ordered scan (closer, literal opener, mention/interp, escape, text). Malformed `@{` is an error. Prose gets one escape. |
| 7 | Mechanical dedent | **Accept** | Exact rules adopted (block vs inline prose, closing line excluded, tabs rejected). |
| 8 | Interpretation matrix; `null`; `now` must be symbolic | **Accept** | Table in SPEC.md "Values and types". Defaults only in schema positions. |
| B | One binding table, no shadowing, collisions are errors, explicit `import module` / `import package` | **Accept** | Fixes REVIEW flaw 7. Resolution algorithm written into SPEC.md. |
| B' | Roots `this`, `root`, `builtin` | **Reject** (Codex conceded in 2b) | No shadowing means `this` and `builtin` never disambiguate; `root` bypasses the import list that bounds agent context. Only `self` stays. Child modules need an import too. |
| B'' | `moved` operands are canonical addresses | **Accept** | Separate `Address` production. |
| C | Annotations are declared, typed package exports; a core set | **Accept** | Core: `@id`, `@unique`, `@max_length`, `@interface`, `@relation`, `@deprecated`. `@max` renamed because it was overloaded. `@relation` added by the editor so kind fields name their relation. Declaration annotations allowed after the name. |
| D | `select` form, field/meta namespaces, bounded predicates, glob semantics, empty match is a warning | **Accept, refined in 2b** | Bare names are candidate fields; metadata only via `meta.` (Codex conceded the `field.` prefix). Kinds may not declare fields named after predicate keywords. |
| E | Stack versions as `{ name, version }` records | **Accept** | No splitting strings at `@`, since scoped npm names contain `@`. The lock holds exact resolutions. |
| F1 | Cut `enum` | **Accept** | `type Plan = "free" | "team"`. |
| F2 | Cut declaration templates from v0 | **Defer to round 3** | Round 3 owns template expansion addresses. The grammar keeps a provisional `template` / `expand` form for that round to attack. |
| F3 | `errors` as strings unless error declarations exist | **Accept** | `errors: ["NameTaken"]`. |
| F4 | Fetched docs cannot create typed exports | **Accept** | A capability without a spec package exposes only its own name. |
| 3 (2b) | Keywords as field names | **Accept, refined** | Keywords are reserved only where a name is bound or a path starts; any word is accepted elsewhere. |
