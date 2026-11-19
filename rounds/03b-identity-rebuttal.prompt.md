# Round 3b: identity rebuttal

Context (read-only): `SPEC.md`, `GRAMMAR.ebnf`, and your critique in
`rounds/03-identity.codex.md`. Answer in Markdown, under 700 words.

The editor accepts most of round 3: schema-declared addressable collections, the `moved`
composition table (chains, cycles, collisions, reserved sources, injectivity),
`moved module a -> b`, coverage assessment separate from execution, output slots
independent of paths, rule applications as `(target, requirement)` pairs, three-way
reconciliation after merges, structured `consumed` entries, and removing live templates in
favour of a scaffold command. Three disagreements; for each, concede, hold or refine.

## 1. No `package` / `capability` addresses in `moved`

You proposed `PackageAddress` and `CapabilityAddress` productions for `moved`. But you also
said consumers must not invent moves inside a package's catalog; export migrations come
from the package author. A capability binding is keyed by its name, which the project
already controls. Editor's position: canonical package symbols (`npm:ui-sdk::input`)
appear in state and in the graph JSON only; `moved` accepts only declaration and module
addresses, plus member paths. Package export renames ship as migration data inside the
package. Is there a case where a project must write a `moved` for a package or capability?

## 2. No separate graph manifest for context declarations

You proposed a graph manifest holding identities and canonical snapshots for output-free
declarations (rules, modules, types). The editor's position: context declarations matter
to the planner only through their effects, and every effect is already recorded per
managed declaration: `context_parts` keyed by context address, and evidence subjects
`(target, requirement)` in the target's record. Their history is git; their renames are
`moved` statements in the spec. A manifest adds a second shared, frequently-conflicting
state file. What concretely breaks without it?

## 3. Short forms for named members

You proposed long-form blocks: `expect { redirects { text: "...", verify: tested } }` and
`errors { name_taken { code: "NameTaken", summary: "..." } }`. Most expectations are one
sentence. Editor's position: every addressable collection accepts both forms, which the
existing grammar already parses (a field or a section):

```spec
action create_workspace {
  errors {
    name_taken: "The workspace name is already in use"
    limit_reached {
      summary: "Free plans allow one workspace"
      code: "PLAN_LIMIT"
    }
  }
  expect {
    slug_unique: "two workspaces never share a slug"
    free_limit {
      text: "a free user's second workspace fails with limit_reached"
      verify: "tested"
    }
  }
}
```

A string value is shorthand for the collection's primary text field (`text` for
expectations, `summary` for errors). An error's wire code defaults to its key. Does the
shorthand create an identity or canonicalisation problem?
