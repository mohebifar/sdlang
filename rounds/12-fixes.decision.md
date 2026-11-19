# Round 12 decisions: fixes from the rescoring round

Inputs: `11-scoring.codex.md` and `11-scoring.editor.md`. Codex found defects the editor's
rescore had missed (an inherited-prompt edit planned as updates by the reference
canonicaliser; a fingerprint that would make the example plan inapplicable; a work-order
hash that disagreed with `hashes.json`; a milestone check set that contradicted its own
adapter). All points were accepted; none contested.

| Area | Asked for | Change |
| --- | --- | --- |
| Architecture | Authored vs inherited precedence; reference derivation must plan review, not update, for an inherited-prompt edit | `behaviour` now holds only the declaration's own normative prose; inherited normative context is in `context`; precedence table in Propagation; `canon.diff_graphs` follows it; the checker asserts that a module-prompt-only edit yields review obligations for three declarations and no updates. |
| Architecture | Canonical byte contract (numbers, NFC) with hash vectors | Rules written; `canon.py` NFC-normalises and fixes number forms; `examples/hash-vectors.json` with an NFC pair, checked. |
| Drift | Fingerprints must be observed bytes | Defined; the plan fixture's drifted file now carries its drifted bytes. |
| Drift | Publication exception for `acquire` | `acquire` publishes after compile, ownership and build checks; its requirements stay `unverified` and become obligations in the next plan. |
| Verification | Implicit framework inputs in run closures; whole-tree fallback | Kind section `inputs` (paths or globs, owned or not); project-scope checks bind to the whole tree, so unmanaged edits stale them and create obligations. |
| Verification | Consequences of `unsupported` and `error` | Rules are required unless declared advisory; `violated` escalates, `unsupported` blocks publication where the check is needed, `error` is infrastructure. |
| Agent interop | Protocol example hashes must equal the graph's | Work-order reference, inherited-context and contract hashes are derived from `canon.py`; the checker compares them with the graphs. |
| Agent interop | Ambient grants must match the adapter | Exact subpaths `next/link`, `next/navigation`, `react`, `react/jsx-runtime`. |
| Agent interop | Normative cross-document validation table | Added to PROTOCOL.md, marking the rules the checker exercises. |
| Packages | Milestone check set vs adapter | Milestone runs all four adapter checks, with conformance limited to ownership and imports. |
| Packages | Generator deletion and relocation | `after` may be null; envelope has `delete` and `move`; output files carry an `op`. |
| Packages | Route-to-path mapping in the contract, not comments | Core substitution and normalisation rule for `route_path`, whose v0 values are static only. |
| Packages (editor) | The `binding` form specified and exemplified | Full vocabulary (`options`, `outputs`, `creates`, `collects`, `retains` with producers, `generator`, `observer`, `checks`, `skills`); `observer` definitions; `examples/drizzle-binding.spec` parsed by the checker. |
