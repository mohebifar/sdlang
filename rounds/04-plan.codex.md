Keep `verify` and `review`, but make them **independent obligations that can both apply**. Treat `repair` as a reason for an `update`, and reject forward-only changes to normative context.

The governing invariant should be: **a transaction publishes only after every affected obligation is satisfied on its final candidate inputs, at the evidence strength required by the spec.** Passing tests, reviewer approval and accepting a baseline are different claims.

A. The five positions need these changes.

1. **Repair: citing a requirement does not establish that a change is a repair.**

   Suppose `expect.generic_error` requires one generic error for invalid credentials. An agent can make a failing test for that requirement, fix it, and also remove rate limiting. The cited requirement and red/green test establish the targeted correction; they do not establish preservation of other behaviour. A broad expectation such as “login works” makes the loophole larger.

   Require the repair work order to identify the existing requirement revision, the violating observation, and the intended correction. Independently review whether the new regression test actually follows from that requirement. Preserve all other applicable requirements and policies. This reduces abuse; it cannot prove its absence.

   Do not require *existing evidence* to fail before repair is permitted. The common case is precisely that existing tests miss the bug. A new regression test can expose a violation of an existing requirement while the old suite remains green. Conversely, missing or stale evidence is not evidence of incorrect code, and a failing runner is not necessarily a product defect.

   “Fails first” needs a precise meaning: run the same independently approved test against the bound pre-fix implementation and the final candidate, with the same relevant fixtures, configuration and check definition. Record both results. A failure caused by importing a helper introduced only by the fix is not a demonstrated behavioural regression.

   For layout bugs, permit before/after visual evidence and a separate judgment. For races, prefer a controlled schedule or reproducible interleaving; one failed run followed by one successful run is weak evidence. When automated reproduction is unavailable, record that limitation and use an allowed evidence method. A `tested` policy must remain unsatisfied if only judgment is available.

   Cut `--unspecified` from v0. Adding a named expectation is already the route for discovering an omitted requirement. The proposed metric otherwise measures willingness to use the escape hatch, rather than spec completeness.

2. **Prose propagation: observation followed only by checks repeats flaw 3.**

   An action changes from throwing on invalid credentials to returning `null`; its existing signature permits both. The caller’s tests cover successful login only. Its `verify` passes, while its error handling is now wrong.

   An observed behavioural change therefore needs checks **and semantic assessment wherever current evidence does not cover the changed assumption**. “Never an update” can describe the initial planning decision; it cannot prohibit escalation after discovering caller nonconformance.

   The observed contract must also include effective requirements inherited from context. Changing the module rule “sessions expire after one hour” changes an action’s behaviour without changing its own `prompt`, `expect` or `errors`.

   There is another hole: a repair changes implementation behaviour while leaving every spec projection unchanged. Callers still need invalidation. Track implementation impact separately from contract changes, using runtime relations and check dependencies. A conservative v0 can traverse the affected runtime dependency closure. Stopping at the first unchanged intermediate projection misses a caller-of-a-caller whose runtime results changed.

   Propagation storms are real. A shared action’s prompt may contain both public semantics and implementation instructions; observing the whole prompt makes both equally expensive. Keep implementation guidance in `hint`, expose contract members where useful, and deduplicate identical checks on the same candidate. Do not suppress genuine impact merely because it is large.

   Finally, observing another declaration grants a read-context obligation: callers’ work orders must receive the relevant behavioural projection and its diff. Today cross-module visibility supplies only public projections ([context budgeting](/Users/mohebifar/dev/intentlang/SPEC.md:461)).

3. **Two obligations: keep the distinction, remove the either/or rule.**

   A module prompt can change both layout and password-validation behaviour. It needs review of the layout and execution of the relevant tests. “Context produces review; everything else produces verify” loses one of those obligations.

   Classify changed inputs by meaning, rather than by whether their declaration is called context. A changed check definition needs execution; a changed output convention needs file reconciliation; a provider swap can require implementation work; normative prose needs semantic assessment. These cases already coexist inside the current `context` projection.

   Review needs three outcomes: **conforms, does not conform, unable to determine**. A binary reviewer otherwise has to turn uncertainty into approval or unnecessary implementation work. Nonconformance must name the requirement and concrete mismatch. Uncertainty leaves the applicable policy unsatisfied.

   An independent reviewer is useful enough to produce bounded escalation recommendations and `judged` evidence. It is not authoritative enough to waive a required test, approve a contradictory caller contract, or erase a structurally incompatible change. An update may legitimately conclude that no code edit is needed, but only after its obligations establish that conclusion.

   Reuse the existing distinction between coverage judgment and test run ([Evidence](/Users/mohebifar/dev/intentlang/SPEC.md:175)). A reviewer’s “code conforms” verdict does not renew test coverage automatically, and neither verdict alone produces `tested`.

