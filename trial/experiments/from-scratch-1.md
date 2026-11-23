# From-scratch experiment 1 (2026-11-21)

One run per arm. Each arm builds the trial's task tracker from an empty scaffold, then makes
`docs/trial.md`'s ten edits on its own result. This is not the pinned trial: `docs/trial.md` starts every
arm from the hand-written baseline and runs each arm three times. It reuses the trial's spec,
edits, agent model and acceptance tests.

## Setup

- **Scaffold, both arms:** `tools/make-arm.sh <arm> <dir> scratch`. The toolchain files
  (`package.json`, lockfile, `tsconfig.json`, `next.config.ts`, `vitest.config.ts`), the root
  layout and installed dependencies. No app code, no tests.
- **Language arm:** the spec tree from `docs/trial.md` and the adapter. The toolchain files are
  adopted with `spec import`; then `spec plan` and `spec apply`, with the agent and a separate
  reviewer. Each edit is `spec_edit.py`, a commit, a plan, and an apply. Retries happen
  inside `spec apply` (three attempts).
- **Markdown arm** (`docs/trial.md`'s strong arm): the same requirements in `docs/specs/*.md`. The
  build instruction asks for the app the spec describes, with the tests its conventions
  describe. Each edit is `docs_edit.py` (the same text as the spec change) and `docs/trial.md`'s
  instruction with the diff. When the project's tests, typecheck or build fail, the agent
  retries up to twice with their output.
- **Agent, both arms:** Claude Code 2.1.330 headless with `claude-sonnet-5-5`, given the same
  tools: read, edit and sandboxed Bash. The language arm's agent also has the spec MCP tools.
  - Bash runs in Claude Code's sandbox, with no network and writes only under the working
    directory.
  - File edits are auto-approved only under the working directory (`acceptEdits`).
  - Neither the agent nor the reviewer can read this repository, the other arm or the
    operator's files.
- **Scoring:** the acceptance tests active at each step, after each step
  (`baseline` for the build). The measures are `docs/trial.md`'s; there is no blind review.
- **Driver:** `tools/experiment.py`. The results, session records, plans and logs are in
  `~/intlang-runs/exp2/operator/`; the workspaces are `~/intlang-runs/exp2/{language,strong}`.

An earlier attempt (`exp1-void`) was void in both arms. A permission rule, `Edit(./**)`,
refused some edits, and in headless mode an unapproved ask is a refusal: the Markdown agent
changed nothing at E2, and every language reviewer was refused its verdict. The wrappers now
use `acceptEdits`, and the driver counts refusals per step. Both arms of the run below had
none.

## Results

| | Language | Markdown |
| --- | --- | --- |
| Build: baseline tests B1–B7 | 7/7 | 7/7 |
| Build: cost, wall time | $5.98, 651 s | $0.67, 134 s |
| Edits intended (all of the edit's tests pass) | 9/10 | 7/10 |
| Regressions | 0 | 0 |
| Acceptance tests passing after E10 | 22/22 | 20/22 |
| Edits: cost | $16.58 | $2.52 |
| Edits: model tokens in (mostly cache reads), out | 22.9M, 177k | 3.4M, 27k |
| Edits: wall time | 1,679 s | 396 s |
| Edits: sessions (agent + reviewer) | 24 + 16 | 10 + 0 |
| Own tests, typecheck and build after E10 | pass | pass |
| `spec check` after every step | `consistent` | n/a |

Per edit (✓ intended, ✗ not; cost):

| | E1 | E2 | E3 | E4 | E5 | E6 | E7 | E8 | E9 | E10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Language | ✓ 2.17 | ✓ 0.82 | ✓ 1.49 | ✓ 0.66 | ✓ 2.08 | ✓ 3.61 | ✓ 3.29 | ✗ 0.46 | ✓ 0.82 | ✓ 1.18 |
| Markdown | ✓ 0.25 | ✓ 0.18 | ✗ 0.47 | ✓ 0.18 | ✓ 0.27 | ✗ 0.29 | ✓ 0.26 | ✗ 0.24 | ✓ 0.21 | ✓ 0.18 |

### The failures

- **Markdown E3:** the done checkbox renders `readonly` and does not toggle (A3.1). The
  agent's own tests, typecheck and build passed, so no retry was triggered.
- **Markdown E6:** the new task page's heading stays at "New task (0)" after tasks are added
  (A6.2). The page was most likely rendered at build time, so the count never updates.
- **Markdown E8:** after the hand patch, the agent merged the patched paragraph into the next
  one. The page no longer matches the patch byte for byte (A8.2).
- **Language E8, the same outcome by another route, and a defect in `spec`:**
  1. `spec plan --adopt src/app/about/page.tsx` planned only `verify` and `review` obligations,
     with no coding dispatch, which is correct.
  2. The old test, which expects "About", failed against the patched page.
  3. The retry agent then rewrote the page as well as the test, and `spec apply` published it.
  An adopted file's bytes are meant to be the new baseline, so a dispatch that discharges an
  adoption must not be allowed to write that file. This defect is not fixed yet.

### Where the language arm's cost went

- **Reviewers.** Every coding transaction gets review: 16 of the language arm's 40 edit
  sessions, about $5.70 of its $16.58. The Markdown arm has no reviewer.
- **Retries on envelope errors, the larger avoidable part (a second defect in `spec`).**
  E7 changes three pages in one transaction, which `spec apply` runs as three dispatches in one
  candidate. Each dispatch's `validate_result` reported the other two dispatches' changes as
  outside its envelope. That cost six agent sessions, $2.91, for a route change that cost the
  Markdown arm $0.26.
- **Other retries.** E1 and E5 each had one retry. E6's came from a reviewer, which found the
  index heading without the count; that retry is the review doing its job.
- **Per session, the language agent costs about twice as much** ($0.40–0.50 against
  $0.20–0.25). It reads the order, the context and the MCP tools' output before it starts.

### Against `docs/trial.md`'s stop rule, informally

This is one run per arm, with a from-scratch start, so it is not the trial. The stop rule
counts edits only.

1. Intended(language) 9 ≥ Intended(strong) 7 − 1: holds.
2. Regressions(language) 0 ≤ half of Regressions(strong) 0: holds, but only trivially.
3. Regressions(strong) ≥ 4: **fails.** The Markdown arm lost nothing it had. On this app the
   language's main claim, keeping behaviour that already works, is not shown.
4. Tokens(language) ≤ 2 × tokens(strong): **fails**, at 6.8× (22.9M against 3.4M input, 6.5×
   for output). Cost is 6.6×.

## What this shows and does not

- With the same model and tools, the language arm got two more edits right, E3 and E6, at
  about 6.6 times the cost. It cannot separate the language from its review: the language arm's
  reviewer reads every change against the requirements, and the Markdown arm has nothing
  equivalent. A Markdown arm with the same reviewer would test that.
- Neither arm regressed anything, so the claim the trial is built around was not exercised.
- Both arms lost the hand edit at E8. The language arm lost it through a defect in its own
  protection.
- It is one run per arm. The same arm can pass or fail an edit on another run.
- It does not validate the design.
