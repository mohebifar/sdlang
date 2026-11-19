# Round 7 decisions: packages

Input: `07-packages.codex.md`. No rebuttal: the editor accepted every point. Codex's
critique mostly *removed* package authority, which also reduces how much has to exist
before the language is useful.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| A1 | npm container needs a static, versioned manifest; importing must run no code; packages must not export managed instances | **Accept** | `intentlang` entry in `package.json` (`format`, `entry`, `skills`, `provides`, `migrations`). Catalog exports are types, constants, `extern`s. |
| A2 | Sections still semantically ambiguous: value vs type schema, redeclared `uses`, entry points undefined, route ≠ path, rendering mixes concepts, removed slots, ambient over-grant | **Accept** | Closed vocabulary table; core fields cannot be redeclared; new `exports` section defines entry points; explicit route mapping; `rendering` dropped from the example; ambient imports are exact specifiers. |
| A3 | Variance too broad: records need exact rules; compare projected values; entities need read/write/storage dimensions; behaviour is separate | **Accept** | Variance only for core action signatures; package `@interface` fields invariant; entity compatibility moved to round 8; unknown is incompatible. Closes the "who classifies compatibility" gap without classifier code. |
| A4 | Relations, checks, generators have different authority; global generator selection; self-granted writes; purity needs an operational definition | **Accept** | Relations fixed in core for v0; generators attached to producer kinds; writes come from the CLI envelope; deterministic = runner-supplied input closure, blocked otherwise. |
| A5 | Capability names are a namespace contract | **Accept** | `provides` alias map in the manifest replaces a `capability` statement form. |
| A6 | `spec/_packages/` sits inside the module tree; local packages need snapshots; docs skills are guidance only; bare capabilities need explicit grants | **Accept** | Local packages live outside `spec/`; pinned by content snapshot. |
| B | Exact v0 kind vocabulary; cut `sort`, custom relations, classifiers, generic variance, `accepts`/`collects` (until contributions ship), generator `handles`/`writes` | **Accept** | |
| C | Omit `unit`; make local kinds cheap and honest about missing conformance | **Accept** | Editor's `unit` proposal withdrawn. |
| D | Lock pins the executable closure; static evaluation vs isolated execution; upgrades compare two interpretations; declarative migration maps; no inferred deletion | **Accept** | Execution policy table and upgrade rules in SPEC.md. |
| E | Milestone needs one hard-coded adapter, not an ecosystem; validate the packed artifact in a clean project; adapter-specific acceptance cases | **Accept** | Core is not a package. Acceptance cases folded into the milestone in the final pass. |
