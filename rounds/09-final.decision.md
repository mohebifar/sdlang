# Round 9 decisions: final pass

Input: `09-final.codex.md`. No rebuttal: every finding was a concrete defect and all were
accepted.

## Part 1: frontend

| Point | Decision | Why |
| --- | --- | --- |
| Own syntax; no TypeScript frontend in v0; small tooling plan (one parser with spans and comments, highlighting, prose-preserving formatter, thin LSP, rename deferred until it can emit `moved`) | **Accept** | Identity, edges, attachment and visibility need a compiler regardless of spelling; an evaluated builder would break no-code planning. Open question closed; plan written into "Surface syntax". |

## Part 2: audit

| # | Defect | Fix |
| --- | --- | --- |
| 1 | `accepts`/`collects`/retained metadata both required and deferred; owner-instance constructor missing from the value matrix | Extension table in Kind definitions (`accepts`, `collects`, package-only `binding` form with `outputs`, `creates`, `retains`, `collects`, `generator`, `observer`, `options`), marked as not needed for the milestone; matrix row covers owner-instance constructors. |
| 2 | Own `hint` edit triggered `update` while inherited hints did not | New `guidance` projection (`hint`, `skills`) that creates no obligation. |
| 3 | Annotation placement contradicted itself | Field annotations in schema records including annotation `parameters`; declaration annotations after the name. |
| 4 | Review example reused pre-edit handles | Review handles are candidate-bound; checker now rejects reuse. |
| 5 | Context-subject ids could start with a digit | `#context.h_<hex>`. |
| 6 | Test case tags could not be moved | Tags restricted to identifier shape in `common.schema.json`. |
| 7 | Root-rule addresses included the filename | `app_requires_session`, `main_nav`. |
| 8 | Binding prose described fields review orders lack | Common binding vs work-order inputs vs review candidate, in PROTOCOL.md. |
| 9 | Inline projections: prose mandatory, schema optional | Schema now requires inline text; `inline`/`resource` removed from references. |
| 10 | Checker narrower than EBNF for selector operand lists | Grammar narrowed (`OperandList`); negative case added. |
| M1 | No canonical graph format or hashing rules | `schemas/graph.schema.json`, generated `examples/graph.json`, canonicalisation and SCC hashing rules in "Projections and hashes". |
| M2 | Package discovery and options schema | Manifest `name` resolves short imports through the lock; manifest `options` names the options type. |
| M3 | Annotation-definition metadata | `targets`, `projection`, `repeatable`, ordered `parameters`. |
| M4 | Test check selection | Check `purpose: "test"`; run id `<check>:<output member>`. |
| M5 | Generator protocol, binding extensions, backfill producer | Generator protocol in PROTOCOL.md; `binding` form; retained roots name their producer (generator or agent). |
| M6 | `test_first` location; summary derivation | Project or module header; explicit derivation order. |
| M7 | Coverage digest naming | Review-order uses `oracle`. |
| W | Worked example: missing `forgot_link` in the authored change; escalation outside tx-1's envelope; accounting | Revised `login` shown; contingent envelopes added to plan binding; plan output separates changes, coding dispatches, reviews, contingency and rollout. |
| R | Readability cuts | Evidence in Core model reduced to the baseline/evidence distinction; duplicate kind example removed from Outputs; Agent interface reduced to one paragraph pointing at PROTOCOL.md. |
| Flaw 4 | "Partially fixed": annotation metadata, provider options, contribution schemas | Now defined (M2, M3, #1). |
| Flaw 8 | Worked output omitted review accounting | Fixed in W. |
