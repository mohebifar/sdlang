# int lang: the first trial, pinned

This file fixes the first milestone's trial (`SPEC.md`, Open questions and first milestone)
before any run. It is committed before the first run. Any change after the first run is
logged in the run log with its reason, and every arm reruns from the baseline; otherwise the
trial is void.

The question is the one in `SPEC.md`: does `update` work as a scoped edit, well enough to
beat a coding agent working from a Markdown spec, at a cost worth paying? The trial does not
test the parser, `spec check`, packages or anything in `EXTENSIONS.md`.

Contents: Setup · Baseline app · The ten edits · Acceptance tests · Arms and procedure ·
Measures · Stop rule · Run log.

## Setup

| Item | Pinned value |
| --- | --- |
| Toolchain | Node.js 22 LTS, Next.js 15 (App Router, static routes only), React 19, TypeScript 5, pnpm 10, Vitest 3, Playwright 1 for UI acceptance tests. Exact versions are whatever `pnpm-lock.yaml` holds at the baseline commit; they are recorded in the run log and never change during the trial. |
| Profile `trial-v1` | One container image based on `node:22`, pinned by digest at the baseline commit (`trial/profile/Dockerfile`). No network in checks or acceptance runs except localhost. No credentials exist. |
| Adapter | `examples/nextjs-adapter.int`, hard-coded in the runner, unchanged. |
| Coding agent | Claude Code in headless mode, one model (`claude-sonnet-5-5`), the same version in every arm, default settings. The version is recorded in the run log. |
| Reviewer (language arm) | The same model in a separate session that wrote none of the code. |
| Repetitions | Three independent runs per arm, each from the baseline commit with fresh sessions. |
| Retries | Up to two per edit, each given only the arm's own failure output (below). Acceptance tests are never shown to any agent. |
| Time cap | 45 minutes of wall time per edit, retries included. An edit that hits the cap counts as failed. |
| Graphs | The runner lowers the `.int` files itself (`spec graph`); no graph is written by hand. |

## Baseline app

A small task tracker with two modules and only the milestone's kinds (`project`, `stack`,
`module`, `nextjs.page`, `action`, `lib`). It has no database and no auth library: a `lib`
holds records in memory, so a server restart empties it.

The baseline code is written by hand, laid out by the adapter's path templates, with one
test file per declaration covering its `expect` members. It must pass its own tests and
the baseline acceptance tests. The language arm adopts it with `spec import` (strategy
`acquire`), then runs one plan to discharge the resulting `verify` and `review`
obligations. That setup is measured but not counted. The index page's row component,
`src/app/tasks/_parts/task_row.tsx`, is mapped as the created output
`tasks.tasks_index#outputs.task_row`, and the new task page's title form,
`src/app/tasks/new/_parts/title_form.tsx`, as `tasks.new_task#outputs.title_form`. The
baseline, the arms' overlays, the acceptance tests and the profile are in `trial/`.

`spec/app.int`

```spec
import nextjs

project tasks_app {
  summary: "A small task tracker used to trial int lang"
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

`spec/tasks/module.int`

```spec
module {
  summary: "Creating, listing and completing tasks"
  prompt: """
    Every tasks page uses the shared page shell: a heading, then the content.
    Show dates as YYYY-MM-DD.
  """
}
```

`spec/tasks/libs.int`

```spec
export lib store {
  summary: "In-memory record storage keyed by id; empty after a restart"
  exports: ["insert", "all", "find", "save"]
  expect {
    newest_first: "all returns records newest first by created_at"
    unknown_id: "find returns null for an unknown id"
  }
}

export lib dates {
  summary: "Date formatting for task pages"
  exports: ["formatDate"]
  expect {
    iso_date: "formatDate returns the date part of a timestamp as YYYY-MM-DD in UTC"
  }
}
```

`spec/tasks/actions.int`

```spec
export action create_task {
  summary: "Adds a task"
  input: { title: string }
  output: { id: string, title: string, done: boolean, created_at: timestamp }
  uses: [store]
  expect {
    stored: "a created task is returned by list_tasks with its title, not done"
  }
}

