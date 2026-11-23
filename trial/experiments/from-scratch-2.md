# From-scratch experiment 2 (2026-11-21): the language arm after C.26

This reruns experiment 1's language arm (`from-scratch-1.md`) with the relaxed CLI from
`docs/decisions.md` C.26:

- a subject's files are a starting point, not a limit, and only the spec, adopted files and
  frozen files are protected;
- edges no longer restrict imports;
- the result file is optional, and cases tagged with a requirement's key cover it;
- each transaction gets one dispatch;
- the adapter's skill is now in every work order.

The setup, model, edits and acceptance tests are experiment 1's. The Markdown arm's harness
did not change, so its experiment 1 run is the comparison. Runs are in `~/intlang-runs/exp4/`.

One rerun was void (`exp3-void`). The skill was attached only to pages, so the action work
orders never said an action's entry point is its `default` export. The Markdown arm's
conventions state this. The rerun's build used named exports and failed the contract test B5.
The skill is now attached to the action and lib kinds too. Experiment 1's language arm got
this right by chance.

## Results

| | Language, exp 1 | Language, exp 2 | Markdown, exp 1 |
| --- | --- | --- | --- |
| Build: B1–B7 | 7/7 | 7/7 | 7/7 |
| Build: cost, wall time | $5.98, 651 s | $3.43, 410 s | $0.67, 134 s |
| Edits intended | 9/10 | **10/10** | 7/10 |
| Regressions | 0 | 0 | 0 |
| Acceptance after E10 | 22/22 | 22/22 | 20/22 |
| Edits: cost | $16.58 | $12.00 | $2.52 |
| Edits: tokens in, out | 22.9M, 177k | 14.3M, 124k | 3.4M, 27k |
| Edits: wall time | 1,679 s | 1,330 s | 396 s |
| Edits: sessions (agent + reviewer) | 24 + 16 | 21 + 19 | 10 + 0 |
| Total cost | $22.56 | $15.43 | $3.20 |

Per edit (✓ intended; cost in dollars):

| | E1 | E2 | E3 | E4 | E5 | E6 | E7 | E8 | E9 | E10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Language, exp 1 | ✓ 2.17 | ✓ 0.82 | ✓ 1.49 | ✓ 0.66 | ✓ 2.08 | ✓ 3.61 | ✓ 3.29 | ✗ 0.46 | ✓ 0.82 | ✓ 1.18 |
| Language, exp 2 | ✓ 2.14 | ✓ 0.55 | ✓ 2.16 | ✓ 1.19 | ✓ 1.24 | ✓ 2.36 | ✓ 0.67 | ✓ 0.39 | ✓ 0.70 | ✓ 0.60 |
| Markdown, exp 1 | ✓ 0.25 | ✓ 0.18 | ✗ 0.47 | ✓ 0.18 | ✓ 0.27 | ✗ 0.29 | ✓ 0.26 | ✗ 0.24 | ✓ 0.21 | ✓ 0.18 |

- **E7 fell from $3.29 to $0.67.** Its three pages went to one agent, with no limits for
  dispatches to trip over.
- **E8 now passes.** The page stayed byte-identical to the hand patch; the agent changed only
  the page's test.
- **The agents' reports changed.** They no longer mention envelopes, validation loops or
  requirement names. Every build agent skipped the result file because its case names matched
  the keys. At E7, the agent fixed a test that still imported the deleted page; in
  experiment 1, the agent left such files alone.
- **Reviews are now the larger cost.** Of the edits' $12.00, agents cost $5.67 and reviewers
  $6.33. The language agent alone costs 2.2 times the Markdown agent, against 6.6 times for
  the language arm as a whole in experiment 1. E1, E3, E4 and E6 each had one or two retries.

Against `docs/trial.md`'s stop rule, informally: intended behaviour holds (10 against 7), and
regressions are trivially half (0 and 0). Rule 3 still fails, because the Markdown arm
regressed nothing. Rule 4 still fails: the language arm used 4.2 times the input tokens
(14.3M against 3.4M) and cost 4.8 times as much.

## What this shows and does not

- The trimming cut a third of the cost (total $22.56 to $15.43; edit tokens down 38%) and
  lost no correctness.
- Each arm has one run. Some per-edit costs went up (E3, E4), so a single run cannot separate
  the change from run-to-run variance.
- The remaining gap is mostly review. Whether review explains the language arm's extra
  passes is still untested: the Markdown arm has no reviewer.
- It does not validate the design.
