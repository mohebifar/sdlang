# Decisions log

One line per decision. Format: `D<round>.<n> [accept|reject|defer] summary — reason`.
Full reasoning lives in `rounds/NN-*.decision.md`.

Note: every round uses Codex model `gpt-6.1-sol`. The requested `gpt-6.2-sol` is listed by
the CLI but rejected for this ChatGPT-account login; the user approved the substitution.

## Round 1: whole-design teardown

- D1.1 [accept] Unit of apply is a transaction: a changed declaration plus dependents whose consumed projection changed incompatibly — interface changes and callers cannot pass checks separately.
- D1.2 [accept] Transactions run serially on a candidate tree and publish only after checks pass on the full tree; evidence records tree id and check hash — disjoint write sets do not compose semantically.
- D1.3 [accept] Plan entries have five dimensions (identity, lifecycle, drift, change, obligation); drift blocks; address excludes kind; protected removal requires `detach` — the single-action table was ambiguous.
- D1.4 [accept] `plan.json` binds baseline commit, graph, lock, state digest and output fingerprints; new files only under `may_create_under` — a reviewed plan must bound what apply does.
- D1.5 [accept] Hashes are canonical projections (`public`, `body`, `context`); `summary` is public; extern records source hash; package components hashed separately; state records consumed hash per edge — hashes missed real inputs.
- D1.6 [accept] Edges carry relations that declare consumed projection, ordering and import grants — one reference graph was doing several jobs.
- D1.7 [accept] No capability-substitution promise; provider swap plans `update` on dependents — compatibility was undefined.
- D1.8 [accept] Baseline and evidence separated; `adopt` stales evidence; `import` starts unverified; new `accepted` and `stale` statuses — adoption kept untrue evidence.
- D1.9 [accept] Milestone gets independent acceptance tests, a control arm, and starts from graph JSON before the parser — old metrics could not detect a wrong change.
- D1.10 [accept] Allowed imports derive from relation-specific grants, no `libraries:` field; worked example completed — the example violated its own conformance rule.
- D1.11 [reject] Cutting the custom syntax — the reviewed artifact is the syntax; Codex conceded.
- D1.12 [reject] Demoting `action` to adapter vocabulary — it is the one transport-independent contract; Codex conceded. Refined: defined only by its callable contract.
- D1.13 [accept] `project`, `stack`, `use`, `module`, `rule`, `type`, `enum`, `extern` are context declarations with no state record.
- D1.14 [defer] Parallel apply deferred past v0; selectors/templates to rounds 2–3; contributions/symbol ownership to round 5.
- D1.15 [defer] Entities and runtime migration state to round 8; kept in the language, outside the milestone.
- D1.16 [accept] Large transactions for breaking changes are correct; document expand-migrate-contract staging.
- D1.17 [defer] Shared output owners joining transactions — round 5.

## Round 2: grammar

- D2.1 [accept] `GRAMMAR.ebnf` written; `tools/check_examples.py` parses every `spec` block and validates every `json` block in SPEC.md, plus negative cases — exit criterion and REVIEW flaw 4.
- D2.2 [accept] Dedicated statement forms replace the generic declaration production — the parser, not the schema, should reject `nextjs.page {}`.
- D2.3 [accept] Entity schema lives in a `fields { }` section — entity columns collided with declaration metadata.
- D2.4 [accept] Line breaks are preserved tokens with explicit continuation points; comma or line break separates items — REVIEW flaw 4 (separators, comments).
- D2.5 [accept] String scanning state machine, `@@{` and `$${` literal openers, one prose escape, mechanical dedent — REVIEW flaw 4 (escapes, `${}` versus `@{}`).
- D2.6 [accept] One expression grammar with a value/type interpretation matrix; `null`; `now` symbolic; defaults only in schema records.
- D2.7 [accept] One binding table per module, no shadowing, any collision is an error; `import module` and `import package` disambiguate — REVIEW flaw 4 (`@{}` resolution) and flaw 7.
- D2.8 [reject] `this`, `root`, `builtin` roots — they never disambiguate, and `root` bypasses imports; only `self` kept.
- D2.9 [accept] `moved` takes canonical addresses.
- D2.10 [accept] Annotations are declared and typed; core set `@id @unique @max_length @interface @relation @deprecated` — REVIEW flaw 4 (annotations).
- D2.11 [accept] Selectors: `select <kinds> where <predicate>`, bare names are fields, `meta.` for metadata, fixed glob semantics, no graph traversal; empty match warns — REVIEW flaw 4 (selector language).
- D2.12 [accept] Stack slots are `{ name, version }` records — REVIEW flaw 4 (string-encoded versions).
- D2.13 [accept] `enum` cut in favour of string-literal unions; enum-like field values are strings.
- D2.14 [accept] `errors` are string codes; free-text `use` roles cut; a capability without a spec package exposes only its name. `use ... { options }` is checked against the provider's options schema — REVIEW flaw 4 (`use` options).
- D2.15 [defer] Declaration templates — round 3.
- D2.16 [accept] Kind definitions split into `fields` (schema) and `outputs` / `checks` (metadata) sections — REVIEW flaw 5, syntax half; semantics in round 7.