export action list_tasks {
  summary: "Lists all tasks, open ones first, each group newest first"
  input: { }
  output: [{ id: string, title: string, done: boolean, created_at: timestamp }]
  uses: [store]
  expect {
    order: "open tasks come before done ones; within each group, newest first"
  }
}

export action complete_task {
  summary: "Marks a task done"
  input: { id: string }
  output: { id: string, done: boolean }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
  expect {
    marks_done: "after complete_task, list_tasks shows the task as done"
  }
}
```

`spec/tasks/pages.int`

```spec
import nextjs
import about

export nextjs.page tasks_index {
  route: "/tasks"
  summary: "Lists tasks"
  calls: [list_tasks, complete_task]
  uses: [dates]
  prompt: """
    One row per task: title, created date and a Complete button for open tasks.
    A link to @{new_task} sits above the list.
  """
  expect {
    listed: "every task is listed with its title and created date"
    complete: "Complete marks the task done and moves it below the open tasks"
    empty: "with no tasks, the page says 'No tasks yet' and links to @{new_task}"
    footer: "the footer links to @{about.about}"
  }
}

export nextjs.page new_task {
  route: "/tasks/new"
  summary: "A form that adds a task"
  calls: [create_task]
  prompt: """
    A title field and an Add button. After adding, go to @{tasks_index}.
  """
  expect {
    adds: "submitting a title adds the task and shows the list with it"
    required: "the Add button is disabled while the title is empty"
  }
}
```

`spec/about/about.int`

```spec
import nextjs
import tasks

export nextjs.page about {
  route: "/about"
  summary: "Says what the app is for"
  prompt: """
    Two short paragraphs. Link back to @{tasks.tasks_index}.
  """
  expect {
    explains: "the page has the heading 'About' and links to @{tasks.tasks_index}"
  }
}
```

The strong arm gets the same content as Markdown: `docs/specs/tasks.md` and
`docs/specs/about.md`, one section per declaration with its summary, signature, prompt and
expectations copied verbatim. The plain arm gets neither.

## The ten edits

Edits run in order, each on the arm's own result of the previous edit. Each edit lists:
the spec change; the one-line instruction for the plain arm; what it tests; the plan the
language arm should produce; the files the edit concerns (for out-of-scope writes, test
files of those declarations included); and the trap, if any. The strong arm's Markdown
change carries the same text as the spec change. Acceptance test ids are defined in the
next section.

**E1. Create, with a compatible `lib` change.** Add `"remove"` to `store`'s exports, with
`expect { removes: "after remove, find returns null and all omits the record" }`. Add:

```spec
export action delete_task {
  summary: "Deletes a task"
  input: { id: string }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
}
```

and in `tasks_index`, `calls: [list_tasks, complete_task, delete_task]` and
`delete: "each row has a Delete button that removes the task from the list"`.
- Plain: "Add a Delete button to each task row; deleting removes the task for good."
- Tests: create, a compatible `lib` export addition, a caller's authored change.
- Expected plan: `update store`; `create delete_task` after it; `update tasks_index` after
  that. `create_task`, `list_tasks` and `complete_task` get `verify` (implementation
  impact from `store`).
- Concerns: `store`, `delete_task`, `tasks_index` (with `_parts/`).
- Acceptance: A1.1, A1.2.

**E2. Compatible input widening.** Add `type TaskStatus = "open" | "done"` to
`actions.int`, and change `list_tasks`'s input to `{ status: TaskStatus? }` with
`filter: "with a status, only tasks with that status are returned; without one, all"`.
- Plain: "Let list_tasks take an optional status, open or done, and return only those
  tasks; with no status it returns all."
- Tests: a compatible change to a consumed projection gives the caller `verify`, not
  `update`.
- Expected plan: `update list_tasks`; `tasks_index` gets `verify`.
- Concerns: `list_tasks`.
- Trap: adding a filter control to the index page.
- Acceptance: A2.1, A2.2.

**E3. Interface change with a caller, and a move.** Add
`moved tasks.complete_task -> tasks.set_task_done` and replace `complete_task` with:

```spec
export action set_task_done {
  summary: "Marks a task done or open"
  input: { id: string, done: boolean }
  output: { id: string, done: boolean }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
  expect {
    toggles: "set_task_done with done false makes a done task open again"
  }
}
```

In `tasks_index`, the prompt's row line becomes "title, created date and a checkbox that
toggles done", `calls` names `set_task_done`, and `complete` becomes
`toggle: "the checkbox toggles a task between open and done, moving it between the groups"`.
- Plain: "Replace complete_task with set_task_done(id, done) and replace the Complete button
  with a checkbox that toggles done both ways."
- Tests: an incompatible signature change pulls its caller into one transaction; `moved`
  keeps identity and moves the output files.
- Expected plan: one transaction, `moved + update set_task_done` and `update tasks_index`.
- Concerns: `set_task_done` (formerly `complete_task`), `tasks_index`.
- Acceptance: A3.1, A3.2. Retires B3.

**E4. Normative prose change to an action.** Add to `create_task`:
`prompt: """Store the title with surrounding whitespace removed and inner runs of spaces collapsed to one."""`.
- Plain: "create_task should trim the title and collapse repeated spaces."
- Tests: a behaviour change in an action gives its caller `verify` + `review`, and the
  caller is not rewritten when it already conforms.
- Expected plan: `update create_task`; `new_task` gets `verify` + `review`.
- Concerns: `create_task`.
- Trap: trimming in the page as well.
- Acceptance: A4.1, A4.2.

**E5. Compatible fan-in.** `list_tasks`'s output items and `set_task_done`'s output both
gain `done_at: timestamp?`, with `expect` members `done_at: "done_at is set when a task is
marked done and null when it is open"` on both.
- Plain: "Record when a task was marked done: list_tasks and set_task_done return done_at,
  null for open tasks."
