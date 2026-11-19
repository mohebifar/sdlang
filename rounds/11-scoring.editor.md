# Round 11: editor's independent rescore

Written and committed before running Codex's rescoring, without seeing it. Same scale and
calibration as round 10: 9 means "hand to a team to build the first milestone with only
minor questions". These are scores of a design on paper.

| Area | R10 editor | R10 Codex | Editor now | Why |
| --- | --- | --- | --- | --- |
| Architecture | 8 | 7 | **9** | The graph contract is now executable: complete before/after graphs, a reference canonicaliser with SCC hashing, computed hashes, a plan schema, and a checker that derives the plan's change set and ordering from the graphs. Implementation impact covers every output change. Remaining minor questions: the canonicaliser hard-codes interface fields per kind, and compatibility classification (`reader-compatible`) is asserted in the plan fixture rather than computed. |
| Drift and brownfield | 8 | 8 | **9** | `acquire` closes the gap between import and agent authority; the walkthrough shows envelope and failure behaviour; adoption's affected evidence and `absorb`'s output are defined. Whole-file ownership is a stated limit, not a gap. |
| Verification | 8.5 | 8 | **9** | Run input closures and the invalidation table answer "which evidence is current after which change", with a two-transaction example from the worked plan; the milestone measures coverage-review quality directly. |
| Grammar and syntax | 9 | 8 | **9** | Recursion rule, selector type matrix, `implements` production, checker aligned with the EBNF on the cases found. |
| Agent interop | 8.5 | 8 | **9** | Every protocol document, the generator protocol and the MCP tool calls have schemas and validated examples with recomputed bindings; review orders carry the same context contract as work orders; regression is per subject. |
| Package ecosystem | 7 | 7 | **8.5** | The milestone adapter is now written in the language and parsed; `implements` explains how adapters materialise core kinds; the manifest has a schema. Held back because the worked example's `database` binding (its `collects` and retained migration root) uses the `binding` form, which is only tabled, not specified and exemplified with the same precision as `kind` and `implements`. What makes it 9: the drizzle binding written out like the nextjs adapter, with the `binding` and `retains` vocabulary fully defined. The ecosystem risk itself is outside what a design controls. |
