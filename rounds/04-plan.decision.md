# Round 4 decisions: plan semantics

Input: `04-plan.codex.md`. No rebuttal round: the editor accepted every point, with
one refinement (hint as the only forward-only context) that removes a flag instead of
contesting a criticism.

| # | Codex point | Decision | Why |
| --- | --- | --- | --- |
| A1 | Citing a requirement does not prove a change is a repair; existing evidence need not fail first; define "fails first"; honest evidence for untestable bugs; cut `--unspecified` | **Accept** | `repair` is a cause of `update`, not a change type. Work order carries requirement revision, observation and intended correction; regression test must be judged to follow from the requirement and must fail on the pre-fix tree under the same check definition. Untestable cases yield `judged`, never `tested`. `--unspecified` cut: adding an `expect` member is the route for missing requirements. |
| A2 | Observation followed only by checks repeats flaw 3; implementation-only changes need propagation too | **Accept** | Observed (behaviour projection) changes give `verify` + `review`. New `behaviour` projection (prompt, expect, errors text, inherited normative context). Implementation impact: transitive `verify` over ordering relations, checks deduplicated; `verify_scope: "all"` option. Closes REVIEW flaw 3. |
| A3 | Verify and review are independent, both can apply; review has three outcomes | **Accept** | Obligations are a set. Review: conforms / does not conform (with requirement and mismatch) / cannot determine. Review cannot waive tests or yield `tested`. Context inputs classified by meaning (table in Propagation). Closes REVIEW flaw 8 (meaning). |
| A4 | Forward-only normative context is a hole | **Accept, refined** | No forward-only flag. `hint` is non-normative by definition and is the only input that applies to future work only. Cost of a normative context edit is printed (subjects, checks, review batches, contingent updates); batched reviews must return a verdict per subject. Closes REVIEW flaw 8 (cost). |
| A5 | Delete: allow co-deleted references; use old graph; enumerate owned files; owner joins; state the real guarantee | **Accept** | Written into "Delete". Closes REVIEW flaw 9. |
| B1 | Partial application needs an applied graph | **Accept** | Transactions are checked against the applied graph plus the current transaction. |
| B2 | Resumption needs a journal of receipts | **Accept** | `.spec/journal/<plan>.jsonl`; commit trailer carries the transaction id for crash detection. |
| B3 | Amendments are new plan revisions with new bindings | **Accept** | |
| B4 | Weak tests: coverage can be revoked | **Accept** | Coverage judgements depend on observed contracts and effective context as well as text. |
| B5 | Repair on drifted code | **Accept** | Repair takes drifted files as input, preserves unrelated edits, never adopts first. |
| B6 | Merge reconciliation entry point | **Accept** | `spec plan --reconcile`. |
| B7 | Generator failure must not silently become agent work | **Accept** | Classified failure; agent fallback only in a reviewed or amended plan. |
| B8 | Obligation-only transactions; cyclic transactions; delete ordering; frozen blocks; checked-input digest excludes receipts | **Accept** | All written into "Transactions" and "Apply". |
| C | Vocabulary: four changes + none, `replace` as an update strategy, `repair` as a cause, two additive obligations | **Accept** | Tables in "Plan entries". |
