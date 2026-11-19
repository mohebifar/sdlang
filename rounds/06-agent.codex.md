The draft has the right trust model—**results are claims; the CLI owns verification**—but the schemas do not yet identify exactly what was authorized, edited, checked, or reviewed. The biggest gaps are snapshot binding, coordinated transactions, repair evidence, and reviewer output.

**A. Schema and example review**

I checked both examples against the keywords used by their schemas with a local validation walker; both pass. A standard Draft 2020-12 validator was unavailable. The same checks accept a repair without `repair`, attempt 4 with `max: 3`, and a result naming another task. Some fixes belong in JSON Schema; cross-document relationships require CLI validation.

For [work-order.schema.json](/Users/mohebifar/dev/intentlang/schemas/work-order.schema.json):

| Field | Problem and concrete change |
|---|---|
| `schema` | Versioning is present, but compatibility rules are absent. Reject unsupported versions before execution. Use a new version for the structural changes below. |
| `task`, `transaction` | `<transaction>/<address>` is not globally unique: another plan can have `tx-4/auth.login`. Scope identity by plan, revision, task, and dispatch. Persist dispatch identity across the execution journal. |
| `plan.id`, `revision` | Useful locators, but insufficient bindings. A stale result can match these strings while referring to different order contents. Add an order digest and immutable input identities. |
| `plan.candidate_base` | A commit cannot represent the exact drifted input required by `repair`. Distinguish the base commit from the candidate’s actual input snapshot, including relevant untracked files and file modes. Fresh attempts must reconstruct that snapshot. |
| `attempt.number`, `max` | No `number <= max` check; `max` has no schema default. JSON Schema defaults would still not implement retry policy. The CLI must materialize and validate these values. |
| `attempt.workspace` | `fresh` and `continued` need exact snapshot semantics. Fresh from the transaction base can erase earlier successful tasks in the same transaction. Identify the restart checkpoint explicitly. |
| `attempt.previous` | Optional despite the promise to include every failure. Failure entries lack check-definition identity, evaluated snapshot, stable failure identity, and durable diagnostics. Require the array and validate its correspondence with the journal. |
| `declaration.address`, `kind` | Only one declaration is representable, whereas SPEC requires one coordinated order for an ordering cycle. Support multiple subjects, with ownership attribution. |
| `declaration.change` | `create/update` is reasonable for coding orders. Do not stretch it to represent obligation-only reviews or deterministic deletion; define separate order kinds. |
| `strategy` | Require the CLI to emit it. For a kind replacement, include the previous kind and old-to-new output mapping. A default must not hide that decision. |
| `cause`, `cause_detail` | A declaration can have several causes. Replace one enum plus prose with typed `causes[]`, naming changed edges, projections, requirements, or escalation receipts. |
| `spec.current`, `previous`, `diff` | Require previous text or explicit `null`, and a diff for updates. Define canonical rendering and diff generation. Renames also need the old address. Prose descriptions alone cannot enforce consistency. |
| `repair` | Currently unrelated to `cause`. Require it for repairs and forbid it otherwise. Add the requirement revision and pre-fix snapshot. Validate that `intended` follows the existing requirement; it cannot introduce new behavior. |
| `context.inherited` | Preserve source identity, revision/hash, and normative classification. Define inheritance order and provide the previous normative context or its diff when context changes trigger work. |
| `context.references` | Missing projection hashes, graph viewpoint, old/new identity, and structured entry points. A string `address` conflates declarations, capabilities, and catalog symbols. Validate relations against the pinned package definitions, allowing custom relations. |
| `context.skills` | These are locators, not skill text, although the delivery promises skill text. Record whether each body was included or omitted, with an immutable resource handle. Its hash must cover the served content. |
| `context.budget` | `used_tokens` is not reproducible without renderer/tokenizer identities and a counting boundary. String omissions cannot distinguish item identity, reason, size, or retrieval handle. See C. |
| `envelope.write` | Add owner and expected input digest, or reference a snapshot manifest containing them. A union envelope without attribution can let a task edit another declaration’s files. Define moves, deletion authority, and retained-output restrictions explicitly. |
| `envelope.create_under` | Roots need owner identity, collision rules, and canonical path semantics. Creation permission must never imply adoption. |
| `envelope.delete` | Bare paths lose output identity. Use owned-output identities plus expected paths/digests. Make an empty list explicit. |
| `envelope.allowed_imports` | A module specifier grants too much: `ui-sdk` permits every export, while the edges grant particular runtime exports. Specify resolved modules, allowed exports, import mode, and granting edge. Resolve aliases and re-exports before checking. |
| `requirements` | Use SPEC’s explicit `{target, requirement}` subject pair everywhere. Add contract/context hashes, effective policy provenance, and potentially several test bindings. |
| `requirements.status` | Label it **baseline evidence**, with its inputs. An old `tested` label is not proof about this candidate. Keep `new` as change metadata rather than another evidence strength. |
| `requirements.test` | A single `{output, case}` cannot express several tests jointly covering one requirement. Include the check/adapter identity and stable test identifiers. Reject ambiguous case names. |
| `checks` | `id/hash` needs pinned execution semantics: runner, arguments, working directory, timeout, environment profile, and relevant inputs. These may live in a referenced manifest. `self_check` must not authorize arbitrary commands supplied by the agent. |
| `result_path` | It sits outside the ordinary write envelope. Define a separate protocol-artifact area excluded from publication and owned-output claims. Use a unique path per dispatch. |
| `tools.mcp` | A pseudo-endpoint string does not describe interoperable stdio startup, arguments, protocol version, or task authorization. Use a transport descriptor or a runner-established connection. |
| `$defs.digest` | Four hex characters are suitable for display, not authoritative binding. Require full digests with a named algorithm; distinguish content digests from Git object IDs. |
| `$defs.address/member` | Syntax checks are insufficient. Validate existence, addressable collection, visibility, and authority against the bound graph. |
| Arrays generally | Require semantic uniqueness: subject pairs, check IDs, output keys, roots, and bindings. `uniqueItems` alone cannot enforce uniqueness by selected properties. |

