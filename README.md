# sdlang

**Specs that stay in sync with the code your agents write.**

You describe your app in `.sd` files. An agent writes the code. `sdc` keeps the two
connected: it knows which files each declaration owns, plans exactly what a spec change
touches, and only commits work that passed tests and an independent review.

```sd
import nextjs

export action create_task {
  summary: "Adds a task"
  input: { title: string }
  output: { id: string, title: string, done: boolean }
  expect {
    stored: "a created task is stored with its title, not done"
  }
}

export nextjs.page new_task {
  route: "/tasks/new"
  calls: [create_task]
  expect {
    required: "the Add button is disabled while the title is empty"
  }
}
```

## A change shows its blast radius before any model runs

Add one field to the action's input:

```diff
-  input: { title: string }
+  input: { title: string, due: timestamp }
```

```console
$ sdc plan
Spec changes
  ~ action       tasks.create_task            own_projection by agent
  ~ page         tasks.new_task               consumed_incompatible by agent

Transactions (serial)
  tx-1  tasks.create_task, tasks.new_task
```

The page calls the action, so it no longer fits and joins the same transaction. The task
list, which calls other actions, isn't touched. The plan is deterministic: the same files
always give the same plan.

## Done means checked, not claimed

```bash
sdc apply --agent ./agent.sh --reviewer ./reviewer.sh
```

Your agent works in a separate worktree. `sdc` diffs what it did, runs the checks from
definitions the agent can't edit, has a reviewer that wrote none of the code compare each
`expect` with the code and its tests, then commits. Any agent that runs headless works:
Claude Code, Codex, your own script.

## Hand edits don't slip through

```console
$ sdc check
drift   src/lib/tasks/store.ts   tasks.store#outputs.main: expected sha256:eb0fbec1…, actual sha256:fa63fcf1…
```

Every owned file's hash is recorded. `sdc check` in CI fails on drift, unapplied spec
changes or stale evidence. Keep a hand edit with `sdc plan --adopt <file>`, which makes it
the new baseline and reviews it against the spec.

## Works with the code you already have

`sdc import src tests` maps existing files to declarations, so you can adopt a project one
feature at a time. Next.js App Router is supported today through the
[`@sdlang/nextjs`](adapters/nextjs) adapter.

## Install

```bash
cargo install --path cli/crates/cli
```

Releases aren't published yet. Then it will be `npm install --save-dev @sdlang/cli`, which
pins the version per project, or a prebuilt binary from the releases page.

## Learn more

- [Getting started](docs/getting-started.md)
- [The language](docs/language.md)
- [How it works](docs/how-it-works.md)
- [Connecting an agent](docs/agents.md)
- [All docs](docs/README.md), including the full [specification](docs/spec.md)
- [VS Code extension and other editors](editors/README.md)

> **Early preview.** The language and protocol will change. The [trial](docs/trial.md)
> that tests whether this works with real agents has only run its setup and first edit,
> so nothing here is proven yet.

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
