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

## Round 3: identity

- D3.1 [accept] Member addresses `decl#collection.key` over schema-declared addressable collections; never positional — REVIEW flaw 1.
- D3.2 [accept] `expect` is a section of named members with a short (`key: "text"`) and long (`key { text, verify, test }`) form; both canonicalise identically — REVIEW flaw 1.
- D3.3 [accept] `errors` is a named collection; wire code defaults to the key and can be pinned.
- D3.4 [accept] Evidence = coverage (reviewer judgement on a requirement revision) + run (pass on a tree); text edits invalidate coverage — an unchanged test could falsely claim a changed requirement.
- D3.5 [accept] `moved` at declaration, module (`moved module`) and member level, resolved as one injective mapping with the composition table in SPEC.md "Moves".
- D3.6 [accept] Entity field moves lower to `rename_field`; no silent drop-and-add.
- D3.7 [accept] `moved` statements are kept indefinitely and reserve their source addresses; no "safe to remove" claim.
- D3.8 [reject] Package and capability addresses in `moved` — export continuity belongs to package migration data; Codex conceded.
- D3.9 [accept] Rules carry named `expect` members; each application is an evidence subject `(target, requirement)`.
- D3.10 [accept] Live templates removed from v0; `spec scaffold` writes ordinary declarations instead — overrides and detachment would cost more than they save.
- D3.11 [accept] Output identity is the slot (`#outputs.page`); the path is a locator, so route edits move files.
- D3.12 [accept] Catalog symbols have canonical package identity independent of import alias; `extern` address is stable while its source locator changes.
- D3.13 [accept] `consumed` is a list of (relation, target, projection, hash).
- D3.14 [reject] Shared graph manifest for context declarations; instead `provenance.commit` locates the applied spec revision for change classification, with conservative fallback.
- D3.15 [accept] Verification policies with an explicit satisfaction table; `accepted` is not a test.
- D3.16 [defer] Identity epochs for pruning `moved` history; constraint and index naming (round 8).

## Round 4: plan semantics

- D4.1 [accept] `repair` is a cause of `update`, started by `spec repair`; requires a requirement-derived regression test that fails on the pre-fix tree; untestable bugs yield `judged` only — REVIEW flaw 2.
- D4.2 [accept] `--unspecified` repairs cut; a missing requirement is added as an `expect` member.
- D4.3 [accept] New `behaviour` projection; relations declare what they observe; observed changes give `verify` + `review` — REVIEW flaw 3.
- D4.4 [accept] Implementation-only changes propagate `verify` over ordering relations, checks deduplicated; `verify_scope: "all"` option.
- D4.5 [accept] Obligations `verify` (deterministic checks) and `review` (independent reviewer: conforms / does not / cannot determine) are independent and can both apply — REVIEW flaw 8 (meaning).
- D4.6 [accept] Context changes classified by meaning: normative prose → review; hint, skills, generators → none; checks → verify; conventions, providers → update.
- D4.7 [accept, refined] No forward-only flag for normative context; `hint` is the only forward-only input; plan prints review cost and batches with per-subject verdicts — REVIEW flaw 8 (cost).
- D4.8 [accept] Delete: co-deleted references allowed, old graph finds consumers, enumerated owned outputs, shared-file owner joins, ownership transfer required, explicit guarantee stated — REVIEW flaw 9.
- D4.9 [accept] Change vocabulary: create, update (strategies: edit, replace; causes include repair), delete, detach, none.
- D4.10 [accept] Partial progress checked against the applied graph; per-transaction atomicity.
- D4.11 [accept] Immutable plans with revisions, a receipt journal, crash detection via commit trailers; amendments rebind and recompute.
- D4.12 [accept] Generator output validated like agent output; failures classified; no silent agent fallback.
- D4.13 [accept] `spec plan --reconcile` after merges; runs on parent trees do not count for the merged tree.
- D4.14 [accept] Checked-input digest excludes state, evidence and journal files.

## Round 5: ownership

- D5.1 [accept] Ownership invariant: one owner per managed file; dependency, contribution and evidence confer no write authority; ownership changes publish atomically — REVIEW flaw 10.
- D5.2 [accept] Kinds declare output slots and creation roots; created outputs get agent-chosen stable keys; paths are locators; route changes move files and repair imports — REVIEW flaw 10.
- D5.3 [accept] A creation root grants no authority over existing files.
- D5.4 [accept] Ownership transfer is a cross-declaration member move of an output.
- D5.5 [accept] Whole-file ownership in v0; symbol ownership deferred until a language adapter mediates edits.
- D5.6 [accept] Contributions: owner accepts schema, ordering, exclusive keys, aggregate input including referenced projections, narrow import grants, owner-deletion rule, materialisation from the applied graph; `collects` selectors for implicit registries.
- D5.7 [accept] `stack` is the managed toolchain owner (manifest, lockfile, config); `use` bindings are managed when their package declares outputs — resolves "project owns nothing" contradiction.
- D5.8 [accept] Published migrations are retained artifacts: immutable, never deleted.
- D5.9 [accept] Exact byte digests for binding and evidence; formatter digest only classifies drift.
- D5.10 [accept] Drift blocks managed declarations, is reported for frozen; `repair` added as a drift resolution; hand-edited generated files are restored, not adopted.
- D5.11 [accept] `spec import` proposes an ownership mapping (integration owner, extern, or extraction); never competing claims.
- D5.12 [accept] Generator-definition changes regenerate the outputs that generator wrote.
