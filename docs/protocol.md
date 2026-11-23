# Agent protocol

Companion to `docs/spec.md` ("Agent interface"). This is the reference for the four protocol
documents; the JSON Schemas in `schemas/` are normative, and `examples/` holds one worked
example of each for tx-4 of docs/spec.md's worked example. `tools/make_examples.py` generates
the examples; `tools/check_examples.py` validates them, recomputes their binding hashes and
checks the cross-document rules a schema cannot express.

| Document | Schema | Written by | Purpose |
| --- | --- | --- | --- |
| Work order | `schemas/work-order.schema.json` | CLI | One transaction's coding dispatch: subjects, spec diffs, context, their files, requirements, checks |
| Result | `schemas/result.schema.json` | coding agent | Optional: `blocked`, test bindings and created-output keys the CLI should not infer, repair regression |
| Review order | `schemas/review-order.schema.json` | CLI | Per-subject conformance, coverage and regression assessments against a frozen candidate |
| Review verdict | `schemas/review-verdict.schema.json` | reviewer agent | One verdict per subject and assessment |
| Generator input and output | `schemas/generator.schema.json` | CLI, generator | One deterministic generator run (see Generator protocol) |
| MCP tool calls | `schemas/mcp-tools.schema.json` | agent, CLI | Inputs and outputs of each tool |
| Check input and result | `schemas/check.schema.json` | CLI, package check | One check run (docs/spec.md, Packages → Check protocol) |

Shared definitions (digests, addresses, members, subjects, test locators, policies) are in
`schemas/common.schema.json`. Every document type has a generated example, including
`examples/generator.input.json` and `examples/generator.output.json` for tx-1's migration.

## Cross-document rules

The schemas check each document alone. The CLI also enforces these relational rules;
`tools/check_examples.py` checks the ones marked ✓ on the examples.

| Rule | Between | Failure |
| --- | --- | --- |
| `order_hash` recomputes from the order's canonical JSON ✓ | order | protocol failure, order not dispatched |
| A verdict echoes the outstanding `dispatch` and `order_hash` ✓; a result may leave them out, and names this order's if it gives them | order, reply | reply discarded |
| `attempt.number` ≤ `attempt.max` ✓ | work order | protocol failure |
| Every requirement and test output a result names resolves, by its full member or an unambiguous tail, to one of the order's ✓ | work order, result | that binding ignored; the next attempt is told |
| `regression` has exactly one entry per repair subject, and none without one | work order, result | result invalid |
| Every `outputs` claim names an order subject as owner, or leaves it to the only subject or the only one with that root. `create`: a creation root of that subject, a path that did not exist before. `remove`: an existing created output of that subject | work order, result, candidate | that claim ignored |
| Reference and contract hashes equal the bound graph's projections and members ✓ | work order, graph | order not dispatched |
| Review-order resource handles are bound to the frozen candidate, never reused from the work order ✓ | work order, review order | order not dispatched |
| A verdict answers exactly the requested subjects ✓, each requested assessment once, nothing unsolicited | review order, verdict | verdict invalid |
| Each coverage verdict names a coverage candidate of that subject | review order, verdict | verdict invalid |
| A regression assessment exists exactly when the subject's `assess` includes `regression`, in the same mode (automated or not automatable) as the result's entry | result, review order, verdict | verdict invalid |
| A generator output echoes its input binding ✓ and writes only listed slots and retained roots | generator input, output | generator failure |

## Binding

Every order carries a `dispatch` id unique across plans, revisions and attempts, and a
`binding` with the plan hash, graph and lock hashes, and `order_hash`: the sha256 of the
order's canonical JSON with that field removed. A work order also binds what the agent
starts from: transaction, base commit and the exact input snapshot (which differs from the
commit for a repair on drifted files, or for a later task in a transaction). A review order
instead binds the frozen `candidate_snapshot` it judges; its transaction and base are
recoverable from the plan. A reply must echo `dispatch` and `order_hash`; a reply that does not match the
outstanding dispatch is discarded. Digests are full and tagged (`sha256:...`, `git:...`).

## Life of one dispatch

1. The CLI writes the order to the protocol area `.spec/run/<dispatch>/`, which is never
   published or owned, renders it as Markdown on the agent's stdin, and runs the agent
   command (`sdc apply --agent "<command>"`) in the candidate worktree with
   `SPEC_WORK_ORDER` and `SPEC_RESULT` set. A transaction is one dispatch: every subject it
   codes is in one order, so one agent sees the whole change.
2. The agent edits files, starting from those the order lists for each subject and
   changing any other the change needs, and may run the checks marked `self_check` through
   the MCP server. It may write a result at `result_path`, then exits.
3. On exit or timeout the CLI stops the agent's whole process tree and freezes the
   candidate. Exit status is informational; without a result the candidate is the claim.
4. The CLI derives the diff itself, rejects changes to protected files (docs/spec.md, Plan
   binding), attributes each changed file to its owner, so declarations outside the
   transaction join it as touched, keys new files under creation roots, binds each
   requirement to the cases tagged with its key unless the result bound it, and runs every
   check from pinned definitions outside the worktree, so an agent cannot weaken a check by
   editing it.
