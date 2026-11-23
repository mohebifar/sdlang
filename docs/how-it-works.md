# How it works

sdlang has three inputs and one rule. The inputs are the spec (`spec/*.sd`), the recorded
state (`.spec/`) and the code on disk. The rule: no model runs until `sdc apply`, so the
same files always give the same graph and the same plan.

```text
spec/*.sd ──graph──▶ hashes ──┐
.spec/ state ─────────────────┼──plan──▶ transactions ──apply──▶ commits + evidence
files on disk ────────────────┘                                       │
                                     sdc check ◀── state, files, evidence
```

## Graph

`sdc graph` parses every `.sd` file, resolves names through imports and packages, and
lowers the result to one JSON graph (`schemas/graph.schema.json`): declarations, their
members and typed edges. File layout and formatting don't reach the graph, so moving a
declaration between files changes nothing.

## Hashes

Each declaration is hashed in parts, so the planner can tell what kind of change happened:

| Part | Covers | A change here means |
| --- | --- | --- |
| `public` | the interface: summary, a route, an action's input and output, a lib's exports | callers may need to change |
| `behaviour` | its own `prompt`, `expect` and `errors` text | it must be rechecked; callers that observe it are re-reviewed |
| `body` | everything else it states, such as `uses` | only it changes |
| `context` | what it inherits: project and module prompts, matching rules, package skills | it is reviewed against the new context |
| `guidance` | `hint` and `skills` | nothing now; it shapes future work only |

## State

After a declaration is applied, `.spec/state/` records its hashes, the hash of each
projection it consumed from its dependencies, and the bytes (sha256) of every file it owns.
State is committed with the code, so each branch carries its own baseline.

## Plan

`sdc plan` compares the graph with state and the files on disk. Each declaration gets one of:

- a **change**: `create`, `update` or `delete`, made by an agent or a generator;
- an **obligation** only: `verify` (rerun the checks) or `review` (a model reads the code
  against the requirement);
- **blocked**, when one of its files drifted.

Changes travel along edges. If an action's `public` hash changes incompatibly, every
declaration that `calls` it gets an `update`. A compatible change gives callers `verify`,
and a change to its `behaviour` gives them `verify` and `review`. The plan prints the reason
for each entry: `own_projection` means the declaration's own interface changed,
`consumed_incompatible` that something it consumes changed in a way it no longer fits.

A **transaction** is a changed declaration plus every dependent that must change with it,
since an interface change and its callers can't pass checks separately. Transactions run in
dependency order, and each one is a single work order for the agent.

## Apply

For each transaction, `sdc apply`:

1. creates a candidate git worktree and writes the work order, with the declarations, the
   files they own and the context they need;
2. runs your agent command there (see [Connecting an agent](agents.md));
3. computes the diff itself, attributes each changed file to its owner, and rejects edits
   to protected files;
4. runs the checks (typecheck, tests) from pinned definitions outside the worktree, so the
   agent can't weaken them;
5. sends each requirement and the tests that cover it to a reviewer that wrote none of the
   code;
6. on success, commits the transaction, records state and evidence, and fast-forwards your
   branch. On failure it retries with the reason, up to `--max-attempts`.

Each transaction is atomic. If a later one fails, earlier ones stay committed.

## Evidence

Evidence records what was checked, on which inputs (the hashes of the spec, code and check
definitions), and the verdict. A requirement counts as met only when evidence for it is
current under its policy (`tested`, `judged` or `accepted`). Editing the code or the spec
makes the old evidence stale.

## Check

`sdc check` is read-only and meant for CI. It fails on unapplied spec changes, drift (an
owned file whose bytes differ from state), ownership problems, and missing or stale
evidence. With `--rerun` it also reruns the recorded tests at HEAD.

## Drift

When someone edits an owned file by hand, the declaration that owns it is blocked until the
edit is resolved. Revert it, or adopt it with `sdc plan --adopt <path>`: the edited bytes
become the baseline and are verified and reviewed against the spec like any other change.

The [specification](spec.md) covers all of this in detail, including moves, repairs,
deletes, merges between branches and brownfield adoption.