The first addition should bind each dispatch. This is a schema fragment; the referenced digest definitions should use full, tagged digests:

```json
{
  "required": ["dispatch_id", "binding"],
  "properties": {
    "dispatch_id": { "type": "string", "minLength": 1 },
    "binding": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "order_hash",
        "plan_hash",
        "input_snapshot",
        "graph_hash",
        "lock_hash",
        "envelope_hash"
      ],
      "properties": {
        "order_hash": { "$ref": "#/$defs/digest" },
        "plan_hash": { "$ref": "#/$defs/digest" },
        "input_snapshot": { "$ref": "#/$defs/digest" },
        "graph_hash": { "$ref": "#/$defs/digest" },
        "lock_hash": { "$ref": "#/$defs/digest" },
        "envelope_hash": { "$ref": "#/$defs/digest" }
      }
    }
  }
}
```

Define `order_hash` over canonical order bytes with that field excluded. The result must echo `dispatch_id` and `order_hash`; the CLI compares them with the outstanding dispatch. The CLI—not the agent—records the resulting candidate snapshot.

For [result.schema.json](/Users/mohebifar/dev/intentlang/schemas/result.schema.json):

| Field | Problem and concrete change |
|---|---|
| `schema`, `task`, `attempt` | Add the dispatch/order binding above. Schema validation cannot establish that a result belongs to the running order. |
| `status` | `done` is easily mistaken for success. Prefer `submitted`; it means the claim is ready for verification. Missing or malformed output is a transport/protocol failure, not another agent verdict. |
| `summary`, `notes` | Useful commentary, never evidence. Bound individual and aggregate sizes. Render them as untrusted content. |
| `created` | These are ownership-registration claims, so they cannot simply be ignored with the touched-file list. Require owner identity for coordinated orders; reject key collisions, duplicate paths, escapes, symlink aliases, and claims on pre-existing files. |
| `created` lifecycle | No way to relocate an already-created output while preserving its key. Add explicit create/relocate/remove claims, constrained by planned authority. The CLI derives actual filesystem changes independently. |
| `tests` | Use explicit subjects and test sets. Require referenced requirements/tests to exist. `change` is agent-reported and mechanically derivable; remove it or mark it advisory. |
| `regression_test` | Requires a test even when reproduction is impossible, and permits both a test and `not_automatable`. Replace it with a discriminated union. |
| `blocked.reason` | Mixes contract questions, scope requests, environment failures, and claimed infeasibility. These need different resolution paths. |
| `blocked.question`, `requirements` | Require a concrete blocker explanation and affected subjects. Requirement IDs alone cannot identify the target of a rule requirement. |
| `blocked.proposal` | `spec_patch` and `use` do not require a patch; `none` can carry one. Use discriminated branches. A spec patch must identify its base revision and permitted spec paths. |
| `blocked.proposal.envelope` | Widening an envelope requires an amended plan, not necessarily a spec edit. Represent requested authority structurally. |
| `allOf` | The status/blocked conditionals are useful. Add equivalent repair/proposal consistency rules; cross-order checks remain CLI responsibilities. |

