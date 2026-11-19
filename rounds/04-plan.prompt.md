# Round 4: plan semantics

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read `SPEC.md`, especially "Core model" (edges, projections, evidence), "Moves", and all
of "State, plan and apply". REVIEW.md flaws 2, 3, 8 and 9 are the triggers:

- flaw 2: no bug-fix flow when code is wrong but the spec is right;
- flaw 3: a prose change to an action's behaviour does not change its public projection,
  so callers are never re-verified;
- flaw 8: `verify` is fuzzy, and a project-prompt edit marks every node;
- flaw 9: `delete` says "references cleaned up" without saying how.

## Editor's draft positions (attack them)

1. **Repair.** New change type `repair`: spec unchanged, code wrong. Started by
   `spec repair auth.login --requirement expect.generic_error` (or by `spec check` finding
   failing evidence). Rules: a repair must cite a requirement, either an existing member
   whose evidence fails, or a new `expect` member added in the same plan (which makes it
   an `update`). The agent must first produce a test that fails on the current tree, then
   fix the code so it passes. A free-text repair with no requirement is allowed only with
   `--unspecified`, is recorded as such in provenance, and counts toward an "unspecified
   fixes" metric that tells the team the spec is missing requirements.
2. **Prose propagation.** Each relation declares what it *consumes* (changes there give
   `update` if incompatible, `verify` if compatible) and what it *observes*. `calls`
   observes the target's behaviour projection: its `prompt`, `expect` and `errors` text.
   A change to an observed projection gives callers a `verify` obligation, never an
   `update`. Verify is only checks, so this is cheap.
3. **Two obligations, not one.** `verify` means: run the deterministic checks (conformance,
   build, mapped tests) on the candidate tree; an agent runs only if they fail. `review`
   means: a reviewer agent compares the code with *changed prose* it cannot test (a module
   prompt now says "split layout" instead of "centered card") and answers conforms / does
   not conform; "does not" escalates to `update`. Context prose changes produce `review`;
   everything else produces `verify`. Without `review`, a module-prompt change would
   "verify" trivially and never be implemented.
4. **Cost control for `review`.** The plan prints the count and an estimate. One reviewer
   task can cover many declarations for the same context change. The author may mark a
   context change as forward-only (`spec plan --context forward`): existing declarations
   record that they were applied under the old context and are not reviewed; new or
   updated declarations follow the new one. The record keeps both hashes so the gap is
   visible.
5. **Delete.** A declaration can only be deleted once no other declaration references it
   (otherwise compile error), so dependents already got an `update` for their removed
   edge in the same plan. Since code may import only what edges grant, no conforming code
   imports the deleted outputs after those updates. The delete transaction contains the
   deleted declaration and every declaration whose edges to it were removed in this plan.
   It removes owned outputs (including agent-created files), drops state and evidence,
   updates owners of shared files it contributed to, and is destructive for entities
   (explicit flag). Unmanaged code that still imports a removed file fails the build, which
   fails the transaction.

## Questions

A. Attack each position with concrete failure scenarios. In particular: can `repair` be
   abused to change behaviour without a spec change? What does "test fails first" mean for
   bugs that are hard to test (layout, race conditions)? Does observing a behaviour
   projection create propagation storms? Is `review` trustworthy enough to escalate or
   to waive an update? Is forward-only context a hole?
B. What other plan situations are still undefined? Consider: partial failure halfway
   through a plan and resumption, amended plans, a `verify` that passes only because tests
   are weak, the interaction of `repair` with drift (code wrong *and* hand-edited), merge
   reconciliation after branch merges (round 3 left the command unspecified), and a
   generator task that fails.
C. Propose the final change and obligation vocabulary as a table (trigger, what runs,
   what can escalate to what, what state is published). Cut anything redundant.

End with a prioritised list of changes.