4. **Cost control: batching is sound; unrestricted forward-only context is a hole.**

   Change a project prompt to “all authenticated requests must check account suspension,” then apply it forward-only. Existing endpoints remain vulnerable while the project appears to require the check. Recording two hashes makes the exception auditable; it does not make those endpoints conform.

   Restrict forward-only treatment to guidance explicitly defined as governing future implementation choices. Normative requirements, verification policies and structural contracts apply to existing declarations.

   If normative grandfathering is retained, model it as an explicit exception with named targets and an effective historical contract. Those declarations must remain visibly out of date against the desired contract. Ordinary updates must reconcile the full gap from their actual applied context, not merely the latest context edit. Hashes alone are insufficient unless the historical content remains retrievable.

   Batch reviews by shared change, but require a result for every subject. One “batch conforms” response cannot silently cover declarations the reviewer did not inspect. Print affected subjects, unique checks, planned review batches and contingent escalation cost. An estimate cannot promise that verification is cheap.

5. **Delete: the final-graph rule is good, but the cleanup proof is too strong.**

   Require no remaining references **in the desired graph**. References among declarations being deleted together must be allowed. Use the old graph to identify former consumers and cleanup work; otherwise removed edges disappear before their consequences are planned.

   Import conformance establishes only part of safety. An unmanaged service can call a deleted route by URL; a dynamic lookup can name a deleted export; a build may exclude a script that imports a removed file. State the actual guarantee: graph references are valid, managed imports conform, and configured checks pass. External compatibility requires an explicit contract or check.

   The delete write set must enumerate actual owned outputs, including registered agent-created files. `may_create_under` is permission to create files, not ownership of everything beneath a directory.

   Shared-file cleanup must be performed through its owner, which joins the transaction. If the deleted declaration owns a shared file with surviving contributors, require ownership transfer or reject deletion. “Updates owners” cannot remain an informal instruction.

   Remove active state and evidence, while retaining deletion provenance and historical evidence through history. Entity deletion may generate a destructive migration, but applying that migration to a live database remains outside the code transaction’s rollback guarantee.

B. These plan situations still need explicit semantics.

1. **Partial application needs an applied graph as well as a desired graph.**

   A plan adds an action and its page. After the action transaction succeeds, the desired graph contains the page but its outputs do not yet exist. Conformance against the entire desired graph would reject this legitimate intermediate state.

   Define intermediate checking against the effective applied declarations plus the current transaction. The desired graph is compiled globally; pending declarations and obligations remain visibly pending. “State, spec and code form a consistent triple” must distinguish accepted progress from completion of the desired spec.

2. **Resumption needs checkpoints, not repeated validation against the original digest.**

   The first successful transaction changes the state digest and code fingerprints to which the plan was bound. The original binding cannot simply be reapplied before transaction two.

   Give the immutable plan an execution journal linking each accepted transaction to its input and published result. Resume validates that chain, skips completed transactions and retries remaining work from a fresh candidate. Unexpected edits require replanning. A crash after publication but before journal acknowledgment must be detectable from the published transaction receipt.

   Independent transactions may continue after failure; dependent ones remain skipped. Earlier successful transactions stay published. That is transaction atomicity, not whole-plan atomicity.

3. **Amendments must create new plan revisions.**

   An unexpected shared-file edit can enlarge ownership scope, merge transactions or introduce ordering cycles. Appending one filename to the old write set is insufficient.

   An amendment gets a new binding and recomputes propagation, ownership, transaction membership and ordering. Preserve the failed candidate as a reviewable patch, not accepted state. Checks and judgments can be reused only when their bound inputs still match. No scope widening silently resumes under the original plan’s authority.

4. **Passing weak tests must not close a known evidence gap.**

   The current coverage/run split helps, but coverage judgments can themselves be wrong. Discovery that a test asserts only “request succeeded” must be able to revoke coverage even when neither requirement text nor test bytes changed.

   Track review dependencies beyond the expectation’s text: relevant observed contracts and effective context can change the meaning of an unchanged test. Evidence remains a scoped claim with a stated method, not a proof of all behaviour.

5. **Repair and drift need an explicit combined path.**

   A hand edit breaks generic-error handling while also adding useful accessibility changes. `adopt` falsely asserts that the edit is fine; `overwrite` can discard useful work.

   Permit an explicit repair transaction whose bound starting input includes the exact drifted files. Preserve unrelated edits, demonstrate the violation against that input, and publish the repaired baseline only after checks pass. The command must identify that drift as an input; it must not quietly renew the baseline or evidence first.

   If the hand edit changes intended behaviour, use `absorb` and review the spec change. Repair cannot adjudicate that intent implicitly.

6. **Merge reconciliation needs a concrete entry point and publication rule.**

   Use `spec plan --reconcile`, with merge parents and merge base recorded in the binding; allow an explicit base when ancestry is ambiguous. It produces an ordinary plan applied through `spec apply`.

   Run the existing three-way identity reconciliation before diffing, including member moves and output ownership ([Moves](/Users/mohebifar/dev/intentlang/SPEC.md:95)). Conflicting identities or ownership block planning; choosing the newest state file is not reconciliation.

   A textual merge success supplies no execution evidence for the merged tree. Preserve coverage judgments only where their inputs remain valid, rerun affected checks, and expose any remaining stale evidence. A merge can require implementation reconciliation even when the resulting spec has no new diff.

