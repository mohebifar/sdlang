# Round 14 decisions: independent rescoring

Input: `14-rescore.codex.md`. Scores: 8, 7, 7, 8, 8, 6, 6, 5, 7 (up from round 13 in
architecture, agent interop, simplicity and team workflow). R13.2 and R13.6 judged fixed;
R13.1, R13.3, R13.4 and R13.5 partly fixed. Every cited defect was checked against the files
and is real. All are accepted.

| # | Defect | Decision | Change |
| --- | --- | --- | --- |
| 1 | Apply excludes all of `.spec/` from checked inputs; Currency excludes named categories and omits the applied store | accept | One definition in Currency (nothing under `.spec/`); Apply refers to it. |
| 2 | Reconciliation prose and plan schema still need both parents and the merge base | accept | Inputs are the merged spec, the applied store and the saved record versions; no commit is read. `binding.reconcile.records` lists base (nullable), current and incoming digests. A source moved to two targets becomes a compile error, so a clean textual merge cannot hide an identity clash. |
| 3 | `lib`'s new required test slot contradicts acquiring one existing file | accept | `acquire` claims only existing files; an empty required slot gets an `update` in the next plan. Walk-through updated. |
| 4 | Graph handle's viewpoint unstated during partial progress | accept | The graph is the applied graph (published declarations plus this transaction's). |
| 5 | Run-input digest has no producer a check could follow | accept | The CLI computes it and passes it in; the result echoes it. |
| 6 | Instance completeness not represented; inventory digest undefined | accept | Expected cases carry instance keys; inventory is the digest of sorted `[case, key]` pairs, checked when selection is complete. |
| 7 | Purpose-specific input fields optional; checker misses several mutations | accept | Schema `allOf` conditions by purpose and scope; checker rejects five bad inputs and three more result mutations. |
| 8 | `no_client_secrets` is only a name | accept | Defined, with `page_default_export`, in a comment in the milestone adapter; SPEC says a rule's meaning is package documentation and changing it needs a new version. |

Not done: O.2 (fan-in placement), O.3 to O.5, O.7 (core-only example), and O.8 (pinning
the trial's configuration and margin). Codex's view, which the editor shares: after this
cleanup, run the trial rather than more broad design rounds. No rescoring after these
changes.
