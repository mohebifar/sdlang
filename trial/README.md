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
| `tools/make-arm.sh` | Makes one arm's workspace for one run. |
| `reference/` | One implementation of the ten edits, as patches, used to calibrate the acceptance tests. Operator only; never shown to an agent. |

Pinned toolchain, from `baseline/pnpm-lock.yaml`: Next.js 15.5.29, React 19.1.10,
TypeScript 5.9.3, Vitest 3.2.7; Node.js 22 and pnpm 10.34.9 from the profile.

## One run

Build the image once:

```bash
docker build -t intlang-trial-v1 trial/profile
```

Make each arm's workspace inside the profile. This is the only step that needs the
network, to install dependencies from the lockfile:

```bash
docker run --rm -v "$PWD/trial:/trial:ro" -v "$PWD/runs:/runs" intlang-trial-v1 /trial/tools/make-arm.sh language /runs/language-1
```

Every later step runs with `--network none`. In the language arm, the setup that
`TRIAL.md` measures but does not count is:

```bash
spec import src tests package.json pnpm-lock.yaml tsconfig.json next.config.ts --accept
```

```bash
git add -A && git commit -m "accept spec import"
```

```bash
spec plan && spec apply --reviewer "<reviewer command>"
```

```bash
spec plan && spec apply --reviewer "<reviewer command>"
```

The first plan acquires all nine declarations with no agent (build checks only). The second
discharges the `verify` and `review` obligations that acquisition leaves; afterwards
`spec check` reports `consistent`. The root layout stays unmanaged, as a kind input.
`spec` must be a Linux build when it runs inside the profile.

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
- The language arm's setup runs end to end with the real adapter: import, acquisition,
  then the obligations discharged by a scripted reviewer, then `spec check` reports
  `consistent`. The trial's own setup uses the pinned model as reviewer instead.
