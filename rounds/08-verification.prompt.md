# Round 8: verification and runtime state

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read `SPEC.md`: "Evidence", "Verification", "Repair", "Toolchain and retained artifacts",
"Packages → Compatibility" (entities are deferred to this round), "Plan entries", and the
worked example (the `Workspace` field addition). REVIEW.md flaw 6: "Runtime state is not
modelled. The plan knows a migration was generated, not whether any database has run it."
REVIEW.md scored verification 6/10 ("AST conformance checks put a heavy burden on package
authors").

## Editor's draft positions (attack them)

1. **Test-case identity.** Tests are outputs (a `tests` slot). A test case is identified by
   `{ output, case }`, where `case` is a stable tag the test author writes into the test
   (for vitest, `it("...", { meta: { case: "forgot_link" } })`); the package's test check
   extracts tags and reports per-case results in a standard JSON (`case → passed | failed |
   skipped`, plus a per-case definition hash computed from the test's AST). Untagged tests
   still run but can never be coverage for a requirement. Tags are unique per output.
2. **Test-first as policy.** `verify: "tested"` requirements that are new get their test
   written first, by a separate agent dispatch with write access only to the `tests` slot,
   from the requirement text. The implementer then cannot edit those test cases (the CLI
   rejects changes to their definition hashes) unless a reviewer approves the change.
   Optional per project: `test_first: true`.
3. **`checked`** means a conformance check (a static analyser shipped by the package)
   reported the subject as satisfied. Packages may map conformance rules to requirement
   patterns; most requirements will never be `checked`.
4. **Entity compatibility** has three dimensions:
   - read: can existing readers handle the new shape (adding an optional field: yes);
   - write: can existing writers still produce valid rows (adding a required field with no
     default: no);
   - storage: can existing rows be transformed (adding a non-null column needs a default or
     a backfill; narrowing a type needs a data check).
   The plan prints each change as `additive`, `backfill`, or `destructive`. Destructive
   changes require `--allow-destructive` and are expected to be staged expand →
   migrate → contract across plans.
5. **Backfills** are declared on the field: `tenant_id: uuid @backfill("copy from owner's
   tenant")`. The backfill is code an agent writes, owned by the database binding as a
   retained artifact next to the migration.
6. **Environments and runtime state.** The root module may declare
   `environment production { database: "DATABASE_URL" }`. Runtime state is *observed*, never
   committed: `spec status --env production` asks the database binding's package to read the
   database's migration ledger and compares it with the retained migrations. A plan whose
   contract step drops a column requires an observation, younger than a configured age,
   showing the corresponding expand migration applied in every declared environment. The
   observation is recorded in the plan binding, not in state.
7. **Secrets and environment variables.** `stack` may declare required variables by name
   and type (`env { DATABASE_URL: secret, NEXT_PUBLIC_SITE: url }`); values never appear in
   spec, state, orders or evidence. Checks run under a named test profile supplied by the
   runner.

## Questions

A. Attack each position with concrete failure scenarios: renamed or moved test cases;
   parameterised tests; a test that covers three requirements; flaky tests and evidence;
   snapshot tests; an implementer that legitimately needs to fix a broken pre-written test;
   a backfill that cannot run in one transaction on a big table; an environment that is
   unreachable from CI; observations that go stale between plan and deploy; two
   environments at different migration heads.
B. What does verification still promise that it cannot deliver? Rewrite the verification
   levels table honestly if needed. Is "checked" worth keeping in v0?
C. Is the boundary right: the spec tool generates and retains migrations and observes
   environments, but never runs migrations or deploys? What breaks at that boundary?
D. End-to-end behaviour (the deferred `flow` kind): should v0 have any story for
   requirements that span several declarations (sign up, then land on the dashboard)?

Give concrete syntax and JSON where relevant. End with a prioritised list of changes.
