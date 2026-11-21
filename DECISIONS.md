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

## Round 6: agent protocol

- D6.1 [accept] Four protocol documents with JSON Schemas (`work-order`, `result`, `review-order`, `review-verdict`) plus shared defs; generated worked examples; the checker validates them and recomputes bindings — exit criterion, REVIEW agent interop.
- D6.2 [accept] Every order has a unique dispatch id and a binding with a recomputable `order_hash`; replies echo it; digests are full and tagged.
- D6.3 [accept] Orders have `subjects[]` with per-subject envelopes so coordinated transactions keep owner attribution.
- D6.4 [accept] Typed `causes[]`; `repair` block required exactly when a cause is repair, with pre-fix snapshot.
- D6.5 [accept] Spec text rendered as `source/1` with a resolution map and a real unified diff.
- D6.6 [accept] Import grants list exports; requirements are subject pairs with contract hash, policy provenance and test sets.
- D6.7 [accept] Result status `submitted | blocked`; created-output claims (create/relocate/remove); discriminated regression and blocker unions with evidence.
- D6.8 [accept] Retries: classified failures; infrastructure reruns do not consume attempts; continue by default; restart only as an explicit recovery action.
- D6.9 [accept] Blockers are claims; resolution paths: spec change, plan amendment, environment recovery; unsupported blockers count as failed attempts.
- D6.10 [accept] Context: inline mandatory set never truncated; other required items fetched in full by resource; optional items packed by `context-pack/1`.
- D6.11 [accept] Review protocol: per-subject exact-once verdicts, three-valued conformance and coverage, regression assessment, compiler-issued context subjects.
- D6.12 [accept] MCP surface: `get_task`, `get_declaration`, `read_context`, `get_diagnostic`, `run_self_check` (execution class), `validate_result`; confidentiality and final-tree limits stated.
- D6.13 [accept] Project-level `verify` default; core default policy `judged`.

## Round 7: packages

- D7.1 [accept] Packages carry a static `intlang` manifest; importing runs no code and creates no declarations; catalog exports are types, constants and `extern`s.
- D7.2 [accept] Closed kind vocabulary: `fields` (schema), `outputs`, `creates`, `exports`, `ambient`, `checks`, `generator`, `skills`; unknown sections are errors; core fields cannot be redeclared — REVIEW flaw 5 (semantics half).
- D7.3 [accept] Compatibility without package code: variance for core action signatures, invariance for package `@interface` fields, entity dimensions in round 8, unknown = incompatible.
- D7.4 [accept] Relations fixed by core in v0; no custom relations.
- D7.5 [accept] Generators are exclusive producers attached to a kind; writes come only from the CLI envelope; determinism defined as a runner-supplied input closure.
- D7.6 [accept] `check` has `purpose`, `scope`, `run` (argv, no shell).
- D7.7 [accept] Execution policy: no package code during planning; checks and generators isolated without credentials or undeclared network; dependency installation is a `stack` operation.
- D7.8 [accept] `provides` alias map replaces a `capability` statement; no interchangeability.
- D7.9 [accept] `spec.lock` pins the executable closure and per-component digests; local packages pinned by snapshot.
- D7.10 [accept] Upgrades compare old and new interpretations; declarative migration maps preserve identity only; no inferred deletion.
- D7.11 [reject] Editor's generic `unit` kind (withdrawn) — it would hide missing conformance; cheap local kinds outside `spec/` replace it.
- D7.12 [accept] Bootstrapping: core semantics in the compiler, one narrow `nextjs` adapter validated by the milestone and a clean-project install, local kinds, bare capabilities with explicit grants — REVIEW package-ecosystem risk.

## Round 8: verification and runtime state