A repair result could use this union:

```json
{
  "regression": {
    "oneOf": [
      {
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "tests"],
        "properties": {
          "kind": { "const": "automated" },
          "tests": {
            "type": "array",
            "minItems": 1,
            "items": { "$ref": "#/$defs/test_locator" }
          }
        }
      },
      {
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "reason"],
        "properties": {
          "kind": { "const": "not_automatable" },
          "reason": { "type": "string", "minLength": 1 }
        }
      }
    ]
  }
}
```

For the **order**, under the current single-cause shape:

```json
{
  "allOf": [
    {
      "if": {
        "properties": {
          "declaration": {
            "properties": { "cause": { "const": "repair" } },
            "required": ["cause"]
          }
        }
      },
      "then": { "required": ["repair"] },
      "else": { "not": { "required": ["repair"] } }
    }
  ]
}
```

For automated repair evidence, the CLI must run the **candidate regression test against the pre-fix implementation**, then against the candidate, under the same check definition. Merely running the old tree’s old tests does not establish reproduction. Record the exact test-overlay recipe and both receipts; the reviewer must confirm that the original failure demonstrates the cited mismatch.

The worked examples are **substantially aligned with tx-4, but not a complete compiled order**:

- The added link, `ui.link`, `expect.forgot_link`, and four proposed test bindings agree with the printed plan.
- Including the forgot-password route without requiring tx-3 to finish is correct: `mentions` does not order work.
- The inherited auth prompt and skill are supported by the earlier module example in SPEC.
- The consumed contracts for `ui.form`, `ui.input`, and `ui.button` are absent. Only the newly added `ui.link` appears. Required context must include unchanged dependencies too.
- `auth.sign_in`’s `behaviour` contains only its errors, omitting its inherited normative context. SPEC explicitly includes that context in the behaviour projection.
- The texts labeled canonical retain local references such as `sign_in` and `@{forgot_password}`. Either emit resolved canonical content or call these source-oriented renderings and supply a resolution map.
- `spec.diff` is a readable summary, not the unified diff its schema description promises.
- `allowed_imports` authorizes the whole `ui-sdk` module rather than its granted exports.
- The `tested` policies need an identified default/project/package source; the worked declaration’s shorthand expectations do not themselves state that policy.
- `used_tokens: 4120` cannot be reproduced from the supplied protocol.
- The result’s route-constant claim is unverified. A local constant is permissible; importing the forgot-password page through `mentions` would not be. No code accompanies the example to distinguish them.

There are also two SPEC-level inconsistencies: “agents receive their module in full” conflicts with optional sibling declarations, and the interface’s single-declaration task conflicts with coordinated orders for cycles.

**B. Retries, delivery, and blocked scenarios**

**An agent uses `blocked` to avoid a difficult test.** It claims “missing dependency” even though the installed stack already provides the needed functionality.

Stopping mutation is reasonable. Accepting the explanation as an established blocker is not. Require supporting references and attempted investigation; classify the blocker independently where possible. An unsupported blocker is an unsuccessful attempt, and may consume the existing retry budget. A valid blocker pauses for resolution.

You cannot reliably prove motivation. The protocol can distinguish **a blocker claim** from **an accepted blocker**, and preserve the evidence for that decision. An agent’s claim must never authorize a dependency, weaken a requirement, or widen scope.