- Tests: two compatible changes with one caller. Under the rule in `SPEC.md`, Transactions,
  the caller is verified in each transaction; the trial measures what that costs.
- Expected plan: two transactions, `update list_tasks` and `update set_task_done`, with
  `tasks_index` `verify` in each.
- Concerns: `list_tasks`, `set_task_done`.
- Acceptance: A5.1, A5.2.

**E6. Inherited normative change, with a needed spec edit.** The `tasks` module prompt gains
"Every tasks page heading shows the number of open tasks in brackets, for example
'Tasks (3)'." `new_task` gains `calls: [create_task, list_tasks]`, since it needs the count.
- Plain: "Every tasks page heading shows the open-task count in brackets, like
  'Tasks (3)'."
- Tests: inherited prose gives `review` to every declaration in the module; nonconformance
  escalates to `update`; actions and libs that conform are left alone.
- Expected plan: `update new_task` (authored `calls` change); `review` + `verify` for
  `tasks_index`, `create_task`, `list_tasks`, `set_task_done`, `delete_task`, `store` and
  `dates`, with `tasks_index` expected to escalate to `update`.
- Concerns: `tasks_index`, `new_task`.
- Acceptance: A6.1, A6.2.

**E7. Route move with created parts.** `tasks_index`'s route becomes `"/todo"`.
- Plain: "Move the task list from /tasks to /todo; everything that links to it follows."
- Tests: a route change moves the slot files and the created `_parts/` files, and the
  `mentions` holders (`new_task`, `about`) are updated to the new link.
- Expected plan: `update tasks_index` with its outputs moved to `src/app/todo/`; `update
  new_task` and `update about`.
- Concerns: `tasks_index`, `new_task`, `about`.
- Acceptance: A7.1, A7.2, A7.3. The acceptance route map's `index` entry becomes `/todo`.

**E8. Adopted hand edit.** Before E8, the operator applies the same hand patch to every
arm: the About heading becomes "About this app" and a paragraph "Built for the int lang
trial." is added. The spec change: `explains` says "the heading 'About this app'" and
`trial_note: "the page says 'Built for the int lang trial.'"` is added. In the language
arm the operator resolves the drift with `adopt`.
- Plain: "The About page was edited by hand and is right as it is; make sure its tests
  cover the new heading and note."
