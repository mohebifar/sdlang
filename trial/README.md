# The first trial: baseline and harness

Everything `TRIAL.md` needs before its first run. `TRIAL.md` is the contract; this file
says where each piece is and how the operator uses it.

| Path | What it is |
| --- | --- |
| `baseline/` | The task tracker every arm starts from, written by hand and laid out by the adapter's path templates. One test file per declaration covers its `expect` and `errors` members, each tagged `[case:<key>]`. |
| `arms/language/` | The language arm's overlay: the spec tree from `TRIAL.md` verbatim, `spec.lock` and a copy of `adapters/nextjs` under `spec-packages/nextjs`. |
| `arms/strong/` | The strong arm's overlay: `docs/specs/tasks.md` and `about.md`, the same content as Markdown. The plain arm has no overlay. |
| `acceptance/` | The acceptance tests: Playwright UI tests (`ui/`), Vitest contract tests (`contract/`), the table of when each test is active (`tests.json`, with the route map), the runner (`run.mjs`) and the E8 hand patch (`patches/e8.mjs`). Never copied into an arm. |
| `profile/Dockerfile` | Profile `trial-v1`: `node:22` pinned by digest, pnpm 10.34.9 and the Chromium that Playwright 1.65.1 drives. |
| `profile/spec.Dockerfile` | The Linux build of `spec` (Rust 1.89 on bookworm, pinned by digest), output to `profile/bin/`, which git ignores. Kept out of the profile image so every arm runs in the same image. |
| `tools/make-arm.sh` | Makes one arm's workspace for one run. |
| `tools/agent.sh`, `tools/reviewer.sh` | The language arm's agent and reviewer commands: Claude Code headless with `claude-sonnet-5-5`. The agent edits files and uses the spec MCP tools, with no shell; the reviewer reads and writes only its verdict. `TRIAL_LOG_DIR` keeps each session's token and cost record. |
| `tools/spec_edit.py` | Applies one edit's spec change to the language arm, as `TRIAL.md` states it. |
| `reference/` | One implementation of the ten edits, as patches, used to calibrate the acceptance tests. Operator only; never shown to an agent. |

Pinned toolchain, from `baseline/pnpm-lock.yaml`: Next.js 15.5.29, React 19.1.10,
TypeScript 5.9.3, Vitest 3.2.7; Node.js 22 and pnpm 10.34.9 from the profile.

## One run

Build the image and the Linux `spec` once. The binary is for the Docker host's
architecture; add `--platform` to both builds to choose another:

```bash
docker build -t intlang-trial-v1 trial/profile
```

```bash
docker build -f trial/profile/spec.Dockerfile --output trial/profile/bin cli
```

Make each arm's workspace inside the profile. This is the only step that needs the
network, to install dependencies from the lockfile:

```bash
docker run --rm -v "$PWD/trial:/trial:ro" -v "$PWD/runs:/runs" intlang-trial-v1 /trial/tools/make-arm.sh language /runs/language-1
```

Every later step runs with `--network none`. The language arm's containers also mount the
binary on the path; the other arms' do not:

```bash
docker run --rm -it --network none -v "$PWD/trial/profile/bin/spec:/usr/local/bin/spec:ro" -v "$PWD/runs:/runs" -w /runs/language-1 intlang-trial-v1 bash
```

The exception, for now: the agent and reviewer need the network and a signed-in Claude
Code, which the profile does not have, so `spec apply` with them runs on the host, and so
do the checks it runs. Keeping those checks in the profile is open (`DECISIONS.md` C.24).
`$TRIAL` is this directory.
In the language arm, the setup that
`TRIAL.md` measures but does not count is:

```bash
spec import src tests package.json pnpm-lock.yaml tsconfig.json next.config.ts --accept
```

```bash
git add -A && git commit -m "accept spec import"
```

```bash
spec plan && spec apply --reviewer "$TRIAL/tools/reviewer.sh"
```

```bash
spec plan && spec apply --reviewer "$TRIAL/tools/reviewer.sh"
```

The first plan acquires all nine declarations with no agent (build checks only). The second
discharges the `verify` and `review` obligations that acquisition leaves; afterwards
`spec check` reports `consistent`. The root layout stays unmanaged, as a kind input.

Each edit in the language arm is: the spec change, a commit, a plan to compare with
`TRIAL.md`'s expected plan, and the apply:

```bash
python3 tools/spec_edit.py E3 /runs/language-1
```

```bash
git add -A spec && git commit -m "spec: E3"
```

```bash
spec plan && spec apply --agent "$TRIAL/tools/agent.sh" --reviewer "$TRIAL/tools/reviewer.sh"
```

Before E8, apply the hand patch (below) and commit only the page; then, after E8's spec
change, plan with `spec plan --adopt src/app/about/page.tsx`.

After each edit, run the acceptance tests active at that edit against the arm's tree,
from a copy of `acceptance/` with its own dependencies installed. The runner builds the arm
in place and removes the build's output afterwards, so the agent never sees a stale
`.next/`:

```bash
node run.mjs /runs/language-1 E3 --out /runs/language-1-E3.json
```

Before E8, apply the hand patch to every arm and keep the copy A8.2 compares against:

```bash
node patches/e8.mjs /runs/language-1 /runs/language-1-about.patched.tsx
```

and run E8 onwards with `PATCHED_ABOUT` set to that copy.

## What has been checked

- The baseline's 14 tests, the typecheck and `next build` pass, on the host and in the
  profile with no network.
- The baseline acceptance tests B1 to B7 pass against the baseline, on the host and in the
  profile.
- Every edit's acceptance tests fail against the unchanged baseline, except A2.2 and A9.2,
  which guard behaviour the baseline already has.
- Against the reference edits (`reference/`), every test active at each step passes, from
  7 of 7 at the baseline to 22 of 22 at E10. That shows each test can pass on a correct
  edit, not that every incorrect edit fails it.
- The language arm's whole procedure, adoption and all ten edits, runs with a
  deterministic agent and reviewer that copy and compare the reference
  (`reference/tools/`): every plan matches `TRIAL.md`'s expected plan, every transaction
  publishes, `spec check` is `consistent` after each edit and every active acceptance test
  passes. It says nothing about how a real agent does.
- The Linux `spec` passes the CLI's 151 tests in its build image, and inside the profile
  with no network it runs the language arm's setup and E1 to E3 with the same agent and
  reviewer: every transaction publishes and `spec check` is `consistent` after each. Built
  and run on arm64 only.
- With the real agent and reviewer (`tools/agent.sh`, `tools/reviewer.sh`), on the host
  rather than in the profile, the language arm's setup and E1 ran from a fresh arm: every
  transaction published, `spec check` reported `consistent` and E1's 9 active acceptance
  tests passed. One run, so it shows the procedure works with a real agent, not how often.
- The language arm's setup runs end to end with the real adapter: import, acquisition,
  then the obligations discharged by a scripted reviewer, then `spec check` reports
  `consistent`. The trial's own setup uses the pinned model as reviewer instead.
