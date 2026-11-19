# Round 8 decisions: verification and runtime state

Input: `08-verification.codex.md`. No rebuttal: every point accepted. The biggest change
is conceptual: verification kinds are not a ladder, and source completion is not
deployment readiness.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| 1 | Test identity needs qualified outputs, explicit case moves, parameter instances, complete discovery, retained attempts, oracle digests covering helpers, fixtures and snapshots | **Accept** | "Tests" subsection. A per-case result protocol example added. Implementation changes require reruns; oracle changes require new coverage. |
| 2 | Test-first limits who edits the oracle but proves nothing about its correctness; needs a test envelope and an oracle-amendment path; applies to changed revisions too | **Accept** | Optional `test_first`; oracle amendments are reviewed exact patches; honest initial outcomes. |
| 3 | `checked` as a rank is unsound; keep conformance as narrow, named rule results with `unsupported` | **Accept** | Package authors are asked only for narrow rules (ownership, imports, schema comparison), answering REVIEW's "heavy burden" concern. |
| B | Verification table should state promises and non-promises; replace the strength ladder with conjunctive policies; separate outcome from currency | **Accept** | `verify` long form `{ tests, review, checks, approval }`; strings are shorthand; effective policy is a union. Schemas updated (`policy` object). Supersedes D3.15's satisfaction table. |
| 4 | Entity compatibility must be directional with stated assumptions; storage feasibility separate from operational cost; propagation by reader/writer relation | **Accept** | "Entity changes" with a JSON classification; `reads`/`writes` consume the matching dimension. |
| 5 | Backfills need an executable operation manifest, resumability, completion validation, environment-side progress | **Accept** | `@database.backfill("id")` references a retained manifest. |
| 6 | Env var names are not resource identities; observations go stale and cannot gate deploys; separate receipts bound into plan revisions; unreachable = unknown | **Accept** | `environment` core context kind with `expected_resource`; `spec observe` writes immutable receipts. |
| C | Generation/observation boundary is right if the tool publishes rollout prerequisites rather than claiming safety; external runner revalidates | **Accept** | `spec rollout --target` handoff. Source transactions never wait for production. Worked example now states the rollout prerequisite and "readiness unknown". Closes REVIEW flaw 6. |
| 7 | "Values never appear" is unenforceable by naming; versioned profiles; separate observer credentials | **Accept** | |
| D | Cross-declaration requirements now via `expect` on the stating declaration; defer `flow` | **Accept** | |
