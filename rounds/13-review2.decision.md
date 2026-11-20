# Round 13 decisions: review of the second-review pass (R2.2 to R2.8)

Input: `13-review2.codex.md`. Codex scored the nine areas 7, 7, 7, 8, 7, 6, 5, 5, 6 and
judged findings 4 and 7 addressed, findings 1, 2, 3, 5 and 6 partly addressed. Every defect
it cited was checked against the files and is real. All are accepted, so no rebuttal round
was needed.

The guiding choice: most defects sit in mechanisms the R2 pass itself added (plan-cadence
checks, cost limits, coverage reconfirmation, the merge driver, the applied store). Where a
mechanism is not needed by the first milestone, it moves to `EXTENSIONS.md` with its open
gaps written down, instead of growing the core further. Where the milestone needs it, it is
completed.

| # | Defect | Decision | Change |
| --- | --- | --- | --- |
| 1 | Build commands cannot meet the JSON check ABI; the "read-only" working copy cannot host a build | accept | Build checks keep the exit-status contract (exit 0 passes). Only conformance and test checks use JSON. The working copy is disposable and writable; writes there are discarded. `build_result` leaves the schema. |
| 2 | A conformance check gets files but not the contract; rule inventory has no applicability | accept | Check input gains `graph`, the path of the bound canonical graph. Check definitions declare `rules { rule: [kinds] }`; the CLI passes only the rules that apply to the subject's kind. |
| 3 | The checker does not prove the inventory claim; the test example has no paired input | accept | The checker derives the inventory from the adapter file, compares exact `(subject, rule)` pairs and rejects extras. A paired `check.test-input.json` carries `expected_cases`, and the checker requires each in the result. Mutation cases (extra rule, missing case) must fail. Generated-case metadata is in the test result schema. |
| 4 | Plan-cadence publication, status and bisection are underspecified | accept, by moving | Plan-cadence checks move to `EXTENSIONS.md` with these gaps listed as open. The milestone runs build in every transaction. |
| 5 | Applied store: retention of inherited text, and store files inside checked inputs | accept | Store roots include `context_parts`; retention is transitive; all of `.spec/` is excluded from checked inputs; store changes publish with the transaction commit, after checks. |
| 6 | "Merge by any method" outruns reconciliation, which needs both parents | accept | The merge driver saves the base, current and incoming versions of each conflicted record under `.spec/reconcile/`, so `--reconcile` needs no parent commits. |
| 7 | `lib` has nowhere to put its tests; its contract limits are unstated | accept | `implements lib` gets a `tests` slot. Compatibility gains a `lib` bullet. The text says `uses` observes no behaviour, so a lib's behaviour change gives users `verify`, not `review`; Compatibility's "whatever their types say" is qualified to observing relations. |
| 8 | Reconfirmation and `--accept-cost` have no wire representation | accept, by moving | Cost limits and reconfirmation move to `EXTENSIONS.md`, marked as needing schema fields before use. The core keeps "the plan prints its cost". |
| 9 | Milestone contradictions; core kinds that are partly provisional; required `rollout`; stale references | accept | Fixed by 4. "One conformance rule" corrected. A note says which kinds the milestone uses and which semantics are provisional. `rollout` is optional in the plan schema. "Below" reference and the stale DECISIONS line fixed. A core-only example is recorded as an open item, not written. |

Not done this round: O.3 (evidence-transition example), O.4 (cross-module inherited-change
example), O.5 (toolchain constraints), and a core-only worked example. These are recorded as
open. No rescoring was run after these changes.