- D8.1 [accept] Verification is a set of evidence kinds with stated promises and non-promises, not a ladder.
- D8.2 [accept] Policies are conjunctions `{ tests, review, checks, approval }`; string shorthands expand; effective policy is a union — supersedes D3.15.
- D8.3 [accept] Status is outcome plus currency per evidence kind; summary labels are derived.
- D8.4 [accept] Test protocol: `{ output, case }` identity, case moves, parameter instances, oracle digests over the test-side closure, complete discovery, retained attempts, quarantine.
- D8.5 [accept] Optional test-first with a test envelope, protected oracle digests and reviewed oracle amendments.
- D8.6 [accept] Conformance checks report named rules (`satisfied`, `violated`, `unsupported`, `error`); packages owe only narrow rules — REVIEW verification burden.
- D8.7 [accept] Entity changes classified on reader, writer and storage dimensions with stated assumptions; `reads`/`writes` consume the matching dimension.
- D8.8 [accept] Backfills are explicit retained operations with manifests and data-validating completion checks.
- D8.9 [accept] `environment` declarations with expected resource identity; `spec observe` writes immutable observation receipts; unreachable is unknown — REVIEW flaw 6.
- D8.10 [accept] The CLI never runs migrations or deploys; `spec rollout` hands off prerequisites to an external runner; source transactions never wait for production — REVIEW flaw 6.
- D8.11 [accept] Secrets declared by name; allowlisted protocol metadata; versioned test profiles; separate observer credentials.
- D8.12 [accept] Cross-declaration requirements expressed as `expect` on the stating declaration; `flow` deferred.

## Editorial pass (between rounds 8 and 9)

- E.1 [accept] Consistency fixes across sections written in different rounds: relations are core-fixed (removed "packages may declare relations"), `enum` and `depends_on` removed from tables, `may_create_under` replaced by creation roots, state record shows the `behaviour` hash and the coverage/run evidence shape, policies stated as conjunctions throughout.
- E.2 [accept] Worked example: the `database` binding joins tx-1 (it owns the retained migration and the schema index); drift resolutions include `repair`; plan counts corrected.
- E.3 [accept] Agent protocol reference moved to `PROTOCOL.md`; SPEC.md keeps the load-bearing rules — readability.
- E.4 [accept] Resolved open questions removed from SPEC.md; taste decisions listed in `QUESTIONS.md` with provisional defaults.
- E.5 [accept] Milestone trial gains a compatible widening, a normative prose change, and a route move with created parts, per rounds 4, 5 and 7.

## Round 9: final pass

- D9.1 [accept] Own syntax; no TypeScript frontend in v0; minimal tooling plan; open question closed.
- D9.2 [accept] Canonical graph format (`schemas/graph.schema.json`) with canonicalisation and SCC hashing rules — the milestone runner consumes graph JSON.
- D9.3 [accept] `guidance` projection (`hint`, `skills`) creates no obligation, including for a declaration's own hint.
- D9.4 [accept] Extension sections `accepts`, `collects` and the package-only `binding` form, marked outside the milestone.
- D9.5 [accept] Package import names and `use` options types declared in the manifest — REVIEW flaw 4 (`use` options) completed.
- D9.6 [accept] Annotation definitions declare `targets`, `projection`, `repeatable`, ordered `parameters` — REVIEW flaw 4 (annotations) completed.
- D9.7 [accept] Check `purpose: "test"` with per-output run ids; generator protocol in PROTOCOL.md; retained roots name their producer.
- D9.8 [accept] Contingent envelopes for obligated declarations, reviewed with the plan, so escalation needs no new plan.
- D9.9 [accept] Protocol fixes: candidate-bound review handles, `#context.h_<hex>` subjects, identifier test tags, mandatory inline references, `oracle` digests, binding prose per order type.
- D9.10 [accept] Root-module addresses carry no filename (`app_requires_session`).
- D9.11 [accept] Selector operand lists narrowed in the grammar to match the parser.
- D9.12 [accept] Worked example shows the revised `login`, contingent escalation and full plan accounting; readability cuts applied.

## Round 10: independent scoring

- D10.1 Scores (editor / Codex): Architecture 8 / 7; Drift and brownfield 8 / 8; Verification 8.5 / 8; Grammar 9 / 8; Agent interop 8.5 / 8; Package ecosystem 7 / 7. Below the 9 bar; fixes taken into round 11.

## Round 11: scoring fixes

