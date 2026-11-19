# Self-review of SPEC.md (draft v0)

Written on 2026-11-18 by the author of the draft, so treat it as a biased first pass.
Overall: about 6 out of 10 as a design draft. The architecture holds up, the grammar is
not yet precise enough to write a parser from, and nothing is validated.

## Scores

| Area | Score | Why |
| --- | --- | --- |
| Architecture (graph, identity, state, plan actions) | 7 | The strongest part. Stable addresses, the interface/body hash split, per-node state and `verify` versus `update` are sound ideas borrowed from tools that work. |
| Drift and brownfield | 6 | The adopt/absorb/overwrite model is good. Mapping nodes to code is mostly hand-waved: whole-file ownership is too coarse, and "contributions" for shared files is a sketch. |
| Verification | 6 | The levels and statuses are right, and the doc admits the weak point. AST conformance checks put a heavy burden on package authors. |
| Grammar and syntax | 5 | Readable, but it is a set of examples, not a grammar. |
| Agent interop | 5 | The work order is a reasonable contract. The result format, retries, the "blocked" path and context assembly are described only in prose. |
| Package ecosystem | 4 | Everything depends on packages nobody has written. The biggest practical risk. |

## Concrete flaws

1. **`expect` ids are positional.** Inserting a line renumbers e1 to e4 and breaks the test mapping. This violates the identity rule the doc insists on elsewhere.
2. **There is no bug-fix flow.** If the code is wrong but the spec is right, nothing in the plan changes, so nothing triggers work. That is the most common kind of change.
3. **Prose changes can slip past propagation.** If an action's prompt changes its behaviour, the interface hash stays the same and callers are never re-verified.
4. **Several constructs are used but never defined:**
   - annotations (`@id`, `@unique`, `@max`, `@default`, `@interface`);
   - the rule selector language (`nextjs.page where route matches "..."`);
   - how `@{}` references resolve (relative versus qualified, self-reference);
   - `use ... { options }`;
   - the inline `nav.item {...}` form in `contributes`, and where `nav` comes from;
   - separator rules (newline versus comma), comments, string escapes;
   - `${route}` interpolation in kind definitions versus `@{ref}` in prose;
   - string-encoded versions such as `"nextjs@15"`.
5. **Kind definitions mix two things.** Field schema and metadata (`outputs`, `checks`) sit in one block.
6. **Runtime state is not modelled.** The plan knows a migration was generated, not whether any database has run it.
7. **Scope collisions are possible.** Capability names (`ui`, `authentication`) are global and can clash with module names (`auth`) or packages.
8. **`verify` is fuzzy.** Who decides whether prose still conforms, and what does it cost when a project prompt edit marks every node?
9. **`delete` says "references cleaned up"** without saying how.
10. **A feature usually spans several files** (page, server action, components, styles, tests). The node-to-outputs mapping beyond one file per node is unspecified.

## Fixes to make first

1. Named expectations with stable ids.
2. A `repair` action for "code does not match an unchanged spec".
3. A real grammar (EBNF) with lexical rules, scoping and reference resolution, and an annotation system.
4. The agent protocol as schemas: work order, result file, retry and "blocked" semantics.
5. Separate schema from metadata in kind definitions.
6. A precise ownership model for multi-file nodes and shared files.

## Why build this at all

The core benefit is turning "ask an agent to change the code and hope" into a change that can be reviewed, scoped and verified. What needs a language, as opposed to Markdown specs plus discipline, is the computable part: identity, impact propagation, ownership and enforcement. The payoff grows with codebase size, team size and the number of agents working concurrently. For a solo developer on a small project the overhead probably outweighs the gain.

## Prior art checked on 2026-11-18

None of these combines typed importable spec code, framework-specific kinds, and a Pulumi-style state file with plan and apply. Re-check before relying on this; the space moves quickly.

- **CodeSpeak** (Andrey Breslav): structured English/Markdown specs compiled by LLMs; shows requirement-level changes before a build.
- **Tessl**: "spec-as-source" framework plus a registry of specs describing libraries for agents.
- **OpenSpec**: Markdown workflow with delta specs against a baseline.
- **GitHub Spec Kit, Kiro**: spec-first workflows; code is generated once and then maintained by hand.
