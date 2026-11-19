The core bet is coherent as a workflow for producing and verifying incremental patches. It is less coherent as a reconciler: an output hash establishes which bytes were accepted, but cannot establish that today’s code still satisfies the spec. The missing design center is the **verified change transaction**, which may span several nodes.

I would lower REVIEW.md’s architecture score until the transaction, invalidation and merge semantics are settled.

## 1. Per-node transactions can make valid changes impossible — critical

**Where:** “State, plan and apply”; “Agent interface”.

**Problem:** Change an action from `lookup(id)` to `lookup({ id, tenant })`, requiring updates to its callers. The action’s mandatory build fails against the old callers, so its state never advances and the dependent caller tasks are skipped; reversing the order fails against the old action.

**Proposal:** Keep nodes as units of attribution, but allow a transaction to cover a compatibility group. Prepare the action and affected caller patches in one integration worktree, run checks on the combined result, then commit their code and state atomically:

```json
{
  "transaction": ["accounts.lookup", "dashboard.home"],
  "checks": ["typecheck", "build", "caller-contract-tests"]
}
```

Partial progress inside that group is staged work, not applied state. This is a more fundamental problem than insufficient multi-file ownership.

## 2. Disjoint writes do not make verification composable — critical

**Where:** “Plan algorithm”; “Agent interface”; “Open questions” on branch merges.

**Problem:** Two worktrees can add conflicting global declarations in different files: each builds against the original baseline, while their conflict-free Git merge fails. Likewise, two branches can merge different nodes’ code and state without textual conflicts even though nobody has checked their combined behavior.

**Proposal:** Treat worktree success as provisional. Merge a wave into a candidate tree, run integration checks there, and only then publish its applied state.

Verification evidence must identify the tree on which it ran:

```json
{
  "check": "build",
  "tree": "<git-tree-id>",
  "check_definition_hash": "...",
  "result": "passed"
}
```

After a branch merge, re-establish the required integration evidence. Per-node state files reduce textual contention; they do not establish semantic independence.

## 3. The action table is not a decision procedure — major

**Where:** “Core model”; “State, plan and apply”; “Lifecycle”.

**Problem:** A hand-edited node with a changed prompt matches both `resolve` and `update`; a renamed node with a changed kind matches `move`, `replace` and potentially `update`. Removing a `protected` node matches `delete`, which its lifecycle prohibits, with no defined resulting state or owner.

**Proposal:** Separate the dimensions instead of forcing one action per node:

1. Resolve identity and explicit moves.
2. Enforce lifecycle restrictions.
3. Resolve baseline drift.
4. Classify the desired implementation change.
5. Determine verification obligations.

A plan could show `move + update`, or `blocked: drift resolution required`. Protected removal should require an explicit detach or leave a tracked tombstone.

Also settle the identity contradiction: Summary includes kind in the address; Core model excludes it. Use `(module, name)` consistently if changing kind is meant to preserve identity.

## 4. A reviewed plan has no binding to the inputs it approved — critical

**Where:** “Plan algorithm”; “Work order”; “Agent interface”.

**Problem:** A human edits an owned file after `plan` but before `apply`; the saved plan can then run without presenting that edit as drift. Separately, an agent may declare a new file outside the original write set, so the reviewed scope is not actually an upper bound on its output.

**Proposal:** Bind the plan to a baseline commit, graph hash, lock hash, state digest and relevant working-tree fingerprints. Recheck those preconditions before applying and before publishing each transaction.

Make new-file permission explicit, for example:

```json
{
  "baseline_commit": "...",
  "graph_hash": "...",
  "write_set": ["src/auth/login.tsx"],
  "may_create_under": ["tests/auth/login/"]
}
```

An expansion beyond that envelope produces an amended plan. Define crash recovery too: discard uncommitted candidate work, and recognize committed transactions by durable identifiers.

## 5. The hashes do not cover the actual input surface — major

**Where:** “Two hashes per node”; “Fields every node accepts”; “State record”; “Packages and providers”.

**Problem:** `summary` is passed to dependents but apparently belongs to the body hash, so changing an explicitly supplied input does not invalidate those dependents. An `extern` symbol’s source can change without changing its declaration, while an upgraded package can change output conventions or generators yet receive only the blanket `verify` treatment.

**Proposal:** Define hashes as precise projections of a canonical graph, and record the inputs actually consumed by each task:

```json
{
  "inputs": {
    "dashboard.home:public_contract": "...",
    "dashboard.home:summary": "...",
    "extern.LegacyChart:source": "...",
    "nextjs:output_policy": "...",
    "nextjs:checks": "..."
  }
}
```

Specify defaults, ordering, annotations and resolved reference identities in canonicalization. Separate changes to instructions, schemas, checks, generators and output policies; they need not imply the same action.

This is distinct from REVIEW.md’s prose-propagation flaw: even mechanically identifiable inputs currently fall outside the stated invalidation rules.

## 6. One reference graph is doing several incompatible jobs — major

**Where:** “Core model”; “Projects, files and modules”; “Plan algorithm”.

**Problem:** A page linking to another page needs its route, while a caller changing alongside an action needs an implementation compatibility relationship; the draft represents both as references and only later informally distinguishes ordering edges. Requiring all data and call dependencies to be acyclic also excludes ordinary mutually related entities and mutually recursive behavior.