- D11.1 [accept] Graph schema tightened (discriminated types, `$select`, set ordering, no source in hashes, signature keys are members).
- D11.2 [accept] Complete worked-example graphs, reference canonicaliser `tools/canon.py` with SCC hashing, computed `hashes.json`, `plan.schema.json` and `plan.json`; the checker derives the plan's change set and ordering from the graphs.
- D11.3 [accept] Implementation impact covers every published output change, including adoption and regeneration.
- D11.4 [accept] Evidence currency: run input closures, an invalidation table across inputs and evidence kinds, reuse after publication by recomputed digests, adoption's affected evidence.
- D11.5 [accept] Brownfield `acquire` strategy and an import-to-extraction walkthrough; `absorb` output defined.
- D11.6 [accept] Structural type recursion legal, alias cycles errors; selector operator/type matrix; `select []` rejected — REVIEW flaw 4 completed.
- D11.7 [accept] Review orders carry inline normative context and references; regression evidence per repair subject; `not_automatable` reviews.
- D11.8 [accept] Schemas for MCP tools and the generator protocol, with validated generator examples.
- D11.9 [accept] `implements <core kind>` package form with a one-implementer rule; the milestone adapter written in the language (`examples/nextjs-adapter.int`); package manifest schema.
- D11.10 [accept] Milestone measures coverage-review quality with seeded weak tests.

## Round 11 rescore

- D11.11 Scores (editor / Codex): Architecture 9 / 8; Drift 9 / 8; Verification 9 / 8; Grammar 9 / 9; Agent interop 9 / 8; Package ecosystem 8.5 / 8. Codex found defects the editor missed; fixes taken into round 12.

## Round 12: rescoring fixes (last allowed round)

- D12.1 [accept] `behaviour` holds own normative prose only; inherited normative context lives in `context`; precedence table; reference planner and checker confirm an inherited-prompt edit yields review obligations, not updates — REVIEW flaw 8 completed.
- D12.2 [accept] Canonical byte contract (NFC, number forms) with checked hash vectors.
- D12.3 [accept] Plan fingerprints are observed bytes; drift blocks only its own declaration.
- D12.4 [accept] `acquire` publishes after compile, ownership and build checks; its requirements become obligations in the next plan.
- D12.5 [accept] Kind `inputs` for implicit framework files; project-scope checks bind to the whole tree.
- D12.6 [accept] Conformance rules required unless advisory; consequences of `violated`, `unsupported`, `error` defined.
- D12.7 [accept] Protocol example hashes derived from the canonicaliser and cross-checked; ambient grants match the adapter; cross-document rules table.
- D12.8 [accept] Milestone runs all four adapter checks.
- D12.9 [accept] Generator protocol covers deletion and relocation.
- D12.10 [accept] Core `route_path` substitution rule; v0 routes are static only.
- D12.11 [accept] `binding` vocabulary specified, `observer` definitions, `examples/drizzle-binding.int`.

## Final scores (round 12) and stop

The loop stopped at its 12-round limit. Final independent scores, editor / Codex:

| Area | v0 | Editor | Codex |
| --- | --- | --- | --- |
| Architecture | 7 | 9 | 8 |
| Drift and brownfield | 6 | 9 | 9 |
| Verification | 6 | 9 | 8 |
| Grammar and syntax | 5 | 9 | 9 |
| Agent interop | 5 | 9 | 8 |
| Package ecosystem | 4 | 9 | 8 |

Not every area reached 9 from both reviewers. Codex's remaining asks, carried forward as
open items for the next design round (none is applied):

- O.1 [resolved by R2.7 and R13.1, not rescored] A check ABI: schemas for check input (subject, output selection, snapshot, profile, working directory, package-relative resolution) and check result (inventory, per-rule results, failures), plus a declared rule inventory per conformance check so omitted rules are detectable and advisory rules have syntax.
- O.2 Obligation placement for compatible fan-in (two changed actions with one caller): merge or verify per transaction, represented in the plan schema.
- O.3 A complete evidence-transition example (implementation edit, oracle edit, unrelated publication) showing which evidence is reused or replaced.
- O.4 A cross-module caller/callee protocol example where only the callee's inherited prompt changes.
- O.5 Declarative toolchain constraints for an adapter (how planning rejects an unsupported combination without running package code) and a compact `spec.lock` example.
- O.6 [resolved by C.4] Cross-language numeric vectors for the canonical byte contract.