**A flaky check fails twice.** Two timeouts under `nextjs.build` trigger a fresh restart, discarding correct work. A later pass rewards repeated execution without explaining the instability.

“Same check” is too coarse. Record check identity, definition hash, snapshot, failure classification, and a normalized failure fingerprint. Separate bounded infrastructure reruns from coding attempts. A product assertion failure goes back to the coder; a runner outage does not request another implementation.

A compact failure record:

```json
{
  "attempt": 2,
  "snapshot": "<full digest>",
  "kind": "check_failed",
  "check": {
    "id": "nextjs.build",
    "definition_hash": "<full digest>"
  },
  "classification": "infrastructure",
  "fingerprint": "<normalized failure digest>",
  "diagnostics": {
    "resource": "artifact:failure-17",
    "digest": "<full digest>"
  }
}
```

**Fresh restart loses a correct partial fix.** Attempt 1 repairs error handling but misses the busy-button test. Attempt 2 fixes the test setup but still fails the same assertion. Attempt 3 resets everything.

Automatic reset after two failures is unjustified. Repeated failure can mean persistent infrastructure trouble, a bad test, an unsatisfied requirement, or incremental progress. Keep `continued` as the default; make reset a documented recovery action with a reason and checkpoint.

For a multi-task transaction, choose explicitly:

- Restart the current task from its entry checkpoint, retaining completed predecessor tasks.
- Restart the entire transaction and rerun its tasks and generators.

Neither may publish partial transaction work. Preserve abandoned candidates as diagnostic patches. A repair checkpoint must include the exact drifted input, not silently restore the last clean commit.

**A blocked answer should not become permanent spec text.** “The package registry is unavailable” is an operational incident. “Use the existing test database for this run” is execution configuration. Neither belongs in `expect` or `prompt`.

Split resolution into:

```json
{
  "resolution": {
    "kind": "spec_change | plan_amendment | environment_recovery",
    "source": "<reviewed patch or operational receipt>",
    "binding": "<affected input identity>"
  }
}
```

Normative behavior belongs in the spec. Authority changes belong in an amended plan. Operational answers belong in explicit, journaled execution inputs, with replanning when they change bound inputs. This avoids an informal side channel without polluting the language.

**Retry history eventually overflows context.** “Every previous failure” cannot mean unlimited logs. Keep every structured failure record inline, with bounded diagnostic excerpts and immutable full-log handles.

Delivery also needs precise lifecycle rules:

- Specify where the rendered prompt goes—stdin is a workable default.
- Use attempt-specific result paths; never accept a result left by a previous dispatch.
- Define timeouts, cancellation, missing output, malformed JSON, size limits, and atomic result completion.
- Stop the process and its descendants before freezing the candidate for verification.
- A nonzero exit need not invalidate a complete claim, but it cannot substitute for one. Define how timeout/signal termination affects acceptance.
- Keep protocol files outside the published code diff.
- Journal attempt checkpoints and failures. SPEC’s existing crash-resume rule starts a fresh candidate; retries within a live execution must not silently change that rule.

**C. Context assembly**

“Required content is never truncated” is sound. **“Every required byte must be in the initial prompt” is not a workable general composition rule.**

Forty actions may fit or may exceed the budget before implementation code, tool responses, retry feedback, and output space are considered. A plan-time error is honest, but makes ordinary dependency fan-out a language usability limit.

I would change the guarantee to:

> Every required item is available in full at an immutable, authorized resource. The initial prompt contains the complete manifest and the mandatory working context.

Always inline the task’s own contract and diff, inherited normative context, requirement texts, authority, and relevant changed dependency projections. Other complete dependency projections can be fetched individually. Do not replace normative text with generated summaries.

If the editor retains fully inline required context, document this as a hard supported-size limit. The error should name the largest contributors, not just the declaration.

For a deterministic optional packing policy, I would omit items in this order:

1. Unreferenced sibling declaration bodies.
2. Supplemental catalog narrative and examples.
3. Non-normative hints.
4. Skill bodies.

