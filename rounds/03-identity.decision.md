# Round 3 decisions: identity

Inputs: `03-identity.codex.md`, rebuttal `03b-identity-rebuttal.codex.md`.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| A1 | Sub-addresses must be schema-declared collections, keyed by name only | **Accept** | Table of core collections (`expect`, `errors`, `input`, `output`, `fields`, `outputs`, `contributes`) added. Keys are unique per collection. Grammar: `'#' Word '.' Word ('.' Word)*`. |
| A1' | `PackageAddress` / `CapabilityAddress` in `moved` | **Reject** (Codex conceded in 3b) | Package export continuity is package-authored migration data; capability identity is its name. Canonical package symbols live only in graph JSON and state. |
| A2 | Text edits must invalidate coverage, not just keep the binding | **Accept** | Evidence split into coverage (reviewer judgement on a requirement revision) and run (pass on a tree). `tested` needs both current. This closes a hole the draft had: an unchanged passing test claiming a changed requirement. |
| A3 | `moved` composition table; `moved module`; rename_field lowering; cross-declaration moves | **Accept** | Written into SPEC.md "Moves". Field rename fails rather than falling back to drop-and-add. |
| A4 | Never claim a move is safe to remove; reserve moved sources | **Accept** | `moved` kept indefinitely in v0. Identity epochs deferred until pruning is needed. |
| A5 | Rule requirements as `(target, requirement)` pairs; rules get named `expect` members | **Accept** | `require`/`check` replaced by `expect { key { text, verify } }` on rules. |
| A6 | Templates: argument stability is not meaning stability; overrides and detachment unsolved | **Accept: remove live templates** | Replaced by `spec scaffold`, which writes ordinary declarations. Grammar `TemplateDef`/`Expansion` and keywords removed. |
| B | Identities for action members, errors, catalog symbols, capabilities, outputs (slot not path), contributions, extern | **Accept** | Output slot identity means a route change moves a file instead of deleting it. Constraints/indexes naming deferred to round 8 with entities. |
| B' | Structured `consumed` entries | **Accept** | One entry per (relation, target, projection). |
| C | Scaffold instead of templates | **Accept** | See A6. |
| D | `expect` as a section of named blocks with `verify` and test locator | **Accept, refined in 3b** | Short form `key: "text"` and long form `key { text, verify, test }` lower to one canonical member (Codex conceded the shorthand). Error wire `code` defaults to the key; a key rename changes the code unless pinned. |
| D' | `verify` policies are not a total order | **Accept** | Satisfaction table in Verification; `accepted` satisfies only `accepted`. |
| 3b-2 | Graph manifest for context declarations | **Reject; refined requirement accepted** | No shared manifest. Instead `provenance.commit` locates the applied spec revision so the planner can classify changes and show diffs; unreachable commit means conservative (incompatible) invalidation. |
| merge | Three-way identity reconciliation after branch merges | **Accept** | Described in "Moves"; details of the merge-check command go to round 4. |
