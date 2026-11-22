# Tasks app spec

The source of truth for this app's behaviour. Each section below is one part of the app,
with its summary, its signature, its instructions and its expectations.

## Conventions

- A page's module is `src/app/<route>/page.tsx`; its default export is the page component.
  Pages are server components unless a file starts with `"use client"`. Client-side parts
  live under the page's `_parts/` directory.
- An action is one server module, `src/server/<module>/<name>.ts`. Its default export takes
  the action's input object and resolves to its output. A listed error rejects with an
  `Error` whose `code` is the error's key and whose message is its summary.
- A lib is one module, `src/lib/<module>/<name>.ts`, exporting the names listed.
- Each part has one test file under `tests/`, mirroring its source path.

## Project: tasks_app

A small task tracker used to trial int lang.

Server-render by default. Plain English, sentence case.

Stack: TypeScript, Node.js 22, Next.js 15, pnpm.

## Module: tasks

Creating, listing and completing tasks.

Every tasks page uses the shared page shell: a heading, then the content.
Show dates as YYYY-MM-DD.

## lib store

In-memory record storage keyed by id; empty after a restart.

- Exports: `insert`, `all`, `find`, `save`

Expectations:

- newest_first: all returns records newest first by created_at
- unknown_id: find returns null for an unknown id

## lib dates

Date formatting for task pages.

- Exports: `formatDate`

Expectations:

- iso_date: formatDate returns the date part of a timestamp as YYYY-MM-DD in UTC

## action create_task

Adds a task.

- Input: `{ title: string }`
- Output: `{ id: string, title: string, done: boolean, created_at: timestamp }`
- Uses: store

Expectations:

- stored: a created task is stored with its title, not done

## action list_tasks

Lists all tasks, open ones first, each group newest first.

- Input: `{ }`
- Output: `[{ id: string, title: string, done: boolean, created_at: timestamp }]`
- Uses: store

Expectations:

- order: open tasks come before done ones; within each group, newest first

## action complete_task

Marks a task done.

- Input: `{ id: string }`
- Output: `{ id: string, done: boolean }`
- Uses: store
- Errors:
  - not_found: No task has that id

Expectations:

- marks_done: after complete_task, the stored task is done

## page tasks_index

Lists tasks.

- Route: `/tasks`
- Calls: list_tasks, complete_task
- Uses: dates

One row per task: title, created date and a Complete button for open tasks.
A link to new_task sits above the list.

Expectations:

- listed: every task is listed with its title and created date
- complete: Complete marks the task done and moves it below the open tasks
- empty: with no tasks, the page says 'No tasks yet' and links to new_task
- footer: the footer links to about

## page new_task

A form that adds a task.

- Route: `/tasks/new`
- Calls: create_task

A title field and an Add button. After adding, go to tasks_index.

Expectations:

- adds: submitting a title adds the task and shows the list with it
- required: the Add button is disabled while the title is empty
