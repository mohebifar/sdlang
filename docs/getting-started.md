# Getting started

This walks through adding sdlang to an existing Next.js App Router project: describe what
is there, let `sdc` adopt the code, then change the spec and have an agent apply it.

sdlang is an early preview. The only adapter today is for Next.js, and the commands and
file formats will change.

## 1. Install sdc

Until the first release is published, build it from this repository (Rust 1.89 or later):

```bash
cargo install --path cli/crates/cli
```

After the first release, install it per project so everyone runs the same version:

```bash
npm install --save-dev @sdlang/cli
```

## 2. Add the Next.js adapter

Kinds such as `nextjs.page` come from a package. Copy the adapter into your project and
point `sd.lock` at it:

```bash
mkdir -p spec-packages && cp -R path/to/sdlang/adapters/nextjs spec-packages/nextjs
```

`sd.lock`:

```json
{
  "lock": "sdlang.lock/0",
  "packages": [{ "id": "npm:@sdlang/nextjs", "path": "spec-packages/nextjs" }]
}
```

Once `@sdlang/nextjs` is on npm, install it and use `node_modules/@sdlang/nextjs` as the path.

## 3. Describe the project

Specs live in `spec/`. A directory is a module. Start with the project and its stack:

`spec/app.sd`

```sd
import nextjs

project tasks_app {
  summary: "A small task tracker"
  verify: "tested"
  prompt: """
    Server-render by default. Plain English, sentence case.
  """
}

stack {
  language: { name: "typescript" }
  runtime: { name: "nodejs", version: "22" }
  framework: { name: "nextjs", version: "15" }
  package_manager: { name: "pnpm" }
}
```

Then describe what the app does, one module per feature:

`spec/tasks/actions.sd`

```sd
export action create_task {
  summary: "Adds a task"
  input: { title: string }
  output: { id: string, title: string, done: boolean, created_at: timestamp }
  expect {
    stored: "a created task is stored with its title, not done"
  }
}
```

`spec/tasks/pages.sd`

```sd
import nextjs

export nextjs.page new_task {
  route: "/tasks/new"
  summary: "A form that adds a task"
  calls: [create_task]
  prompt: """
    A title field and an Add button. After adding, go back to the list.
  """
  expect {
    required: "the Add button is disabled while the title is empty"
  }
}
```

Check that it compiles:

```bash
sdc graph . > /dev/null
```

## 4. Adopt the existing code

`sdc import` proposes which existing files each declaration owns, using the adapter's path
templates (a page at `/tasks/new` lives in `src/app/tasks/new/page.tsx`):

```bash
sdc import src tests
```

Review the proposal, then accept it and commit:

```bash
sdc import src tests --accept
```

The first plan acquires those files. It needs no coding agent: it runs the tests and has a
reviewer check each `expect` against the code.

```bash
sdc plan
```

```bash
sdc apply --agent ./agent.sh --reviewer ./reviewer.sh
```

[Connecting an agent](agents.md) shows what the two commands receive. Each transaction
lands as its own commit, with state in `.spec/`, which you commit too.

## 5. Change the spec

Edit a declaration, for example add `due: timestamp?` to `create_task`'s input, and plan:

```bash
sdc plan
```

The plan lists every declaration the edit reaches and why. `new_task` calls `create_task`,
so it joins the same transaction. Nothing runs until you apply:

```bash
sdc apply --agent ./agent.sh --reviewer ./reviewer.sh
```

The agent works in a separate git worktree. Its change is committed only after the checks
and an independent review pass.

## 6. Keep it in sync in CI

```bash
sdc check
```

`sdc check` fails when the spec has unapplied changes, an owned file was edited by hand
(drift), or the evidence behind a declaration is out of date. To keep a hand edit, adopt
it: its bytes become the new baseline and are reviewed against the spec.

```bash
sdc plan --adopt src/lib/tasks/store.ts
```

## Next

- [The language](language.md) for everything a declaration can say.
- [How it works](how-it-works.md) for what the plan's reasons mean.
