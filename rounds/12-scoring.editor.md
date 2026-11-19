# Round 12: editor's final independent scores

Written and committed before Codex's final rescoring, without seeing it. Calibration note:
in round 11 the editor scored five areas at 9 and Codex then found real defects in four of
them (an inherited-prompt edit planned as updates, a self-contradicting fingerprint, a hash
mismatch between fixtures, a milestone/adapter check conflict). The scores below try to
account for that: each 9 rests on something the checker now verifies, not only on prose.

| Area | R11 editor / Codex | Editor now | Basis |
| --- | --- | --- | --- |
| Architecture | 9 / 8 | **9** | Precedence of authored vs inherited changes is stated and verified on the worked graphs (module-prompt edit → review only); the canonical byte contract has hash vectors including NFC; plan derivation is checked. Residual: compatibility classes are asserted in the plan fixture, not computed by the reference canonicaliser. |
| Drift and brownfield | 9 / 8 | **9** | Fingerprints are observed bytes, so drift blocks only its declaration; `acquire` has an explicit, bounded publication rule; walkthrough and adoption currency defined. |
| Verification | 9 / 8 | **9** | Run closures include implicit framework `inputs`; project-scope checks bind the whole tree; conformance outcomes have defined consequences; invalidation table and reuse rule unchanged. |
| Grammar and syntax | 9 / 9 | **9** | Unchanged except `implements`, `${meta.*}` and the `route_path` value rule; all examples and adapter files parse. |
| Agent interop | 9 / 8 | **9** | Example hashes derive from the canonicaliser and are cross-checked against the graphs; ambient grants match the adapter; one table of cross-document rules, with the checked ones marked. |
| Package ecosystem | 8.5 / 8 | **9** | Both providers the worked example needs are written in the language and parsed (`nextjs-adapter.spec`, `drizzle-binding.spec`); `binding`, `retains`, `observer`, `implements` and the route substitution rule are specified; the milestone and adapter agree on checks; generators handle deletion and moves. The ecosystem risk itself (adapters beyond these two) is outside what a design controls and is stated as such. |
