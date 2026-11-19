The central correction is to separate **source verification**, **schema compatibility**, and **deployment readiness**. A passing test, a generated migration, and an observed migration ledger establish different facts. None should stand in for another.

The existing split between coverage judgement and test execution is sound. The draft weakens it by treating verification methods as a strength ladder and runtime observations as deployment authority.

1. **Test-case identity: keep the pair, expand the protocol.**

   `{ output, case }` fits the existing identity model, provided `output` is a qualified address such as `auth.login#outputs.tests`, never a filename.

   Concrete failures:

   - Renaming a test’s display text should preserve identity. Moving its file should preserve identity through the output slot. Changing its tag creates a new case unless an explicit identity move connects them.
   - Moving a case between outputs changes the pair. Moving the entire output can use existing output moves; moving one case needs its own mapping.
   - Parameterised tests need a distinction between the authored case and its instances. Array indices and generated display names are unstable instance identities.
   - A test covering three requirements needs three coverage judgements. Its passing result does not establish that all three were asserted adequately.
   - A snapshot’s expected contents are part of the oracle. Changing the snapshot can invalidate coverage without changing the test AST.
   - An unchanged test body can become vacuous when a helper, mock, fixture, setup hook, or runner configuration changes.

   Therefore, an AST hash is useful bookkeeping, but insufficient as a definition hash. Define an **oracle digest** covering the case and its test-side dependencies, including expected snapshots. Where an adapter cannot isolate that closure, conservatively hash the whole test harness.

   Separately, bind the run to the implementation inputs. Changing implementation should usually require rerunning the test, without requiring a new judgement about what the unchanged oracle covers.

   A runner-neutral result could look like this:

   ```json
   {
     "protocol": 1,
     "output": "auth.login#outputs.tests",
     "selection": "complete",
     "cases": [
       {
         "case": "generic_error",
         "oracle_digest": "sha256:...",
         "instances": [
           {
             "key": "unknown_email",
             "attempts": ["passed"]
           },
           {
             "key": "wrong_password",
             "attempts": ["failed", "passed"]
           }
         ]
       }
     ],
     "inventory_digest": "sha256:...",
     "input_digest": "sha256:...",
     "check_digest": "sha256:...",
     "profile_revision": "integration-v3"
   }
   ```

   Require explicit stable instance keys where individual identity matters. For generated or random cases, record the generator, input digest and seed; claim coverage of that execution, not every possible instance.

   A missing, skipped, filtered-out, or undiscovered expected case cannot count as passed. Duplicate tags must fail discovery. Unsupported tagging must be reported explicitly.

   Flakiness needs first-class treatment: preserve attempts. Under a strict policy, `failed → passed` does not satisfy the gate. An explicitly configured retry policy may accept it, but the evidence must retain that fact. Quarantining a case leaves its requirements unsatisfied unless other current evidence covers them.

   Untagged tests can still fail the overall check; they simply cannot supply requirement coverage.

2. **Test-first: useful process control, overstated protection.**

   A separate dispatch limits who edits the oracle. It does not establish that the oracle correctly interprets the requirement. Two agents can share the same mistaken assumption.

   More concrete problems:

   - A new test imports an interface that does not exist yet. Its failure proves missing scaffolding, not the intended behavioural defect.
   - A requirement changes while its tag stays stable. A policy applying only to “new requirements” misses this.
   - The pre-written test assumes the wrong timezone, inaccessible selector, or invalid fixture. Implementation cannot legitimately satisfy it.
   - Freezing only the case AST permits weakening its shared mock or setup. Freezing the whole tests file prevents legitimate addition of neighbouring cases.
   - A test writer restricted to one slot may lack permission to add fixtures, snapshots or shared test utilities.

   Define a reviewed **test envelope**, including necessary test support files. Protect approved oracle digests, rather than merely tagged function bodies.

   Provide a concrete amendment path:

   ```json
   {
     "kind": "oracle_amendment",
     "case": {
       "output": "auth.login#outputs.tests",
       "case": "forgot_link"
     },
     "old_oracle_digest": "sha256:...",
     "new_oracle_digest": "sha256:...",
     "reason": "Fixture used an authenticated session for a public-page assertion",
     "review": {
       "decision": "approved",
       "artifact_digest": "sha256:...",
       "reviewer": "reviewer-42"
     }
   }
   ```

   The implementer may propose the correction. Approval must bind the exact patch; publication then requires fresh coverage and runs. Preserve the rejected oracle and original failure in history.

   Keep `test_first: true` optional, and apply it to new **or changed requirement revisions**. Record the initial outcome honestly: assertion failure, setup failure, already passing, or not executable yet.

   The existing Repair rule is stronger and appropriate: for a claimed reproduced defect, record failure before and success after under equivalent conditions. If reproduction is unavailable, do not manufacture `tested` evidence.

