# Round 3: identity gaps

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read `SPEC.md` (sections "Identity", "Evidence", "Surface syntax" → "Statements",
"State record", "Plan entries") and `GRAMMAR.ebnf` (`Moved`, `Address`, `TemplateDef`,
`Expansion`). REVIEW.md flaw 1 (positional `expect` ids) is the trigger for this round.

## Editor's draft positions (attack them)

1. **Sub-addresses.** Anything inside a declaration that state or evidence must track gets
   a sub-address `decl#section.key`: expect items `auth.login#expect.redirects`, entity
   fields `core.Workspace#fields.name`. The grammar currently has `Address ::= Word ('.'
   Word)* ('#' Word)?`; this would widen it to `'#' Word ('.' Word)*`.
2. **Named expectations.** `expect: { redirects: "..." }`. The key is the identity; the
   text is body. Editing the text keeps the test mapping but marks the item's evidence
   `stale` so the test is re-validated against the new text (by whom?). Adding or removing
   an item changes only that item.
3. **`moved` covers every level**:
   `moved auth.login -> auth.sign_in` (declaration),
   `moved auth -> account` (module prefix: every address beneath it),
   `moved auth.login#expect.redirects -> auth.login#expect.valid_redirect` (expect item),
   `moved core.Workspace#fields.name -> core.Workspace#fields.title` (entity field: the
   planner must emit a column rename, not drop-and-add).
   No rename inference: the planner may only *suggest* a `moved` when a deleted and a
   created address have near-identical canonical bodies.
4. **Lifetime of `moved`.** A `moved` statement is needed until every state record that
   uses the old address has been re-keyed. Since state is per branch, a long-lived branch
   may still carry the old address. Proposal: keep `moved` statements until removed by
   hand; the plan warns "moved source absent from state: safe to remove" but never fails.
5. **Rule-sourced requirements.** A rule's `require` attaches to every matched declaration
   and needs evidence per target. Proposed address: `dashboard.home#rule.app_requires_session`.
6. **Templates.** `template crud(entity: ref entity) { ... }` holds declarations with fixed
   local names. `expand crud workspace_pages(entity: core.Workspace)` creates a namespace:
   its members are addressed `<module>.workspace_pages.<local name>`. Changing an argument
   never changes an address. Editing the template marks every expansion's members with a
   body change. Not decided: overriding one member's fields, detaching one member to edit
   it by hand, and whether templates belong in v0 at all.

## Questions

A. Attack each position. Where does identity still break: renames that look like
   delete+create, collisions between sub-address sections, items whose identity
   should survive moving between declarations, moved chains (`a -> b`, later `b -> c`),
   cycles (`a -> b` and `b -> a`), a `moved` target that already exists, branch merges
   where one branch renamed and the other edited.
B. What else needs a stable identity that the draft forgets? Consider action inputs and
   errors, catalog declarations from packages, capabilities, generated outputs (files whose
   path is derived from a field like `route`), and `extern`.
C. Templates: give a concrete recommendation. If they stay, specify overrides and
   detachment with syntax. If they go, say what replaces the need (CRUD pages per entity).
D. Is `expect` as a record field right, or should it be a section (`expect { ... }`) or
   individual declarations? Consider expectations that need metadata such as a required
   verification level or a pointer to a test.

Give concrete syntax, grammar changes and state-record JSON where relevant. End with a
prioritised list of changes.