## Post-final corrections (not rescored)

Three defects Codex found in the reference artifacts during the final scoring were plain
bugs, so they were corrected after the last round. The scores above were given before
these corrections and were not revised.

- P.1 [fix] The checker's `order_hash` used a different canonical form from graph hashing (no NFC); both now use `canon.sha`.
- P.2 [fix] The reference canonicaliser read only a declaration's own module header; it now includes every ancestor module's prompt, and the checker tests a nested module.
- P.3 [fix] The work-order example rendered inherited context inside a `behaviour` reference; references now have a `context` projection (also in review orders and `get_declaration`), and the example carries `sign_in`'s inherited context separately with its own hash.
- P.4 [fix] The cross-document output-claim rule is split by operation (`create`, `relocate`, `remove`).

## REVIEW.md flaws: final disposition

| Flaw | Status | Where |
| --- | --- | --- |
| 1. Positional `expect` ids | Fixed | SPEC.md Identity, Moves, Named members (D3.1–D3.2) |
| 2. No bug-fix flow | Fixed | SPEC.md Repair (D4.1) |
| 3. Prose changes slip past propagation | Fixed | Projections (`behaviour`, `context`), Propagation precedence (D4.3, D12.1) |
| 4. Constructs used but undefined | Fixed | GRAMMAR.ebnf; SPEC.md Surface syntax, Annotations, Selectors, Packages (D2.x, D9.5–D9.6, D11.6) |
| 5. Kind definitions mix schema and metadata | Fixed | Packages → Kind definitions (D2.16, D7.2) |
| 6. Runtime state not modelled | Fixed (design); execution by external runners | Runtime state (D8.9–D8.10) |
| 7. Scope collisions | Fixed | Scoping and resolution (D2.7) |
| 8. `verify` fuzzy and costly | Fixed | Plan entries, Propagation, Verification, PROTOCOL.md Reviews (D4.5–D4.7, D12.1) |
| 9. `delete` cleanup hand-waved | Fixed | Delete (D4.8) |
| 10. Multi-file nodes unspecified | Fixed (whole-file ownership; symbol ownership deferred, D5.5) | Ownership and drift (D5.1–D5.2) |

## Author's answers to QUESTIONS.md (2026-11-18)

- A.1 [accept] All provisional defaults Q1–Q10 are kept as decisions, except the file extension.
- A.2 [change] Source files end in `.int` instead of `.spec`. The CLI stays `spec`, as do the `spec/` source directory, the `.spec/` state directory and `spec.lock`, which are named after the CLI rather than the extension. The two adapter examples were renamed to `examples/*.int`, and the worked graph's source paths changed with them, so the example hashes were regenerated. `.int` has no GitHub Linguist entry. Its known uses (Micro Focus COBOL intermediate code, Unreal Engine localisation, some game scripts) are outside this audience. Round files keep the old paths because they record what was said at the time.
- A.3 [change] The language is called int lang, written `intlang` in identifiers: protocol schema ids (`intlang.plan/1`), the package manifest key, the schema URLs and the example adapter's npm scope (`@intlang/nextjs`). The repository folder and the CLI name `spec` are unchanged.

## Second review (2026-11-19)

`REVIEW-2.md` re-evaluates v0.2 at about 7 out of 10 and adds three unscored areas
(simplicity, cost, team workflow). R2.1 fixed its finding 8; R2.2 to R2.8 addressed findings 1
to 7; round 13 reviewed that pass.