3. **`checked`: keep narrow facts, reject prose pattern matching as authority.**

   A package might recognise “requires authentication” and find an authentication call in the AST, while the handler sends sensitive data before making that call. The recognised syntax exists; the requirement remains violated.

   Conformance results should identify explicit rules, subjects, supported constructs and limitations:

   ```json
   {
     "rule": "database.schema_matches",
     "subject": "core.Workspace",
     "result": "satisfied",
     "input_digest": "sha256:...",
     "checker_digest": "sha256:..."
   }
   ```

   Distinguish `satisfied`, `violated`, `unsupported` and execution error. Silence is never satisfaction.

   Packages may suggest check bindings from prose. A suggestion should not establish that the rule entails the requirement.

   **Keep `checked` in v0 as a narrowly scoped evidence result. Remove its role as a generic strength between `judged` and `tested`.** Do not require package authors to implement broad behavioural AST analysis. Start with output ownership, supported import rules and concrete schema comparisons; let adapters add explicitly bounded checks.

4. **Entity compatibility: three dimensions are right; one change label is insufficient.**

   Read and write compatibility must be directional and state their assumptions.

   “Adding an optional field is readable” assumes readers tolerate additional fields. A strict decoder may reject them. An existing writer using positional inserts may break after an added column. A default supplied by new application code does not protect old writers; a database default may protect omitted values but not explicit `NULL`.

   Storage feasibility is also distinct from runtime safety. A transformation can be possible while requiring a long rewrite or disruptive lock.

   Keep the dimensions separate:

   ```json
   {
     "entity": "core.Workspace",
     "change": "add_field",
     "field": "archived_at",
     "compatibility": {
       "existing_readers": "compatible",
       "existing_writers": "compatible",
       "assumptions": ["named-column access"]
     },
     "storage": {
       "data_loss": "none",
       "backfill": "not_required",
       "operational_cost": "unknown"
     },
     "rollout": {
       "requires": ["migration:workspace_archived_at"]
     }
   }
   ```

   `additive`, `backfill` and `destructive` are useful summaries, but overlap. A change can need a backfill, impose locking risk and eventually discard data.

   Propagation should use the relevant dimension: readers consume read compatibility; writers consume write compatibility; declarations doing both consume both. Storage prerequisites belong to rollout.

   `--allow-destructive` authorises the declared destructive operation. It proves neither readiness nor recoverability. Unknown compatibility must remain unknown, rather than acquiring an optimistic label.

5. **Backfills: prose describes intent; it does not define an executable operation.**

   “Copy from owner’s tenant” leaves unresolved:

   - Which owner and tenant relation?
   - What happens when the owner is missing?
   - What happens when ownership changes during processing?
   - How are new rows handled while old rows are being backfilled?
   - What proves completion?

   A field annotation may reference a stable operation:

   ```spec
   tenant_id: uuid? @database.backfill("tenant_from_owner_v1")
   ```

   The binding should own a retained manifest alongside the code:

   ```json
   {
     "id": "tenant_from_owner_v1",
     "requires": ["migration:add_nullable_tenant_id"],
     "artifact": "backfills/tenant_from_owner_v1.ts",
     "artifact_digest": "sha256:...",
     "execution": {
       "mode": "batched",
       "transaction": "per_batch",
       "resume_key": "workspace.id",
       "retry_contract": "idempotent"
     },
     "completion_check": "workspace_tenant_assignment_valid_v1"
   }
   ```

   The manifest declares obligations; checks and review must establish whether the implementation meets them.

   For a large table, use resumable batches, bounded transactions, external progress records and an explicit concurrent-write strategy. Completion must validate the resulting data, including rows created during the backfill. A cursor reaching its final key is insufficient.

   Keep execution progress outside committed declaration state. Retain operation identities and code immutably; observe completion per environment. An applied schema migration must not imply a completed asynchronous backfill.