- Tests: drift is adopted without an agent rewriting the file; evidence goes stale and is
  renewed.
- Expected plan: no `update` of the page file; `verify` + `review` for `about`; a test
  update if the reviewer finds the tests do not cover `trial_note`.
- Concerns: `about` (test file only; the page file must keep the hand-patched bytes).
- Acceptance: A8.1, A8.2.

**E9. Scope temptation.** `new_task` gains
`too_long: "a title over 80 characters shows 'Keep it under 80 characters' under the field and is not submitted"`.
- Plain: "On the new task page, block titles over 80 characters with the message 'Keep it
  under 80 characters'."
- Tests: whether the agent stays in the page or also adds validation to `create_task`,
  whose contract did not change.
- Expected plan: `update new_task` only.
- Concerns: `new_task`.
- Trap: adding a length check to `create_task`. A2-style direct calls detect it (A9.2).
- Acceptance: A9.1, A9.2.

**E10. Delete.** Remove `spec/about/about.int`, the `footer` member and the `import about`
from `pages.int`.
- Plain: "Remove the About page and the footer link to it."
- Tests: delete, with a referencing caller updated first.
- Expected plan: `update tasks_index`, then `delete about`.
- Concerns: `tasks_index`, `about`.
- Acceptance: A10.1, A10.2. Retires B6, B7, A7.3, A8.1, A8.2.

## Acceptance tests

Written before the first run by someone who has not seen any arm's output, from this file
only. They live outside every arm's workspace and are run by the operator after each edit
against that arm's tree. Two kinds:

- **UI tests** (Playwright) against `next build` then `next start`, with a fresh server per
  test file so the in-memory store starts empty. Routes come from a route map the operator
  owns.
- **Contract tests** (Vitest) that import an action's module from its adapter path
  (`src/server/tasks/<name>.ts`) and call its default export. A declared error is a
  rejection with an `Error` whose `code` is the error's key, the adapter's convention.
  Names, signatures and that convention are part of every arm's instructions (the
  baseline's `complete_task` shows it to the plain arm), so this is fair across arms.

| Id | Edit | Kind | Asserts |
| --- | --- | --- | --- |
| B1 | baseline | UI | an added task is listed with its title and today's date as YYYY-MM-DD |
| B2 | baseline | UI | with no tasks the list says "No tasks yet" and links to the new task page |
| B3 | baseline | UI | Complete moves a task below the open ones |
| B4 | baseline | UI | the Add button is disabled for an empty title; adding lands on the list showing the task |
| B5 | baseline | contract | `list_tasks` orders open before done, newest first in each group |
| B6 | baseline | UI | the About page has the heading "About" and links to the list |
| B7 | baseline | UI | the list's footer links to the About page |
| A1.1 | E1 | UI | Delete removes a task from the list |
| A1.2 | E1 | contract | `delete_task` on an unknown id fails with `not_found` |
| A2.1 | E2 | contract | `list_tasks({ status: "done" })` returns only done tasks, `"open"` only open ones |
| A2.2 | E2 | contract | `list_tasks({})` returns all tasks |
| A3.1 | E3 | UI | the checkbox marks a task done and then open again |
| A3.2 | E3 | contract | `set_task_done` on an unknown id fails with `not_found` |
| A4.1 | E4 | contract | `create_task({ title: "  Buy   milk " })` returns and stores "Buy milk" |
| A4.2 | E4 | UI | a title typed with extra spaces is listed trimmed |
| A5.1 | E5 | contract | `set_task_done` returns `done_at` set when done, null when open |
| A5.2 | E5 | contract | `list_tasks` items carry `done_at` consistent with A5.1 |
| A6.1 | E6 | UI | the list heading shows the open count, e.g. "Tasks (2)" |
| A6.2 | E6 | UI | the new task page heading shows the same count |
| A7.1 | E7 | UI | `/todo` lists tasks; `/tasks` is not the list |
| A7.2 | E7 | UI | adding a task lands on `/todo` |
| A7.3 | E7 | UI | the About page links to `/todo` |
| A8.1 | E8 | UI | the About page shows "About this app" and "Built for the int lang trial." |
| A8.2 | E8 | file | the About page file is byte-identical to the hand patch |
| A9.1 | E9 | UI | an 81-character title shows the message and adds nothing |
| A9.2 | E9 | contract | `create_task` called directly with an 81-character title still succeeds |
| A10.1 | E10 | UI | `/about` responds 404 |
| A10.2 | E10 | UI | the list has no link to `/about` |

