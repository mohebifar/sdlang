# Round 11 decisions: fixes from the scoring round

Inputs: `10-scoring.codex.md` (Codex's scores and "to reach 9" lists) and
`10-scoring.editor.md` (the editor's). Every concrete change either reviewer named was
accepted; none was contested, so there was no rebuttal.

| Area | Asked for | Change |
| --- | --- | --- |
| Architecture | Precise graph encoding: discriminated `$type`, signature vs members, set ordering, source not hashed | `graph.schema.json` uses a discriminated union for types and a `$select` value; set arrays sorted by canonical JSON; source never hashed; action `input`/`output` keys are also members. |
| Architecture | Complete graph fixtures with expected projection hashes and plans | `examples/graph.before.json` and `graph.after.json` (the whole worked example, closed: no dangling edges), `tools/canon.py` reference canonicaliser with SCC hashing, `examples/hashes.json`, `schemas/plan.schema.json` and `examples/plan.json`. The checker recomputes the hashes and derives the plan's create/update/obligation sets and ordering from the graphs. |
| Architecture | Implementation impact beyond "no projection change" | Any published change to outputs (update for any cause, adoption, regeneration) gives transitive ordering dependents `verify`. |
| Drift | Import-and-extraction walkthrough; agents cannot write unmanaged files | `acquire` strategy publishes ownership and baseline with no agent; walkthrough of a shared `src/lib/auth.ts` through integration owner to extraction, with envelope and failure behaviour. |
| Drift | "Affected evidence" on adoption | Defined in Verification → Currency, including frozen declarations. |
| Drift (editor) | Defined output for `absorb` | Spec patch from an agent without code write access, human review, and the plan must then show the declaration conforming. |
| Verification | Invalidation table across inputs and evidence kinds | Verification → Currency table. |
| Verification | Input set behind evidence digests; reuse after another transaction | Run input closure defined over ordering relations, stack and oracle; evidence stays current while its bound digests recompute equal; two-transaction example from the worked plan. |
| Verification (editor) | Measure coverage-review quality | Milestone seeds weak tests and measures reviewer rejection. |
| Grammar | SCC hashing vs type-cycle ban | Structural recursion through record fields, list elements or `ref` is legal; alias cycles are errors; examples given. |
| Grammar | Selector operator/type matrix | Table with field types, operands, missing-field and multi-kind rules. |
| Grammar | Checker accepted `select []` | Rejected; negative case added. |
| Agent interop | Review-order context contract | Review orders carry inline `inherited` and `references` like work orders. |
| Agent interop | Regression per repair subject; `not_automatable` in review | Result `regression` is an array keyed by subject; review regression is automated (receipts) or not_automatable (reason, artifacts). |
| Agent interop (editor) | Schemas for MCP tools and generator protocol | `schemas/mcp-tools.schema.json`, `schemas/generator.schema.json`, generator examples for tx-1, validated with recomputed bindings. |
| Packages | How adapters materialise core kinds and stack outputs; conflict rule | Package-only `implements <core kind>` form (grammar production added); at most one implementer per core kind. |
| Packages | Milestone adapter profile; static manifest schema | `examples/nextjs-adapter.spec` (parsed by the checker): page kind, `implements action`, `implements stack`, checks, generator, route mapping; `schemas/package-manifest.schema.json` with `examples/package.json`. |
| Blocking | Graph-first input contract incomplete; mentions exempted from dangling check | Fixtures are complete; the checker no longer exempts mentions. |