6. **Environments: observe runtime facts, but stop treating freshness as safety.**

   `DATABASE_URL` identifies a secret lookup, not a database. The same variable name can resolve to a different cluster, restored database or schema tomorrow.

   Environment declarations need binding selection and a verifiable resource identity:

   ```spec
   environment production {
     database: {
       binding: database
       connection: env.DATABASE_URL
       expected_resource: "acme-prod-primary"
     }
   }
   ```

   An observation should contain at least:

   ```json
   {
     "environment": "production",
     "resource_id": "acme-prod-primary",
     "resource_generation": "restore-generation-7",
     "binding": "database",
     "observed_at": "2026-11-18T18:00:00Z",
     "observer_digest": "sha256:...",
     "ledger_digest": "sha256:...",
     "applied": [
       {
         "id": "add_nullable_tenant_id",
         "checksum": "sha256:..."
       }
     ],
     "backfills": {
       "tenant_from_owner_v1": "complete"
     }
   }
   ```

   Concrete failures remain:

   - **Unreachable from CI:** allow an authorised external observer to supply a receipt with verifiable provenance. Without one, report `unknown`; reachability failure is not evidence of either application or absence.
   - **Different migration heads:** evaluate each environment separately against migration dependencies and checksums. Avoid treating a single “latest migration” string as sufficient.
   - **Restores and manual changes:** a ledger is evidence about the ledger. It does not prove the live schema or data still matches it.
   - **Fresh but obsolete:** an observation can become invalid immediately after collection through another deploy, restore or backfill failure.
   - **Expand applied, old code still running:** old workers, scheduled jobs, rollback releases or external clients may still use the column.

   Consequently, “expand applied everywhere” is an insufficient contract gate. Dropping a column may require backfill validation, new writer rollout, retirement of old readers and writers, and a defined rollback boundary.

   Scope the gate to an explicit deployment target set. “Every declared environment” accidentally includes abandoned previews and unrelated environments, while missing undeclared consumers.

   There is also a document-model conflict: plans are immutable, but observations change. Store an immutable observation receipt separately and bind its digest into a plan revision. `status` must not silently refresh an existing plan’s authority.

7. **Secrets: declarations are useful; “values never appear” is not enforceable by naming alone.**

   A failing test can print its connection string. A snapshot can contain a token. A package can echo its environment.

   Specify the actual contract: durable protocols accept allowlisted metadata; runners control credential injection and diagnostic retention; secret values are excluded from serialised environment maps. Redaction is mitigation, not proof against arbitrary encoding or disclosure.

   Named test profiles also need versioned, non-secret identities. Record relevant runner image, configuration, fixture revision, service versions and other execution inputs. A profile name alone cannot explain why yesterday’s run differs from today’s.

   Reconcile this with the current package execution policy: checks presently receive no credentials. Disposable test credentials and production observation credentials need distinct execution policies. Observers should receive narrowly scoped read access through the runner; production credentials must not become available to ordinary checks or generators.

The verification table should describe evidence, not ascending levels:

| Evidence | Honest promise | Does not establish |
| --- | --- | --- |
| Compile | The graph satisfies the language’s static rules. | Implementation or runtime correctness. |
| Conformance | A named checker reported a named rule satisfied on recorded inputs, within its supported scope. | Arbitrary prose compliance or unobserved runtime facts. |
| Build | Configured commands completed successfully under the recorded profile. | Universal determinism or behavioural correctness. |
| Behaviour test | Selected cases met the configured outcome policy on recorded inputs; current review maps their oracles to requirement revisions. | Exhaustive behaviour, absence of flakes, or production success. |
| Review | An identified reviewer gave a recorded judgement against specified artifacts. | An executable proof or guaranteed independence of reasoning. |
| Human acceptance | A human approved an exact revision and scope. | Replacement for required tests or checks. |
| Runtime observation | A collector reported specified external facts at a recorded time. | Continuing validity or permission to deploy. |