5. It runs the repair reproduction if any (the candidate's regression test against the
   pre-fix snapshot, then against the candidate), then dispatches reviews against the
   frozen snapshot. If the candidate changes afterwards, affected runs and reviews are
   invalidated and repeated.

A result is only needed for what the candidate cannot say: `blocked` with a reason, a
test case bound to a requirement whose key it does not carry, a chosen key for a created
output, a repair's regression entry. Requirements and outputs may be named by their key
(`same_message`, `tests`) or any unambiguous tail of the member.

## Retries

`attempt.max` defaults to 3. A failed attempt is recorded as a classified failure
(`implementation`, `test`, `infrastructure`, `protocol`; tests a reviewer finds
insufficient are a `test` failure) with the check's definition hash,
the snapshot, a normalised failure fingerprint, a bounded excerpt and a handle to the full
log. Infrastructure failures are rerun by the CLI a bounded number of times and never
consume a coding attempt. The next coding attempt continues in the same candidate with
every structured failure inline. Restarting from a checkpoint (the task's entry, or the
whole transaction's) is an explicit recovery action with a recorded reason, never automatic,
and abandoned candidates are kept as patches. Nothing from a failed transaction publishes.

## Blocked

A result's `regression` has one entry per repair subject in the order, either automated
(the tests that must fail on the pre-fix snapshot) or `not_automatable` (a reason and
artifacts).

`blocked` is a claim the CLI classifies, not an accepted fact. Each blocker kind names what
would resolve it and the evidence the agent gathered:

| Blocker | Example | Resolution |
| --- | --- | --- |
| `spec_question` | "Should the link open in a new tab?" | A reviewed spec change (the agent may attach a patch); then re-plan |
| `authority_request` | a dependency, or a change to a protected file | An amended plan, or a `use` added to the spec |
| `environment` | the package registry is down | Operational recovery, journaled as an execution input |

A blocker whose evidence does not support it (the requested package is already in the
lock) is rejected and counts as a failed attempt. An accepted blocker stops the
transaction. Behaviour answers belong in the spec; scope answers belong in an amended plan;
operational answers never become spec text.

## Context assembly

Context is assembled deterministically from the bound graph:

- **Inline, always, never truncated** (`context.inherited`, `context.references` and the
  subjects; the schema has no other way to carry them): each subject's canonical spec,
  previous spec, diff
  and name resolution; inherited normative context (project, then ancestor modules
  outermost first, then matching rules by address) with previous text where it changed;
  every requirement; the subjects' files; and every consumed or observed projection of what the
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
subject, `<target>#context.h_<first 12 hex digits of the text's hash>`. A review order
carries the same inline context contract as a work order for what the reviewer must judge
against: inherited normative text and the resolved projections the subjects reference,
with previous text where it changed. For a repair, the regression assessment is either
automated (pre-fix and candidate run receipts) or `not_automatable`, in which case the
reviewer receives the agent's reason and its before and after artifacts and the evidence
can only ever be review. Resource handles in a
review order are bound to the frozen candidate, never reused from the work order, whose
handles name the pre-edit bytes. Coverage candidates carry each test's oracle digest. A verdict never creates a passing run and never upgrades evidence to `tested`.

## MCP tools

Inputs and outputs of every tool are defined in `schemas/mcp-tools.schema.json`.

| Tool | Class | Does |
| --- | --- | --- |
| `get_task` | read | The order's obligations, ownership and entry points |
| `get_declaration(address, projection)` | read | A projection of any declaration from the bound graph, with its hash |
| `read_context(resource, cursor)` | read | Any manifest resource, paged, bytes matching its digest |
| `get_diagnostic(receipt, cursor)` | read | Full logs of an earlier failure |
| `run_self_check(id)` | execute | A `self_check` check from its pinned definition, sandboxed; its receipt is advisory |
| `validate_result(draft)` | read | Optional: names a draft result gives that resolve to nothing, protected files changed, and notes on what else the change reaches |

There are no tools that write state or record evidence. Two limits are
stated plainly. Projection filtering selects context; it is not confidentiality while the
agent can read the repository. And validating the final tree cannot prove that no
forbidden write happened in between (a check script edited, run and restored), which is
why checks run from protected definitions and, where available, the runner isolates the
filesystem.

## Generator protocol

A generator is run like a check (see docs/spec.md, Packages → Checks and generators) with one
JSON document on stdin and one on stdout, both defined in `schemas/generator.schema.json`.
The input carries the dispatch and binding, the subjects (old and new canonical graph
entries, plus the aggregate for owners with `collects` or contributions), the envelope of
writable output slots and retained roots with their existing artifacts, explicit decisions
(such as `rename_field` pairs from `moved`), a fixed `generation_id` for every generated
name and timestamp, and a staging directory. The output echoes the binding and reports
`written` (files and retained artifacts with checksums and dependencies), `unsupported`
(with a reason; never silently handed to an agent) or `needs_decision` (with questions; the
plan is amended). Unlike an agent, a generator writes only inside its envelope; the CLI
validates that, the digests, then runs checks.
