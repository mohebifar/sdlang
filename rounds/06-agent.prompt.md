# Round 6: agent protocol

You are a critic and co-designer for a spec language. Read-only; answer in Markdown.

Read:

- `SPEC.md` "Agent interface", "Plan entries", "Repair", "Plan binding, journal and
  amendments", "Apply", "Ownership and drift".
- `schemas/work-order.schema.json` and `schemas/result.schema.json`: the editor's draft
  JSON Schemas.
- `examples/work-order.json` and `examples/result.json`: one worked example each, for the
  `auth.login` update in SPEC.md's worked example (tx-4).

REVIEW.md scored agent interop 5/10: "The result format, retries, the blocked path and
context assembly are described only in prose."

## Editor's draft positions (attack them)

1. **Delivery.** `spec apply --agent "<command>"` writes the work order to
   `.spec/run/<tx>/<address>.order.json`, renders it to a Markdown prompt, and runs the
   command in the candidate worktree with `SPEC_WORK_ORDER` and `SPEC_RESULT` set. The
   agent writes the result file and exits. Exit code is informational only.
2. **Retries.** Up to `attempt.max` (default 3). After a failure the CLI starts attempt
   n+1 in the same candidate (`workspace: "continued"`) with every previous failure in
   `attempt.previous`. If two consecutive attempts fail the same check, the next attempt
   starts `fresh` from `candidate_base`. Exhausting attempts fails the transaction.
3. **Blocked.** `status: "blocked"` stops the transaction immediately, with no retry. The
   plan shows the question and proposal. Answers go into the spec (a prompt line, an
   `expect` member, a `use`), never into a side channel, and the human re-plans. A
   `spec_patch` proposal is a diff the human may apply after review.
4. **Context assembly.** Deterministic. Required items are never truncated: the
   declaration's spec, diff, normative inherited context, every requirement text, the
   consumed and observed projections of referenced declarations, and the envelope. If
   required items exceed the budget, that is a *plan-time* error naming the declaration.
   Optional items (skill text, catalog docs, sibling declarations in the module) are
   omitted in a fixed priority order, listed in `context.budget.omitted`, and reachable
   through tools.
5. **MCP tool surface**, all read-only: `get_declaration(address, projection)` limited to
   what the declaration's edges and module visibility allow; `read_skill(name, path)`;
   `list_omitted()`; `run_self_check(id)`; `validate_result(draft)`, which returns schema
   errors and envelope violations before the agent finishes. No write, state or envelope
   tools. Files are edited directly in the worktree.
6. **Result is a claim.** The CLI ignores the agent's list of touched files and re-derives
   the diff. `tests` are proposed bindings; a reviewer judges coverage.
7. **Reviewer tasks** (the `review` obligation and coverage judgements) are not yet
   specified. They need their own order and verdict format.

## Questions

A. Attack the schemas field by field: missing fields an agent needs, fields that leak
   information it should not have or that let it game verification, fields that cannot be
   produced deterministically, and anything the CLI cannot actually check. Is the worked
   example consistent with SPEC.md's worked example and its own schema?
B. Attack retries and the blocked path with concrete scenarios: an agent that returns
   `blocked` to avoid hard work; a flaky check; a fresh restart losing a correct partial
   fix; a blocked answer that should not become a permanent spec line.
C. Context assembly: is "required never truncated, else plan-time error" workable for a
   declaration that calls 40 actions? What is the deterministic priority order?
D. Reviewer protocol: propose the review order and verdict formats (schema sketch, not
   full JSON Schema), including batched reviews with per-subject verdicts and coverage
   judgements.
E. MCP surface: what is missing or dangerous?

Give concrete schema changes (JSON fragments). End with a prioritised list.