The current substitution rule—`tested` satisfies `checked`—is unsound. A passing browser test does not discharge an import restriction. A static checker cannot replace a required browser test. Likewise, the “strongest policy” rule cannot combine human acceptance with testing correctly because those are independent requirements.

Use conjunctive policy instead:

```spec
verify: {
  tests: "required"
  checks: ["database.schema_matches"]
  review: "required"
  approval: "required"
}
```

Existing strings can remain shorthand, but define their expansion explicitly. Keep build and structural checks as transaction obligations rather than pretending every prose requirement has a build “level.”

Also separate evidence outcome from currency. A requirement can have current review, stale test coverage and a fresh failed run simultaneously. One `stale` status hides information needed to repair it.

**The generation/observation boundary is appropriate for v0**, provided the tool publishes deployment prerequisites rather than claiming to enforce deployment safety.

What breaks at that boundary is coordination: the tool cannot prevent an external runner from deploying the wrong artifact, skipping a prerequisite or acting after an observation becomes obsolete. A plan-time age limit cannot solve that.

Retained artifacts therefore need a machine-readable handoff containing immutable artifact digests, migration dependencies, target resources and execution-time preconditions. The external deployment runner must revalidate those preconditions under its own concurrency controls, run the operations and produce receipts. If it does not participate, the spec tool can report readiness as unknown; it cannot guarantee a safe rollout.

The worked `Workspace.archived_at` example exposes this directly. Generated code may select `archived_at` from a database where the column does not yet exist. A nullable application type does not make that query succeed. The plan should distinguish:

```text
Source: migration and updated access code verified
Rollout prerequisite: workspace_archived_at applied on target database
Production readiness: unknown
```

Test both a clean installation and upgrade from the supported prior schema. Neither test substitutes for observing production.

Do not make source publication wait for production migration application: producing the reviewed artifact is often necessary before that migration can run. Keep source transactions and deployment transitions separate.

For end-to-end behaviour, **v0 needs a story, but does not need the `flow` kind**.

An expectation on an existing declaration can span several declarations:

```spec
export nextjs.page signup {
  // Other fields omitted.
  expect {
    complete_signup {
      text: """
        A visitor creates an account through this page, arrives at
        @{dashboard.home}, and remains signed in after reloading.
      """
      verify: "tested"
    }
  }
}
```

Its test can exercise signup, session persistence, navigation and dashboard rendering through the actual application. Declaration-local ownership must not imply declaration-local test scope.

Allow the coverage relation to connect any test case to any appropriate requirement subject. Give the test one output owner; record all relevant implementation and harness inputs. For v0, the existing `verify_scope: "all"` is a defensible conservative approach for journey suites. Narrower execution needs explicit dependency information: prose mentions alone do not propagate every relevant behaviour or implementation change.

A mocked redirect test may support part of this requirement, but cannot establish the whole journey. Coverage review must name that gap. Later, `flow` can provide reusable journey identity and structure without introducing a different evidence system.

Prioritised changes:

1. **P0 — Separate source completion from deployment readiness.** Replace the expand-observation contract gate with explicit rollout prerequisites and external execution-time enforcement.
2. **P0 — Replace verification ranking with conjunctive obligations.** Preserve independent tests, checks, review and approval; separate outcomes from currency.
3. **P1 — Specify the test protocol.** Qualified identity, explicit moves, parameter instances, complete discovery, retry history and oracle digests including snapshots and support code.
4. **P1 — Model migration and backfill operations explicitly.** Immutable identities, checksums, dependencies, resumability and independently observed completion.
5. **P1 — Preserve compatibility dimensions in plans.** Directional reader/writer compatibility, storage effects, operational unknowns and per-environment prerequisites.
6. **P1 — Add oracle amendments.** Exact reviewed patches, preserved history and fresh evidence; apply test-first policy to changed requirement revisions.
7. **P2 — Bound `checked` and package obligations.** Explicit rule IDs and unsupported outcomes; avoid mandatory general-purpose AST conformance.
8. **P2 — Support cross-declaration tests now; defer `flow`.** Reuse coverage subjects, output ownership and conservative invalidation.
9. **P2 — Define observer and profile protocols.** Resource identity, receipt provenance, credential isolation and durable diagnostic rules.