- R2.1 [fix] Seven document defects from finding 8, all in `SPEC.md`: the `context` row is back inside the projections table; the summary and goals say evidence names its inputs, not a tree; the package-only forms are listed in full; the `page` kind excerpt has the adapter's four ambient imports and four checks; `stack` owns `next.config.ts`; the open question about the name now asks only whether the CLI keeps `spec`; the manifest example matches `examples/package.json`, with `provides` shown for a `use`-bound package instead.
- R2.2 [change] Finding 1, simplicity. Provisional material moved to a new `EXTENSIONS.md`: rules and selectors, package annotation definitions, contributions, bindings and retained artifacts, runtime state, upgrades, test first and journeys, the tooling plan. `SPEC.md` opens with the ten ideas a reader must learn. The three overlapping propagation descriptions are one precedence table. The formatting-only drift digest is removed: reformatting is drift, and `adopt` accepts it. New rule: no new concept enters `SPEC.md` before the first milestone reports.
- R2.3 [change] Finding 2, team workflow. New section, Team workflow: a read-only `spec check` with named findings, for CI; two supported ways to run `spec apply` (locally or by a bot on the pull request); a `.spec/` merge driver that defers conflicts to `--reconcile`; `--rerun` to reproduce runs on a trusted machine, with the limits of evidence trust stated. Old canonical projections live in a committed, content-addressed `.spec/applied/` store, so squash merges and rebases lose nothing; the journal is history once a plan's branch merges.
- R2.4 [change] Finding 3, shared code. New core kind `lib` for helpers, hooks and shared types: its public projection is `summary` plus `exports`; it is reached with `uses`; adding an export is compatible, removing one is not. `path` claims an existing file, which replaces the local `acme.lib` kind in the adoption walk-through. The adapter gains `implements lib`.
- R2.5 [change] Finding 4, silent verification. Action `errors` members are requirements. Declarations with no requirements are reported `unspecified` by `spec plan` and `spec check` (`--strict` fails on them). The compiler warns on hints that read as requirements. `tested` is described as "tests passed and a model judged them sufficient" until the milestone measures the reviewer. Worked example: the `auth` module header is shown; the two hints that read as requirements were reworded; the plan's coverage reviews include `request_reset#errors.rate_limited`. Correction to the review: the "never reveal" requirement was not missing, because the `auth` module prompt states it and inherited normative text is reviewed; the hint was misfiled, not the only copy.
- R2.6 [change] Finding 5, cost. Checks declare `cadence`: `"plan"` checks (the adapter's `next build`) run once after the last transaction, with bisection over journaled commits on failure. Projects may set `limits` on coding dispatches, review subjects and check runs; a plan over a limit needs `--accept-cost`. Coverage made stale by one coarse oracle change is reconfirmed in one batched review.
- R2.7 [change] Finding 6, adapter cost; resolves O.1. New `schemas/check.schema.json` (input, build, conformance and test results) with three examples, validated by the checker, which also checks that a conformance result reports its declared rule inventory exactly. Conformance checks declare `rules` and `advisory`. Ownership, envelopes and TypeScript imports are enforced by the core, not by adapters. Bootstrapping lists what an adapter must provide.
- R2.8 [change] Finding 7, milestone. Two control arms: plain instructions, and a strong arm (the agent with the repository, its tests and a Markdown spec per feature). Cost joins the measures. A stop rule fixed before the trial: if the language does not beat the strong arm by a margin set in advance, it stops.

## Round 13: review of the second-review pass

Codex (`rounds/13-review2.codex.md`) scored the nine areas 7 (architecture), 7 (drift and
brownfield), 7 (verification), 8 (grammar), 7 (agent interop), 6 (packages), 5 (simplicity),
5 (cost) and 6 (team workflow). It judged findings 4 and 7 addressed and 1, 2, 3, 5 and 6
partly addressed, and found nine defects, mostly in mechanisms the R2 pass added. All were
accepted (`rounds/13-review2.decision.md`). Not rescored after these changes.

- R13.1 [fix] Check protocol: build checks keep the exit-status contract and leave the schema; the working copy is disposable and writable; the input carries the bound graph's path; conformance rules declare the kinds they apply to (`rules { rule: [kinds] }`) and the input carries only the applicable ones; a paired test input carries `expected_cases`; generated-case metadata is in the test result. The checker derives the rule inventory from the adapter, compares exact `(subject, rule)` pairs, pairs the test input and result, and fails if three mutations (an extra subject, an invented rule, a dropped case) are not caught.
- R13.2 [move] Plan-cadence checks, cost limits and coverage reconfirmation moved to `EXTENSIONS.md`, Cost controls, each with its open gaps (deferred-check receipts and statuses, non-monotonic bisection, no schema field for cost acceptance or an `unchanged` verdict). The core keeps the plan's printed cost. The milestone and the worked example run build in every transaction, which removes the milestone contradiction.
- R13.3 [fix] Applied store: roots include `context_parts`, retention is transitive, store changes publish with the transaction commit, and everything under `.spec/` is excluded from checked inputs.
- R13.4 [fix] Merge driver: saves base, current and incoming versions of each conflicted record under `.spec/reconcile/`, during a merge, rebase or cherry-pick, so `--reconcile` needs no parent commits.
- R13.5 [fix] `lib`: the adapter gives it a test slot; Compatibility has a `lib` bullet; `uses` observes no behaviour, so a lib's behaviour change gives users `verify`, not `review`, and the "whatever their types say" rule is qualified to observing relations.
- R13.6 [fix] The milestone lists its kinds; `entity`, `rule` and `environment` are marked as partly provisional; `rollout` is optional in the plan schema; a stale "below" and the stale R2 status line are corrected; the milestone's conformance rule count matches the adapter.
- O.7 [open] A core-only worked example, using only the kinds the first milestone needs, with its own graphs and plan fixture.

## Round 14: independent rescoring

Codex (`rounds/14-rescore.codex.md`) scored 8 (architecture), 7 (drift and brownfield),
7 (verification), 8 (grammar), 8 (agent interop), 6 (packages), 6 (simplicity), 5 (cost)
and 7 (team workflow). It judged R13.2 and R13.6 fixed and R13.1, R13.3, R13.4 and R13.5
partly fixed, found three contradictions and four gaps in the check contract, and advised
running the trial after a focused cleanup rather than more broad rounds. All were checked
against the files and accepted (`rounds/14-rescore.decision.md`). Not rescored after these
changes. Its full checker run failed for lack of `jsonschema`; the editor's run, with it,
passes.

- R14.1 [fix] Checked inputs are defined once, in Currency: no closure includes anything under `.spec/`. Apply refers to it.
- R14.2 [fix] Reconciliation reads the merged spec, the applied store and the saved base, current and incoming record versions, and no commit. The plan schema's `binding.reconcile` lists those versions by digest instead of `parents` and `base`. A source moved to two targets is a compile error, which covers clean textual merges that clash.
- R14.3 [fix] `acquire` claims only existing files; a required slot with no file is left empty and the next plan schedules an `update` that writes it. The `lib` walk-through says its test file comes from that update.
- R14.4 [fix] Check contract: the input's graph is the applied graph; a test input carries expected cases with instance keys and the CLI-computed run-input digest, which the result echoes; the inventory digest has a defined form, checked when selection is complete; the schema requires purpose-specific fields and ties subject and outputs to scope. The milestone adapter defines its two conformance rules. The checker catches three more mutations (dropped instance, duplicate case, changed profile) and five bad inputs.
- O.2 [resolved by T.2] Placement of compatible fan-in obligations.
- O.8 [resolved by T.1] Pin the trial: toolchain and profile, the ten edits, the independent acceptance tests and the success margin.

## Trial pinned (editor, not reviewed)

- T.1 [new] `TRIAL.md` fixes the first trial before any run: toolchain majors and profile (exact versions come from the baseline lock and are logged), one coding agent and model for every arm, three runs per arm, two retries, a 45-minute cap per edit; a core-only baseline app (a task tracker with two modules, two libs, four actions and three pages), adopted by `acquire`; ten edits with expected plans, the files each concerns and the traps; 28 acceptance tests (UI and contract), written independently and run by the operator, never by the runner; a weak-test probe of 45 coverage judgements; blind code review; and a numeric stop rule.
- T.2 [rule] Compatible fan-in (O.2): obligations do not merge transactions; a caller with `verify` from two transactions is verified in each, against that transaction's applied graph. E5 measures the cost.
- T.3 [change] The stop rule no longer counts out-of-scope writes: the language arm's envelope refuses them by construction, so "retained behaviour or out-of-scope writes" let the language win by definition. Regressions decide, with intended behaviour no worse, a minimum strong-arm regression count (4) so the comparison can show a difference, and cost bounds (tokens 2×, human minutes 1.5×).
- T.4 [fix] The milestone's test check runs the project's own tests; acceptance tests stay outside the runner, so no arm gets feedback from them. `action` lists `uses` among its fields, as the `lib` section already assumed.
- O.7 [partly resolved by T.1] `TRIAL.md` has a core-only baseline spec that parses; its graph JSON is written at trial setup, not here.

## CLI implementation (editor, at the user's request)

- C.1 [change of order] The user asked for the CLI to be built now, in Rust (logos, clap), ahead of the trial that `SPEC.md` puts first. The parser is the first phase; the runner the trial needs follows as phases 4 and 5 (`cli/README.md`).
- C.2 [dogfood] The CLI's own plan is written in int lang (`cli/spec/`), with a local package `cli/spec-packages/rust/` that maps modules to crates, `lib` to Rust module files and a `command` kind to clap subcommands. It is the first spec outside the web stack; it needed no new language concept.
- C.3 [built] Phase 1: `crates/syntax` (logos lexer, string scanner with prose dedent, syntax tree with spans and JSON output, recursive-descent parser, codespan diagnostics) and `crates/cli` (`spec parse [--json] <paths>`). It accepts every spec block in the design documents and every `.int` file in the repository, and agrees with `tools/check_examples.py` on its 23 grammar cases; a test keeps the two case tables in step. 38 tests; clippy clean with warnings denied.
- C.4 [rule] Canonical numbers and keys, pinned (resolves O.6): integers and integral floats in exact plain decimal; other numbers as JavaScript's `Number.prototype.toString` writes them (chosen because adapters run on Node); keys sorted by Unicode code point, not UTF-16; keys equal after NFC are an error. "Shortest round-trip form" had let Python (`1e-05`), Rust and JavaScript (`0.00001`) disagree. `tools/canon.py` now writes this form, `examples/hash-vectors.json` gains nine vectors, and `examples/hashes.json` is unchanged (the graphs hold no floats).
- C.5 [built] Phase 2: `crates/canon` (canonical JSON, digests, the five projection hashes with type references and recursive components) and `spec hash [--canonical | --graph]`. It reproduces all 15 vectors and every projection hash in `examples/hashes.json` for both worked graphs, and agrees byte for byte with `tools/canon.py` on 4,000 generated numbers. The interface table was still the reference's fixed `KIND_INTERFACE` (superseded by C.10). 58 tests across the workspace; clippy clean.
- C.6 [built] Phase 3: `crates/graph` (loader, scopes and path resolution, core and package kind schemas, selectors, lowering) and `spec graph [dir]`. The worked example now exists as source (`examples/worked/before`, `examples/worked/after`, with packages under `examples/worked/packages`), and lowers to `examples/graph.before.json` and `graph.after.json` exactly, except the lock digest, whose input format is open (O.5); the lowered graphs reproduce `examples/hashes.json`. 80 tests across the workspace; clippy clean.
- C.7 [fixture] The worked graphs' `source` lines now match the source files (several were off by one or defaulted to 1), and modules are sorted by path. Neither is hashed; `examples/hashes.json` is unchanged; `generator.input.json` changed only in the source lines it embeds and its order hash.
- C.8 [choices, phase 3 only, not yet in SPEC.md] A stand-in `spec.lock` (`{ packages: [{ id, path }] }`) until O.5; the graph's `lock` is the digest of that document. A local package's id is `local:<path>`. Example packages invent catalog conventions SPEC.md leaves open: runtime exports as `extern` declarations of the package's own kind or a core kind, manifest skills as referenceable symbols, and `provides` as the only way through a capability. `type` declarations lower as `core:type` with `fields.type`; `let`s are inlined and do not appear in the graph. A selector's `where` is written with single spaces and every nested group parenthesised.
- C.9 [dogfood] Lowering the CLI's own spec (`cli/spec`) found four real errors in it, all now fixed: two declarations named with reserved words (`project`, `select`); capabilities named `json` and `diagnostics`, colliding with a builtin type and a lib; and child modules using `rust.crate` without importing the package. Two usability notes for the language: capability names share one namespace with everything else, so ordinary library names collide easily; and because imports do not flow into child modules, a project-local package must be imported in every module.
- O.9 [resolved by C.10] `intlang-canon` took interface fields from the fixed `KIND_INTERFACE` table instead of the kind schemas phase 3 reads.
- C.10 [schema] The graph gains a required `kinds` table: for every kind a declaration uses, its interface fields (a package kind's `@interface` fields; the core kinds' fixed lists), or `"all"` for `stack` and `use`. Hashing now reads nothing outside the graph, so `spec hash --graph` works on any graph, including one using package kinds the hasher has never seen; before, a conforming hasher needed the package's kind definitions or a hard-coded table, which the conformance contract ("hashes from the graph") did not allow. `tools/canon.py` and `intlang-canon` both drop their tables; the Rust frontend writes `kinds` from the schemas it loads, and `tools/worked_graph.py` writes it for the fixtures. `"all"` also fixes a latent gap: `SPEC.md` says all of a stack's fields are interface, while the old table listed four. `examples/hashes.json` is unchanged; the checker requires `kinds` to list exactly the kinds declarations use.
- C.11 [built] Phase 4: `crates/plan` (materialisation of output slots and generators, state records and the applied store, compatibility, the planner) and `spec plan [dir] [--id] [--out | --json]`. The worked example is planned from source: a baseline recorded from `examples/worked/before`, the signup page edited by hand, then `examples/worked/after` planned against it reproduces `examples/plan.json`'s entries, transactions, blocked list and coverage reviews exactly, and the plan validates against `schemas/plan.schema.json`; only the digests differ, because the fixture's are labels. Separate tests cover a module-prompt edit (review and verify only), an incompatible signature change, observed behaviour, delete, move, frozen, an empty slot, a guidance-only edit and greenfield. `canon` now exposes the projection documents, which the applied store holds. The canonical-graph and canonical-JSON work of phases 2 and 3 is unchanged.
- C.12 [choices, phase 4 only, not yet in SPEC.md]
  - **State.** `consumed` records `public` for every ordering relation and `collects`, and `behaviour` and `context` as well for `calls`, `reads` and `writes`. `mentions` are not recorded, since they neither order nor observe. There is no `guidance` hash, as in SPEC.md's example. The applied store is `.spec/applied/<hex>.json`, holding canonical JSON, and is checked against its name when read. `record_baseline` writes records for the current graph with the files on disk as accepted bytes. It is a library function only, until `apply` and `import` publish through it.
  - **Binding.** `graph_before` is the digest of every record's address and hashes. The planner never reads an old graph. `state_digest` covers each record file's path and exact digest. `fingerprints` holds the owned outputs of every entry, plus drifted files. Kind `inputs` are not yet included.
  - **Order.** SPEC.md orders transactions only by ordering edges. Ties go to data (entities and uses), then logic (types, actions and libs), then everything else, then address. Members of a transaction follow the same key. This is the rule that reproduces the fixture's numbering.
  - **Obligations.** An obligation attaches to every transaction whose change caused it, and the entry names the first. Obligations with no changed source (inherited context, or an observed target whose own context changed) share one obligation-only transaction, which runs last. An implementation-impact `verify` is dropped when the same source already gives one.
  - **Other.** A pure move is a transaction with change `none`, so its re-keyed record publishes. A changed or empty slot path is an `update` with cause `convention`. A blocked declaration gets no entry and propagates nothing. A delete has strategy `none`. A changed `summary` or annotation is incompatible, because prose cannot be compared.
  - **Not yet planned:** `rollout`, `reviews.conformance`, contributions, created-output moves, check-definition, generator, provider and option changes, stale evidence, repair, reconcile, and `detach` (see O.11).
- O.10 [open] SPEC.md does not say how transactions with no ordering edge between them are ordered, or where obligation-only work goes. C.12 picks rules that reproduce `examples/plan.json`; SPEC.md should state them or say the order is free.
- O.11 [open] `detach` of a `protected` declaration needs the lifecycle of a declaration that is gone from the new graph, and the state record does not carry it. Either the record gains `lifecycle`, or removal of a protected declaration needs an explicit request.

