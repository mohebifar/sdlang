# The language

A tour of `.sd` files. The [specification](spec.md) has the full rules and the
[grammar](grammar.ebnf) the exact syntax.

## Declarations

Everything is a declaration: a kind, a name and a body of named members.

```sd
export action complete_task {
  summary: "Marks a task done"
  input: { id: string }
  output: { id: string, done: boolean }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
  expect {
    marks_done: "after complete_task, the stored task is done"
  }
}
```

- The **address** is the module path plus the name: `tasks.complete_task`. Members have
  addresses too: `tasks.complete_task#expect.marks_done`. Renaming a declaration is an
  update, not a delete and a create, as long as you record it (see [Moves](#moves)).
- The **kind** decides which files the declaration owns. Core kinds include `action`,
  `lib`, `entity` and `rule`; packages add others, such as `nextjs.page`. The current
  release implements `project`, `stack`, `use`, `module`, `action`, `lib` and
  `nextjs.page` end to end; entities and rules parse and lower, but not all of their
  behaviour is built yet.
- **`expect`** members are the requirements. Each becomes something to test and review,
  and evidence is recorded per member.
- **`summary`**, **`prompt`** and **`hint`** are prose for the agent. A `prompt` says what
  to build; a `hint` is advice that doesn't change what counts as correct.

## Edges

References between declarations are typed edges:

```sd
import nextjs

export nextjs.page tasks_index {
  route: "/tasks"
  summary: "Lists tasks"
  calls: [list_tasks, complete_task]
  uses: [dates]
  prompt: """
    One row per task: title, created date and a Complete button for open tasks.
    A link to @{new_task} sits above the list.
  """
}
```

- `calls`, `uses`, `reads`, `writes` and `requires` name other declarations. Each relation
  decides which changes travel along it and in which order work runs. When
  `complete_task`'s input changes, `tasks_index` is affected because it calls it. Edges
  guide planning and point agents to related code; they don't restrict imports.
- `@{new_task}` inside prose is a **mention**. It is checked like any other reference, and
  renaming `new_task` updates it.

## Types

Inputs, outputs and entity fields use a small type language:

```sd
export entity Workspace {
  summary: "A team's shared space"
  fields {
    id: uuid @id
    name: string @max_length(80)
    slug: string @unique
    owner: ref authentication.User
    created_at: timestamp = now
    archived_at: timestamp?
  }
}
```

Builtins include `string`, `number`, `integer`, `boolean`, `uuid`, `email`, `url`,
`timestamp`, `date` and `json`. `T?` is optional, `[T]` a list, `{ a: T }` a record,
`A | B` a union, and `ref X` a reference to an entity. Annotations such as `@unique` add
constraints, and `= now` is a default meaning "at creation time".

## Modules and visibility

A directory under `spec/` is a module, and file boundaries inside it carry no meaning. An
optional `module.sd` holds the module's header:

```sd
module {
  summary: "Creating, listing and completing tasks"
  prompt: """
    Every tasks page uses the shared page shell: a heading, then the content.
    Show dates as YYYY-MM-DD.
  """
}
```

Prompts are inherited: project, then module, then declaration. Declarations are private
to their module unless marked `export`. `import tasks` brings in a module and
`import nextjs` a package.

## Packages

A package provides kinds, path templates, checks, generators and skills. Projects choose
providers by role:

```sd
use "better-auth" for authentication
use "drizzle" for database { dialect: "postgres" }
```

A package is a directory with a `package.sd` header and its spec sources:

```sd
package ui_sdk {
  entry: "./spec/index.sd"
  provides {
    ui {
      button: Button
      form: Form
    }
  }
}
```

The header gives the import name and points to the package's files. Where the package
came from and which version it is live in `sd.lock`, so the same directory works from npm,
git or a local path. A team's own kinds go in a local package such as
`spec-packages/acme/`. The Next.js adapter is in [adapters/nextjs](../adapters/nextjs).

## Rules

A rule applies requirements to every declaration a selector matches:

```sd
rule app_requires_session {
  applies_to: select nextjs.page where route matches "/app/**"
  expect {
    redirects {
      text: "unauthenticated visitors are redirected to the login page"
      verify: "tested"
    }
  }
}
```

## Verification policy

`verify` says what must pass before a requirement counts as met: `"tested"` (tests and a
review), `"judged"` (a review) or `"accepted"` (a person's approval). It can be set on the
project, a declaration or a single requirement.

## Moves

A rename keeps its identity, its state and its evidence when it is recorded:

```sd
moved tasks.sign_in -> auth.sign_in
```

Without a `moved` statement, the old address is planned as a delete and the new one as a
create.