7. **Generator failure must not silently become agent work.**

   A matching generator can fail after creating half a migration, return success without producing required outputs, or write outside ownership boundaries. Its output requires independent validation just like agent output.

   A failed attempt is discarded or reset to a known checkpoint before retry. Distinguish unsupported change, generator defect, infrastructure failure and invalid generated output. Agent fallback is allowed only when it was included in the reviewed execution strategy or appears in an amended plan.

   Generators participating in rollbackable code transactions must not perform undeclared external mutations. Producing migration files and executing a migration are separate operations.

8. **The scheduler and evidence binding have several smaller, consequential gaps.**

   Obligation-only plans need transaction roots: a check-definition change or context review may contain no managed declaration with a code change. Inside a merged cyclic transaction, tasks cannot all run in topological order; define coordinated work for the cycle. Deletion also needs operation-specific ordering rather than blindly reusing creation order.

   A frozen declaration that requires an update blocks the relevant transaction. It cannot be skipped while the upstream change is published as verified.

   Earlier checks do not automatically cover later transactions. Refresh affected runs on the final tree, or leave them stale. Deduplicate checks by check definition and candidate inputs, not merely by command name.

   Finally, distinguish the checked source-input digest from the commit publishing its receipts. Writing a tree hash into evidence changes a Git tree containing that evidence. Exclude receipt metadata from the checked-input digest and define exactly which executable inputs it covers.

C. I would use the following vocabulary.

Keep the existing independent identity, lifecycle and drift dimensions. For changes, use four operations plus `none`. `repair` remains a CLI entry point and provenance reason for `update`; `replace` becomes an explicitly planned update strategy. Neither needs separate propagation semantics.

| Dimension / vocabulary | Trigger | What runs | Escalation | State published on success |
| --- | --- | --- | --- | --- |
| Change: `create` | New managed identity | Planned generator and/or agent, then required obligations | Scope or strategy expansion requires amendment | New ownership, applied contracts, dependency snapshots, baseline and evidence |
| Change: `update` | Own contract change, incompatible consumed contract, confirmed nonconformance, or explicit repair request | Scoped reconciliation of existing implementation; may conclude no edit is needed | Wider work requires amendment; replacement requires an explicit strategy | Updated baseline and applicable contracts/evidence; repair preserves contract hashes and records its cause |
| Change: `delete` | Identity absent from desired graph; no surviving graph references | Owned-output removal, contribution cleanup and required checks | Ownership conflict or destructive operation blocks until resolved | Active ownership/state removed; historical provenance retained |
| Change: `detach` | Explicit end of management, including protected removal | Ownership/state removal; files retained; graph references reconciled | Remaining references or ownership conflicts block | Outputs become unmanaged; historical evidence makes no current managed-conformance claim |
| Change: `none` | No implementation operation needed, but evidence or context requires work | Applicable obligations only | Confirmed nonconformance produces an update within scope, otherwise amendment | Renewed evidence or explicit context reconciliation, without inventing a code change |
| Identity: `moved` | Continuing identity asserted by `moved` | Re-key identities, ownership locators and bindings; reconcile affected paths/imports | Can coexist with any required update | Identity history preserved; evidence reassessed according to changed inputs |
| Obligation: `verify` | Required after mutation; changed check inputs, compatible consumed contracts, runtime dependency impact, or stale runs | CLI-controlled conformance, build and mapped tests on final candidate inputs | Confirmed implementation violation → update; coverage gap → review; infrastructure failure remains failure | Exact run receipts; `tested` only with current coverage |
| Obligation: `review` | Changed normative prose, unresolved semantic impact, stale coverage or judgment, or required review policy | Independent assessment of code conformance and/or test coverage, with per-subject results | Nonconformance → update; uncertainty leaves policy unsatisfied | Bound judgment or coverage record, with reviewer and inputs; no automatic upgrade to `tested` |

Both obligations can attach to one declaration. Publication requires their conjunction and the existing policy satisfaction rules. A failed check should never automatically authorize an arbitrary code edit merely because the file lies inside the write envelope.

Prioritised changes:

1. **Define publication precisely:** effective applied graph, final candidate inputs, conjunctive obligations and evidence-policy gates.
2. **Close propagation gaps:** implementation changes as well as spec changes; effective behavioural context; semantic assessment of observed prose.
3. **Specify repair:** requirement-derived correction, independently assessed red/green evidence, drift-aware inputs and honest nonautomated evidence.
4. **Define partial progress and resumption:** immutable plans, transaction receipts, checkpoints and amended-plan rebinding.
5. **Reject normative forward-only context:** retain future-only guidance or model explicit, visibly outstanding exceptions.
6. **Make deletion concrete:** old/new graph closure, enumerated ownership and shared-owner participation.
7. **Specify reconciliation and generator failures:** ordinary bound plans, validated outputs, controlled retries and no implicit fallback.
8. **Simplify the vocabulary:** four change operations, independent moves, and two additive obligations.