A test is **active** from its edit until the edit that retires it. After E3, B3 is
retired; B1, B2, B4 and B5 keep running with the route map updated at E7.

**Weak-test probe** (language arm only, separate from the measures below). For E2, E4 and
E9, a deliberately weak test is written in advance: it calls the code and asserts only that
nothing throws. After each run's edit is published, the operator submits a coverage review
order that binds the weak test to the edit's new requirement, five times per edit and run
(45 judgements in total), and records how many the reviewer accepts as sufficient.

## Arms and procedure

All arms start from the same baseline code and use the same agent, edits, retries, time cap
and acceptance tests.

- **Language.** The operator edits the `.int` files, runs `spec plan`,
  compares the plan with the expected plan above (recording any difference as a planner
  defect, then applying anyway), and runs `spec apply`. On failure, the retry gets the
  runner's failure report.
- **Strong.** The operator applies the edit to `docs/specs/*.md` and gives the agent: "The
  spec in docs/specs/ changed; the diff is below. Change the code to match, keep all other
  behaviour, and keep the project's tests passing." The agent may run the project's tests,
  typecheck and build. A retry gets their output.
- **Plain.** The agent gets the one-line instruction and nothing else; retries as strong.

Nobody edits code by hand in any arm, apart from the E8 patch. After each edit a reviewer
who does not know the arm reads the code diff, stripped of `spec/`, `.spec/`, `docs/specs/`
and graph files, and records "accept" or "would request changes" with a reason and the
minutes spent.

## Measures

Per edit and run:

| Measure | Definition |
| --- | --- |
| Intended | All of the edit's acceptance tests pass. |
| Regressions | Active earlier acceptance tests (baseline included) that passed before and fail now. Counted once per (run, test), at the first edit where it fails. |
| Out-of-scope files | Files changed outside the edit's "Concerns" list. Reported, never decisive: the language arm's envelope refuses these writes by construction. |
| Review corrections | Edits the blind reviewer would send back. |
| Diff ratio | Changed code lines over changed spec lines (Markdown lines for strong). |
| Cost | Model tokens in and out across all sessions (agents and reviewers), wall time, and human minutes (operator plan review, drift choices and the blind review). |

## Stop rule

Fixed now. Totals are over the 30 edit-runs per arm (10 edits by 3 runs). The language
continues to milestone step 5 only if all four hold:

1. **Intended behaviour is not worse:** Intended(language) ≥ Intended(strong) − 1.
2. **Behaviour is retained better:** Regressions(language) ≤ half of Regressions(strong).
3. **The comparison can show it:** Regressions(strong) ≥ 4. If the strong arm keeps almost
   everything working on this app, the language's main claim is not shown, and the trial
   counts as a failure for the language, not as a reason to design harder edits.
4. **The cost is paid for:** tokens(language) ≤ 2 × tokens(strong), and human
   minutes(language) ≤ 1.5 × human minutes(strong).

Out-of-scope files, review corrections and the diff ratio are reported alongside the result
but do not decide it. If the plain arm beats the strong arm on Intended or Regressions, the
edits are miscalibrated, and the report says so.

Separately, if the reviewer accepts more than 9 of the 45 weak-test judgements (20%), the
`tested` label is unreliable as designed, and coverage review is redesigned before step 5,
whatever the stop rule says.

## Run log

Filled in during the trial; empty until the first run.

| Date | Arm | Run | Edit | Event |
| --- | --- | --- | --- | --- |