**Proposal:** Give edges operational types:

```json
{
  "from": "auth.login",
  "to": "auth.forgot_password",
  "relation": "navigation",
  "consumes": ["route"],
  "requires_applied_target": false
}
```

Distinguish name resolution, consumed contract fields, implementation scheduling and verification invalidation. Allow cycles in the domain graph; use compatible groups where cyclic implementation work is necessary.

A declaration dependency should not automatically become an execution dependency.

## 7. Capability substitution promises compatibility without defining it — major

**Where:** “Surface syntax”; “Packages and providers”.

**Problem:** Two authentication packages can both export `User`, `session` and `skills` while disagreeing about identifiers, session lifetime, error behavior and storage. Changing one `use` line therefore does not establish a compatible substitution, and changes to setup or shared infrastructure have no clearly identified implementation owner.

**Proposal:** Either define versioned capability contracts, including behavioral obligations and explicit adapters, or remove substitution from v0’s promises. Distinguish:

- an installed library;
- a declared capability contract;
- the project implementation that wires the library into that contract.

For the first milestone, bind directly to one concrete authentication library. A provider swap should be an explicit migration operation, not merely context invalidation.

## 8. Adoption can preserve evidence that is no longer true — major

**Where:** “Drift, hand-written code and shared files”; “State record”; “Verification”.

**Problem:** A human can adopt an edited implementation or test, updating its output hash while retaining a `tested` expectation whose previous run no longer supports the current bytes. `spec import` similarly records code in state without requiring checks, and `frozen` ignores changes while the record may continue displaying old verification statuses.

**Proposal:** Separate **observed baseline** from **verification evidence**. Adoption accepts bytes; it does not automatically renew their compliance certificate.

Invalidate affected evidence when its implementation, test, configuration or other recorded inputs change. Import starts as unverified; frozen outputs can remain unwritable while still reporting stale evidence. A human acceptance should have its own status rather than inheriting `tested`.

## 9. The milestone cannot measure its stated riskiest assumption — major

**Where:** “Goals and non-goals”; “Open questions and first milestone”.

**Problem:** An agent can delete required behavior, produce a small diff, stay within scope and pass typecheck/build—scoring well on every proposed milestone metric. Ten sequential edits therefore measure operational obedience and buildability, not “a small, correct change.”

**Proposal:** Give each trial edit an independently prepared behavioral acceptance check, outside the implementation agent’s write authority. Include an interface change, an adopted human edit and a requirement that tempts the agent to expand scope.

Measure retained behavior, intended behavior, review corrections and scope escapes. Run the same tasks with ordinary agent instructions against the same baseline so the experiment tests whether graph/state orchestration provides a benefit. Build the harness before investing in the parser.

## 10. The worked example violates the stated import enforcement rule — major

**Where:** “Packages and providers” kind definition; “Verification”; “Worked example”.

**Problem:** Level 2 rejects imports outside `uses`, but the page schema types `uses` as UI components: `login` must access `sign_in`, and `sign_in` must access the authentication library despite declaring no such `uses`. Either the example fails conformance or `calls`, `reads`, `requires` and framework imports grant undocumented exemptions.

**Proposal:** Define the allowed dependency set explicitly as a union of differently typed declarations:

```text
calls: [sign_in]
libraries: [authentication]
uses: [ui.form, ui.input, ui.button]
```

Kind-supplied framework imports can be an explicit ambient allowance. Skills should supply instructions, not silently authorize code dependencies.

Turn the example into a complete compiler-and-planner fixture: include the new page/action declarations, package catalogs and old state. Its exact waves cannot currently be reproduced because the reset action’s dependencies are absent.

## Cut list

- **Custom syntax and editor tooling:** start with a versioned JSON graph and work-order runner. Determinism does not require a new grammar.
- **Entities, migrations and persistence-specific core semantics:** these introduce a second reconciliation problem before code editing is proven.
- **Capability swapping and package authoring:** hard-code one integration; defer the ecosystem abstraction.
- **Parallel apply:** use serial candidate transactions until merged-result verification and recovery work.
- **Selectors, contributions, automatic absorb/import, symbol ownership and templates:** each adds a substantial independent mechanism.
- **`project`, `stack`, `use` and `module` as ordinary implementation nodes:** treat them initially as configuration/context, unless they explicitly own an artifact.
- **Core `action` restricted to server behavior:** make application concepts adapter vocabulary. The runner’s essential concepts are managed target, contract, ownership, checks and transaction.
- **Endpoints, UI components, shared types and enums as separately managed kinds:** introduce them when a trial requires independent identity and ownership. Do not create state records merely because something has a name.

Keep project/module context, a minimal managed target, explicit references, whole-file ownership, drift detection and concrete verification commands.

## Top 5

1. **Replace mandatory per-node transactions with verified patch transactions** that can span incompatible interface changes.
2. **Bind plans to immutable inputs and bounded write permissions**, with explicit publication and crash-recovery rules.
3. **Require checks on the combined candidate tree**, including after parallel or branch merges.
4. **Define consumed-input hashing and evidence invalidation**, including external code, adoption and package policy changes.
5. **Build the behavioral evaluation harness first**, then test the smallest graph/state runner against an ordinary agent workflow.