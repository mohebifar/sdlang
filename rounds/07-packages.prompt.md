# Round 7: packages

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read `SPEC.md`: "Packages", "Definition kinds", "Edges and relations", "Propagation"
(compatible vs incompatible changes), "Outputs", "Contributions", "Toolchain and retained
artifacts", "Annotations", "Core symbols". REVIEW.md scored the package ecosystem 4/10:
"Everything depends on packages nobody has written. The biggest practical risk." Flaw 5
(kind definitions mix schema and metadata) is only half fixed: the syntax now has sections,
but their semantics are not defined.

## Editor's draft positions (attack them)

1. **What a package contains.** An npm package with an `intentlang` field in
   `package.json` pointing at its spec sources. Its `.spec` files may use five
   package-only statement forms: `kind`, `annotation`, `relation`, `check`, `generator`,
   plus ordinary exported catalog declarations. Executables (generators, check runners,
   conformance analysers) are ordinary JS in the package, referenced by the definitions.
2. **Kind definition sections**, each with fixed semantics:
   ```spec
   export kind page {
     sort: "managed"
     fields {
       route: route_path @interface
       rendering: "static" | "server" | "client" = "server"
       uses: [ref ui.component] @relation("uses")
       calls: [ref action] @relation("calls")
     }
     outputs { page: "src/app/${route}/page.tsx" }
     creates { parts: "src/app/${route}/_parts/" }
     ambient { imports: ["next", "next/*", "react"] }
     checks: [typecheck, build, conformance]
   }
   ```
   `fields` is the only schema; everything else is metadata the compiler validates against
   a fixed core vocabulary. `accepts { ... }` and `collects: select ...` define contribution
   owners.
3. **Compatibility by variance, not hand-written classifiers.** Each `@interface` field
   declares a direction: `@interface(in)` (what callers supply, e.g. an action's `input`),
   `@interface(out)` (what dependents receive, e.g. `output`, entity fields), or plain
   `@interface` (invariant, e.g. `route`). The core type system decides compatibility
   structurally: an `out` change is compatible if the new type is a subtype of the old
   (adding an optional field, narrowing a union), an `in` change if it is a supertype
   (adding an optional input), an invariant change never is. Kinds need no classifier code.
4. **Relations, checks, generators as definitions:**
   ```spec
   export relation renders {
     consumes: "public"
     observes: "behaviour"
     orders: true
     grants: "entry_points"
   }

   export check typecheck {
     run: ["pnpm", "tsc", "--noEmit"]
     scope: "project"
   }

   export generator migrate {
     handles: select entity
     changes: ["create", "update", "delete"]
     run: ["node", "./gen/migrate.js"]
     writes: ["outputs.schema", "retained.migrations"]
   }
   ```
   A generator is a process that receives the declaration diff as JSON and must be a pure
   function of its declared inputs; the CLI validates its output like an agent's.
5. **Capabilities as declared interfaces.** A provider package declares
   `capability authentication { exports { User: entity, session: type, skills: skill } }`.
   No interchangeability promise in v0; the declaration only fixes which names exist.
6. **Bootstrapping without an ecosystem.**
   - Project-local packages: `spec/_packages/<name>/` holds kind definitions a team writes
     for itself, imported with `import package "./_packages/acme-kinds" as acme`.
   - A generic core kind `unit` for anything no package covers: a managed declaration whose
     outputs and checks are listed explicitly in the declaration itself
     (`unit cli { outputs { main: "src/cli.ts" } checks: [...] }`), with no conventions,
     no conformance, and only `uses`/`calls` edges.
   - `use "pdf-lib" for pdf` with no spec package yields a bare capability (name only), and
     `spec skill add pdf --from <docs>` stores its documentation as a project skill.
   - v0 ships `@intentlang/core` (entity, action, rule, unit) and `@intentlang/nextjs`.

## Questions

A. Attack each position with concrete failure scenarios. Does variance-based compatibility
   hold up for entities (where a new non-null column with no default is incompatible even
   though it is "adding a field") and for behaviour (it says nothing)? What does "pure
   function of declared inputs" mean for a generator that shells out to `drizzle-kit`?
B. What must the core vocabulary of kind metadata be, exactly? List every section and field
   a kind may declare, with its type and semantics, and cut anything not needed in v0.
C. Is `unit` a good idea or a hole that lets projects avoid writing kinds forever? If a
   hole, what replaces it?
D. Package trust and versioning: generators and checks run package code. What does the
   lock pin, what is sandboxed, and what happens to existing state when a package upgrade
   changes a kind's schema (a field renamed or removed)?
E. Bootstrapping: given that nobody has written packages, what is the smallest package
   surface the first milestone needs, and how should the `nextjs` package be validated?

Give concrete syntax. End with a prioritised list of changes.