Keep identities and retrieval handles. A consumed catalog contract is **required**, even when its surrounding documentation is optional.

Within each class, remove the largest whole item first, breaking ties by canonical resource ID. Pin this policy version. Never truncate an item mid-sentence.

```json
{
  "budget": {
    "policy": "context-pack/1",
    "renderer": "work-order-markdown/1",
    "tokenizer": "<pinned identifier and revision>",
    "input_limit_tokens": 60000,
    "reserved_output_tokens": 8000,
    "reserved_feedback_tokens": 4000,
    "used_input_tokens": 4120,
    "omitted": [
      {
        "id": "skill:authentication.skills/better-auth",
        "class": "skill",
        "reason": "budget",
        "tokens": 6200,
        "resource": "context:skill-12",
        "digest": "<full digest>"
      }
    ]
  }
}
```

Also define:

- Stable ordering and deduplication of repeated projections.
- Project → ancestor modules → local module ordering; stable rule ordering.
- Whether a projection comes from the applied graph or desired graph. Runtime dependencies must agree with the applied graph plus current transaction; `mentions` may expose a pending declaration’s desired route.
- The counting boundary, including prompt wrappers and tool descriptions.
- Bounded retry metadata and tool-response pagination.

Plan-time counting can prove the initial dispatch fits. It cannot predict arbitrary future diagnostics or tool responses; those require runtime limits and resource handles.

**D. Reviewer protocol**

Use a separate order and verdict format. A new reviewer session may use the same model, but it must not have authored the implementation and must have no candidate write authority.

The order of execution should be:

1. Complete coding and generator tasks.
2. Freeze the candidate; validate ownership, envelope, and result bindings.
3. Run CLI checks and repair reproduction.
4. Dispatch conformance and coverage reviews against that frozen snapshot.
5. If work changes afterward, invalidate affected runs and reviews, then repeat the relevant obligations.
6. Publish only when every obligation is satisfied.

Review order sketch; angle-bracket values indicate typed placeholders:

```json
{
  "schema": "intentlang.review-order/1",
  "dispatch_id": "<CLI-issued identity>",
  "binding": {
    "plan_hash": "<digest>",
    "candidate_snapshot": "<digest>",
    "graph_hash": "<digest>",
    "lock_hash": "<digest>"
  },
  "context_manifest": "<immutable manifest resource>",
  "subjects": [
    {
      "id": "subject-1",
      "target": "auth.login",
      "requirement": "auth.login#expect.forgot_link",
      "contract_hash": "<digest>",
      "normative_context_hash": "<digest>",
      "policy": "tested",
      "assessments": ["conformance", "coverage"],
      "coverage_candidates": [
        {
          "id": "binding-1",
          "tests": [
            {
              "output": "auth.login#outputs.tests",
              "test_id": "forgot_link",
              "definition_hash": "<CLI-derived digest>",
              "check_id": "tests:auth.login#outputs.tests"
            }
          ],
          "run_receipts": ["run:17"]
        }
      ],
      "repair": null
    }
  ]
}
```

Reviewers also need subjects for normative prompt obligations without named `expect` members. Represent these as compiler-issued context subjects with source/hash identities. They are protocol identities, not new user-authored expectation names.

Verdict sketch:

```json
{
  "schema": "intentlang.review-verdict/1",
  "dispatch_id": "<matching dispatch>",
  "order_hash": "<matching digest>",
  "subjects": [
    {
      "id": "subject-1",
      "conformance": {
        "verdict": "conforms",
        "reason": "The rendered link targets the declared reset route.",
        "evidence": [
          {
            "resource": "candidate:src/app/login/_parts/LoginForm.tsx",
            "locator": "<snapshot-bound location>"
          }
        ]
      },
      "coverage": [
        {
          "binding": "binding-1",
          "verdict": "sufficient",
          "reason": "The test checks the rendered destination."
        }
      ]
    }
  ]
}
```

Conformance values: `conforms`, `does_not_conform`, `cannot_determine`. Coverage values: `sufficient`, `insufficient`, `cannot_determine`.

Important acceptance rules:

