# Round 5 decisions: ownership

Input: `05-ownership.codex.md`. No rebuttal: every point accepted. Two editor choices
made while applying them are noted.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| Inv | Separate ownership, dependency, contribution, evidence; one file owner | **Accept** | Stated as the ownership invariant. The ownership index is derived, not stored. |
| A1a | Created part becomes shared: explicit transfer | **Accept** | Editor choice: transfer is a cross-declaration member move (`moved auth.login#outputs.login_form -> ui.AuthForm#outputs.component`), reusing round 3's machinery instead of a new operation. |
| A1b | Created outputs need stable keys, not path-derived identity | **Accept** | Agent registers a key; root and relative path are recorded separately. |
| A1c | Route rename must move created files and repair imports | **Accept** | Created files move with their root; plan lists moves and collisions. |
| A1d | Creation root grants no authority over existing files | **Accept** | |
| A2 | Whole files in v0; owner integrates shared-frame requests | **Accept** | |
| A3 | Contribution aggregation contract: owner deletion, ordering, exclusive keys, referenced projections, import grants, reverse aggregate input, generation from applied graph | **Accept** | All written into "Contributions". Editor addition: `collects: select ...` for implicit registries (schema index), reusing rule-selector restrictions. |
| A4a | `project` owns nothing, so it cannot own the manifest | **Accept** | Editor choice: `stack` becomes the managed toolchain owner (manifest, lockfile, declared config files); `use` contributes to its aggregate input. A `use` binding becomes managed when its package declares integration outputs. |
| A4b | Unowned config blocks adoption | **Accept** | Config files are owned by `stack` through package-declared slots. |
| A4c | Migrations must outlive entity deletion | **Accept** | Retained artifacts, owned by the producing binding, immutable, corrected by later migrations. |
| A4d | `blocked` for dependencies must name a concrete amendment | **Accept** | Folded into round 6's blocked schema. |
| A5 | Exact bytes for binding and evidence; formatter digest only classifies | **Accept** | Two digests per output. |
| B | Owner + contributions is an authority model; deterministic materialisation needs a domain contract; global CSS gets one owner and an integration task | **Accept** | |
| C | Symbol ownership later, only with an adapter-mediated editing protocol | **Accept (defer)** | |
| D | `spec import` proposes an ownership mapping; integration owner / extern / extraction | **Accept** | Also: hand-edited generated registry is restored, not adopted. |
| D' | Drift-vs-frozen wording contradiction | **Accept** | One rule: drift blocks managed, is reported for frozen. |
| E | Ownership state JSON; generator identity invalidates generated artifacts | **Accept** | State record example updated; generator-definition change now regenerates its outputs. |
| P1-7 | Milestone should exercise extraction, relocation, occupied paths, hand-edited registries, conflicting contributions | **Defer** | Contributions and registries are outside the first milestone; extraction and relocation added to the trial list in the final pass. |
