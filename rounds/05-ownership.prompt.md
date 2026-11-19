# Round 5: ownership

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read `SPEC.md`: "Identity" (output slots), "Edges and relations", "Plan entries",
"Transactions", "Delete", "Plan binding, journal and amendments", and "Drift, hand-written
code and shared files". REVIEW.md flaw 10 (a feature spans page, server action,
components, styles, tests; the node-to-outputs mapping beyond one file is unspecified) and
the self-review's "Drift and brownfield" score of 6 are the triggers.

## Editor's draft positions (attack them)

1. **Output slots.** A kind declares named slots and creation roots:
   ```spec
   export kind page {
     fields { route: route_path @interface }
     outputs {
       page: "src/app/${route}/page.tsx"
       tests: "tests/app/${route}/page.test.ts"
     }
     creates {
       parts: "src/app/${route}/_parts/"
     }
   }
   ```
   Slot files exist from `create` on. Files the agent creates under a creation root are
   registered in state as `#outputs.parts.<relative path>` and owned from then on. Files
   outside slots and roots cannot be written.
2. **Granularity.** v0 owns whole files. Packages should pick conventions that give each
   declaration its own files (one action per file), so whole-file ownership rarely bites.
   Symbol ownership comes later through language adapters: an owned region is a top-level
   symbol in a file (`{ path, symbol: "signIn" }`), hashed as a normalised AST. The shared
   frame of a file (imports, module-level directives) is owned by the file's *host*
   declaration; other declarations may add imports their symbols need, and frame drift
   is attributed to the host.
3. **Contributions.** A shared file has exactly one owner declaration. The owner's kind
   declares what it accepts; contributors target an owner instance by name:
   ```spec
   nextjs.nav main_nav {
     summary: "Top navigation"
   }

   nextjs.page login {
     route: "/login"
     contributes {
       nav: main_nav { label: "Sign in", to: self, when: "signed out" }
     }
   }
   ```
   `main_nav { ... }` is checked against `nextjs.nav`'s accepted contribution schema. The
   member `auth.login#contributes.nav` is the identity. A new `contributes` relation runs
   from contributor to owner; the owner consumes the set of contributions. Preferred
   materialisation: the package's generator writes contributions deterministically into a
   registry file (`src/generated/nav.ts`) owned by the owner, and the owner's hand- or
   agent-written code reads it. Then a contribution change is generator work, with no agent.
4. **Special files.** `package.json` and the lockfile are owned by the `project`
   declaration and maintained only by generators driven by `stack` and `use` (the package
   manager's own commands). An agent that needs a new dependency must return `blocked`
   asking for a `use`. Migration files are append-only: each is owned by the entity that
   produced it and frozen once its transaction publishes. Config files nobody owns
   (`tsconfig.json`, `next.config.js`) are unmanaged; agents cannot write them; a task that
   needs to returns `blocked`.
5. **Formatting noise.** Outputs are hashed after running the project formatter, so
   formatter-only edits never register as drift.

## Questions

A. Attack each position with concrete failure scenarios: a page that needs a new shared
   component used by two pages; a route rename moving files that other code imports; two
   declarations in one transaction both wanting to add the same import to a shared frame;
   an agent creating a file under a creation root that another declaration later needs; a
   contribution whose owner is deleted; contribution ordering (nav item order) and
   conflicts (two pages claiming the same nav slot).
B. Is "one owner plus contributions" enough for middleware, route registries, i18n
   message catalogs, database schema indexes, and dependency injection containers? What
   about files that are genuinely co-edited by many declarations with no natural
   structured contribution (a global CSS file)?
C. Symbol ownership: is the host-frame model workable? What breaks with re-exports,
   default exports, decorators, or generated code inside a file? Would you ship it in v0,
   later, or never?
D. How does ownership interact with drift detection and adoption of brownfield code
   (`spec import`)? What does `import` do for a file that already serves three would-be
   declarations?
E. Propose the ownership section of the state record as JSON, with one example covering a
   slot, a created file, a contribution and a registry file.

End with a prioritised list of changes.
