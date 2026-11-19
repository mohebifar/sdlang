# Agent protocol

Companion to `SPEC.md` ("Agent interface"). This is the reference for the four protocol
documents; the JSON Schemas in `schemas/` are normative, and `examples/` holds one worked
example of each for tx-4 of SPEC.md's worked example. `tools/make_examples.py` generates
the examples; `tools/check_examples.py` validates them, recomputes their binding hashes and
checks the cross-document rules a schema cannot express.

| Document | Schema | Written by | Purpose |
| --- | --- | --- | --- |
| Work order | `schemas/work-order.schema.json` | CLI | One coding dispatch: subjects, spec diffs, context, envelope, requirements, checks |
| Result | `schemas/result.schema.json` | coding agent | `submitted` or `blocked`, created-output claims, proposed test bindings, repair regression |
| Review order | `schemas/review-order.schema.json` | CLI | Per-subject conformance, coverage and regression assessments against a frozen candidate |
| Review verdict | `schemas/review-verdict.schema.json` | reviewer agent | One verdict per subject and assessment |

Shared definitions (digests, addresses, members, subjects, test locators, policies) are in
`schemas/common.schema.json`.

## Binding

Every order carries a `dispatch` id unique across plans, revisions and attempts, and a
`binding`: plan hash, transaction, base commit, the exact input snapshot (which differs
from the commit for a repair on drifted files, or a later task in a transaction), graph and
lock hashes, and `order_hash`, the sha256 of the order's canonical JSON with that field
removed. A reply must echo `dispatch` and `order_hash`; a reply that does not match the
outstanding dispatch is discarded. Digests are full and tagged (`sha256:...`, `git:...`).

## Life of one dispatch

1. The CLI writes the order to the protocol area `.spec/run/<dispatch>/`, which is never
   published or owned, renders it as Markdown on the agent's stdin, and runs the agent
   command (`spec apply --agent "<command>"`) in the candidate worktree with
   `SPEC_WORK_ORDER` and `SPEC_RESULT` set.
2. The agent edits files and may run the checks marked `self_check` through the MCP
   server. It writes its result atomically at `result_path` and exits.
3. On exit or timeout the CLI stops the agent's whole process tree and freezes the
   candidate. Exit status is informational: a non-zero exit with a valid result is still a
   claim, and a zero exit with no valid result is a protocol failure.
4. The CLI derives the diff itself, checks it against each subject's envelope, ownership
   claims and allowed imports (resolved to export level), and runs every check from pinned
   definitions outside the worktree, so an agent cannot weaken a check by editing it.
5. It runs the repair reproduction if any (the candidate's regression test against the
   pre-fix snapshot, then against the candidate), then dispatches reviews against the
   frozen snapshot. If the candidate changes afterwards, affected runs and reviews are
   invalidated and repeated.

A coordinated transaction (an ordering cycle) is one order with several subjects, each
with its own envelope, so authority stays attributed to an owner.

## Retries

`attempt.max` defaults to 3. A failed attempt is recorded as a classified failure
(`implementation`, `test`, `infrastructure`, `protocol`) with the check's definition hash,
the snapshot, a normalised failure fingerprint, a bounded excerpt and a handle to the full
log. Infrastructure failures are rerun by the CLI a bounded number of times and never
consume a coding attempt. The next coding attempt continues in the same candidate with
every structured failure inline. Restarting from a checkpoint (the task's entry, or the
whole transaction's) is an explicit recovery action with a recorded reason, never automatic,
and abandoned candidates are kept as patches. Nothing from a failed transaction publishes.

## Blocked

`blocked` is a claim the CLI classifies, not an accepted fact. Each blocker kind names what
would resolve it and the evidence the agent gathered:

| Blocker | Example | Resolution |
| --- | --- | --- |
| `spec_question` | "Should the link open in a new tab?" | A reviewed spec change (the agent may attach a patch); then re-plan |
| `authority_request` | a dependency, an extra write path, an import | An amended plan, or a `use` added to the spec |
| `environment` | the package registry is down | Operational recovery, journaled as an execution input |

A blocker whose evidence does not support it (the requested package is already in the
lock) is rejected and counts as a failed attempt. An accepted blocker stops the
transaction. Behaviour answers belong in the spec; scope answers belong in an amended plan;
operational answers never become spec text.

## Context assembly

Context is assembled deterministically from the bound graph:

- **Inline, always, never truncated:** each subject's canonical spec, previous spec, diff
  and name resolution; inherited normative context (project, then ancestor modules
  outermost first, then matching rules by address) with previous text where it changed;
  every requirement; the envelope; and every consumed or observed projection of what the
  subjects reference, changed or not.
- **Required but fetched:** current file contents and other required items too large to
  inline are listed in `context.manifest` with immutable resource handles and served in
  full by the `read_context` tool. Required content is never summarised.
- **Optional, packed by policy `context-pack/1`:** omitted in this order when the budget
  is short: unreferenced sibling declarations, catalog documentation, non-normative hints,
  skill bodies; within a class, largest first, ties by id. Omitted items stay in the
  manifest and remain fetchable.

The budget records the renderer, tokenizer, limits and reserved output and feedback
space, so the count is reproducible. If the inline part alone exceeds the limit, `spec
plan` fails and names the largest contributors.

## Reviews

The reviewer is a separate session with no write authority that wrote none of the
candidate's code; its independence is established by the runner's provenance, not by
anything it reports. A review order can batch many subjects, but the verdict must answer
every requested subject and assessment exactly once: conformance (`conforms`,
`does_not_conform` with the mismatch, `cannot_determine` with what is missing), coverage of
each candidate test set (`sufficient`, `insufficient`, `cannot_determine`), and for repairs
whether the regression test follows from the requirement and demonstrates the observation.
Normative prompt text with no `expect` member is reviewed as a compiler-issued context
subject. A verdict never creates a passing run and never upgrades evidence to `tested`.

## MCP tools

| Tool | Class | Does |
| --- | --- | --- |
| `get_task` | read | The order's obligations, ownership and entry points |
| `get_declaration(address, projection)` | read | A projection from the bound graph, if the subjects' edges or module visibility allow it, with its hash |
| `read_context(resource, cursor)` | read | Any manifest resource, paged, bytes matching its digest |
| `get_diagnostic(receipt, cursor)` | read | Full logs of an earlier failure |
| `run_self_check(id)` | execute | A `self_check` check from its pinned definition, sandboxed; its receipt is advisory |
| `validate_result(draft)` | read | Schema, binding, envelope and ownership errors for a draft result against the current candidate |

There are no tools that write state, widen the envelope or record evidence. Two limits are
stated plainly. Projection filtering selects context; it is not confidentiality while the
agent can read the repository. And validating the final tree cannot prove that no
forbidden write happened in between (a check script edited, run and restored), which is
why checks run from protected definitions and, where available, the runner isolates the
filesystem.
