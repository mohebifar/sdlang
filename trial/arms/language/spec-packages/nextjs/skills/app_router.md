# Next.js App Router, as @intlang/nextjs lays it out

- A page's module is `src/app/<route>/page.tsx`; its default export is the page component.
  Pages are server components unless a file starts with `"use client"`.
- Put client-side parts (forms, anything with state or event handlers) under the page's
  `_parts/` creation root, mark them `"use client"`, and register each one in your result's
  `outputs`.
- Client code must not read `process.env` except `NEXT_PUBLIC_*` names, and must not import
  modules marked `import "server-only"`.
- Call actions by importing their `default` export from `src/server/<module>/<name>.ts`; that
  is the only thing a `calls` edge lets a page import from the action.
- An action's default export takes its input object and resolves to its output. A declared
  error rejects with an `Error` whose `code` is the error's key and whose message is its
  summary.
- Tests live in `tests/app/<route>/page.test.ts` and run under Vitest. Tag each test with its
  case in the title, `[case:<case>]`, or `[case:<case>:<key>]` for one instance of a
  parameterised case, and bind the cases to requirements in your result's `tests`.
