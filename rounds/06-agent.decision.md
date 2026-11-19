# Round 6 decisions: agent protocol

Input: `06-agent.codex.md`. No rebuttal: every substantive point accepted. The draft
schemas were rewritten; Codex's field-by-field table drove the changes.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| A-bind | Bind every order and reply to exact identities: dispatch id, order hash, input snapshot | **Accept** | `binding` block; `order_hash` is sha256 over canonical JSON minus itself; replies echo it. The checker recomputes it. |
| A-digest | 4-hex digests are display-only | **Accept** | Tagged full digests (`sha256:`, `git:`) in `common.schema.json`. |
| A-subj | Single-declaration orders contradict coordinated (cyclic) transactions | **Accept** | `subjects[]`, each with its own envelope for owner attribution. |
| A-cause | One cause enum + prose → typed `causes[]` | **Accept** | |
| A-repair | `repair` unrelated to cause; needs revision and pre-fix snapshot | **Accept** | Schema conditional: repair required iff a cause is `repair`; `pre_fix_snapshot` and `contract_hash` added. |
| A-spec | "Canonical" text kept local names; diff was a summary | **Accept** | Rendering `source/1` plus a `resolution` map; `diff` is a real unified diff (generated). |
| A-ctx | Context references need hashes, viewpoint, unchanged dependencies, inherited context in behaviour | **Accept** | Example now includes all four catalog contracts and sign_in's inherited normative context; `viewpoint` applied/desired. |
| A-imports | Module-level import grants over-grant | **Accept** | `allowed_imports` lists exports. |
| A-req | Requirements as subject pairs, contract hash, policy provenance, test sets; status is baseline only | **Accept** | Added project `verify` default to the worked example so `policy_source: "project"` is real; core default `judged`. |
| A-checks | `self_check` must not be an arbitrary command | **Accept** | Boolean flag; commands live in pinned check definitions run via `run_self_check`. |
| A-result | `done` → `submitted`; ownership claims are real claims; discriminated regression and blocked unions; bounded untrusted text | **Accept** | |
| A-paths | Result path outside the protocol area; per dispatch | **Accept** | `.spec/run/<dispatch>/`. |
| B-blocked | Blocker is a claim; unsupported blocker is a failed attempt; three resolution paths (spec, plan amendment, environment) | **Accept** | Operational answers never become spec text. |
| B-flaky | Separate infrastructure reruns from coding attempts; classified failures with fingerprints | **Accept** | |
| B-fresh | Automatic fresh restart after two same-check failures loses work | **Accept** | Restart is an explicit recovery action with a checkpoint and reason. |
| B-life | Delivery lifecycle: stdin, timeouts, kill process tree, freeze, exit codes | **Accept** | |
| C | "Required never truncated, else plan-time error" too strict; manifest + fetchable resources; deterministic packing order | **Accept** | Inline set is still never truncated; other required items are fetched in full; packing policy `context-pack/1` with fixed omission order. |
| D | Separate review order and verdict, per-subject, exact-once, three-valued conformance and coverage, regression assessment, context subjects | **Accept** | Two more schemas plus examples. |
| E | `run_self_check` is execution; `read_skill(path)` traversal; add `read_context`, `get_task`, `get_diagnostic`; state the confidentiality and final-tree limits | **Accept** | Tool table in SPEC.md. |
| Spec | "Module in full" contradicts optional siblings | **Accept** | Visibility paragraph rewritten. |
