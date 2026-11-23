# Next.js App Router, as @sdlang/nextjs lays it out

- A page's module is `src/app/<route>/page.tsx`; its default export is the page component.
  Pages are server components unless a file starts with `"use client"`.
- Put client-side parts (forms, anything with state or event handlers) under the page's
  `_parts/` directory and mark them `"use client"`.
- Client code must not read `process.env` except `NEXT_PUBLIC_*` names, and must not import
  modules marked `import "server-only"`.
- Call actions by importing their `default` export from `src/server/<module>/<name>.ts`.
- An action's default export takes its input object and resolves to its output. A declared
  error rejects with an `Error` whose `code` is the error's key and whose message is its
  summary.
- Tests live under `tests/`, mirroring the source path (`tests/app/<route>/page.test.ts`), and
  run under Vitest with its globals. Name each test case after the requirement it covers, in
  its title: `it("[case:filter] returns only done tasks", ...)` covers `...#expect.filter`;
  `[case:<key>:<instance>]` names one instance of a parameterised case. Tests may import
  whatever they need, such as the store, to set up state.
