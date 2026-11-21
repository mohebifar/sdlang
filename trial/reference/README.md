# Reference edits (operator only)

One implementation of each of `TRIAL.md`'s ten edits, made to calibrate the acceptance
tests before the first run. It is not an arm, it is not measured, and it must never be
shown to any agent: `tools/make-arm.sh` never copies it.

`0001-E1.patch` to `0010-E10.patch` apply in order with `git am` to a workspace made by
`tools/make-arm.sh plain`. `E8-about.patched.tsx` is the About page right after the E8
hand patch (`acceptance/patches/e8.mjs`), which A8.2 compares against. Each patch keeps the
project's own tests and typecheck passing.

## Result

With the reference checked out at each step, every acceptance test active at that step
passes, and the runner leaves the tree as it found it:

| Step | Active tests | Passed |
| --- | --- | --- |
| baseline | 7 | 7 |
| E1 | 9 | 9 |
| E2 | 11 | 11 |
| E3 | 12 | 12 |
| E4 | 14 | 14 |
| E5 | 16 | 16 |
| E6 | 18 | 18 |
| E7 | 21 | 21 |
| E8 | 23 | 23 |
| E9 | 25 | 25 |
| E10 | 22 | 22 |

This shows that each test can pass on a correct edit; with every edit test failing on the
unchanged baseline (except A2.2 and A9.2, which guard behaviour the baseline already has),
the tests separate the two. It does not show that every incorrect edit fails them.

## The language arm's procedure, replayed

`tools/oracle_agent.py` makes a work order's subjects equal to the reference, inside the
order's envelope, registering created outputs and binding tests by their case tags;
`tools/oracle_reviewer.py` finds a subject conforming exactly when its code equals the
reference. With them, the language arm's procedure was replayed from adoption to E10
(`../tools/spec_edit.py` for each spec change, `spec plan`, `spec apply`, `spec check`,
the acceptance runner):

| Edit | Plan against `TRIAL.md` | Applied | Acceptance |
| --- | --- | --- | --- |
| E1 | matches, after adding `new_task`'s transitive `verify` to `TRIAL.md` | 3 transactions | 9/9 |
| E2 | matches, after adding `tasks_index`'s observed `review` to `TRIAL.md` | 1 | 11/11 |
| E3 | matches | 1 (moved files moved by the CLI) | 12/12 |
| E4 | matches | 1 | 14/14 |
| E5 | matches, after adding the observed `review` to `TRIAL.md` | 2 | 16/16 |
| E6 | matches | 2 (`tasks_index` escalated to an update) | 18/18 |
| E7 | matches | 1 (route move; `about` and `new_task` updated for their mentions) | 21/21 |
| E8 | matches, with `spec plan --adopt` | 1 (page kept byte-identical; its test escalated and updated) | 23/23 |
| E9 | matches | 1 | 25/25 |
| E10 | matches, one transaction | 1 | 22/22 |

`spec check` was `consistent` after each edit. A real agent and reviewer will do worse;
this replay checks the runner and the plans, not them.