- Require exactly one verdict for every requested subject and assessment. Reject duplicates, omissions, and unsolicited subjects.
- Batch by shared change/context, but preserve per-subject answers. No batch-wide “approved” shortcut.
- Judge a **set of tests** when several jointly cover a requirement.
- Require a concrete mismatch for nonconformance, and missing information for `cannot_determine`.
- For repairs, assess whether the regression follows from the cited requirement and whether the pre-fix failure demonstrates that mismatch.
- Keep run results in CLI receipts. Reviewers neither manufacture passing runs nor upgrade evidence strength.
- `conforms` does not satisfy a `tested` policy. Insufficient coverage leaves the policy unsatisfied; it is not automatically proof that the implementation is wrong.
- Derive reviewer identity and independence from runner provenance, not a self-reported `by` string.

Large review batches may be partitioned by subject. Publication still requires the complete expected verdict set on the final candidate.

**E. MCP surface**

The proposed surface is small, but “all read-only” is inaccurate: `run_self_check` executes code that can write files, contact services, and start processes.

| Tool | Missing or dangerous behavior |
|---|---|
| `get_declaration(address, projection)` | Bind to an immutable graph/snapshot and return projection hash, provenance, and old/new versions where relevant. Support paging. Define whether module visibility or actual edges authorize each projection. |
| `read_skill(name, path)` | An arbitrary path creates traversal and unrelated-file disclosure risks. Accept a manifest-issued resource ID, or restrict paths beneath the pinned skill package. Serve bytes matching its digest. |
| `list_omitted()` | Return authorized descriptors and retrieval handles. As drafted, some omitted material—catalog docs and siblings—has no clearly matching retrieval method. |
| `run_self_check(id)` | Classify as execution, with allowlisted definitions, bounded resources, cancellation, and isolated scratch space. Distinguish self-check receipts from authoritative final runs. |
| `validate_result(draft)` | Return the candidate snapshot it inspected and explicit validation stages. It cannot establish coverage quality or promise publication. Revalidate after process termination. |

I would add:

```json
{
  "read_context": {
    "resource": "<manifest-issued ID>",
    "cursor": null
  },
  "get_task": {
    "include": ["obligations", "ownership", "entry_points"]
  },
  "get_diagnostic": {
    "receipt": "<authorized receipt ID>",
    "cursor": null
  }
}
```

`get_task` should expose obligations and provenance, not mutable authority controls. Snapshot-bound code/test reads are also needed for reviewers unless the runner supplies a read-only filesystem.

Two boundaries need explicit wording:

- **Projection filtering is not confidentiality** while the agent has unrestricted repository reads. It is context selection unless filesystem access enforces the same restriction.
- **A final diff cannot prove that no forbidden write occurred.** An agent can change a check script, run it, then restore it. If the promise concerns all writes, enforce filesystem/process isolation. Otherwise state that the CLI validates the submitted final tree, and run authoritative checks from pinned, protected definitions against that tree.

Expose contracts and useful diagnostics. Keep credentials, runner authority, and mutable evidence storage outside the candidate. Test quality remains a reviewer responsibility even when the check runner is protected.

**Priorities**

1. **P0 — Bind every order, result, run, and verdict to exact identities and snapshots.** Define protected protocol artifacts and candidate freezing.
2. **P0 — Represent coordinated transactions and per-owner authority.** Resolve the single-declaration/order-cycle contradiction.
3. **P0 — Specify reviewer orders and per-subject conformance/coverage verdicts.** Publication must enforce completeness and freshness.
4. **P0 — Complete repair semantics.** Require revision-bound repair inputs and reproducible pre-fix/candidate test receipts.
5. **P1 — Replace “same check twice ⇒ fresh” with classified failures and explicit checkpoints.** Separate infrastructure retries from implementation attempts.
6. **P1 — Treat blockers as claims with distinct spec, authority, and operational resolution paths.**
7. **P1 — Make context packing reproducible and all omitted content retrievable.** Reconcile full-module visibility with optional siblings.
8. **P1 — Define import-export grants, output lifecycle claims, and path validation.**
9. **P2 — Correct the worked order’s missing contracts, incomplete behaviour projection, pseudo-diff, and unsupported token accounting; then publish executable protocol fixtures.**