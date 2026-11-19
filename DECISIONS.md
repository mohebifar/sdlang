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
