# Round 1b: rebuttal

You are a critic and co-designer for a spec language. Context files (read-only):
`SPEC.md` (draft v0), `REVIEW.md` (self-review), and your previous critique in
`rounds/01-teardown.codex.md`. You are read-only; answer in Markdown.

The editor accepts most of your round-1 critique (transactions spanning compatibility
groups, candidate-tree integration checks, multi-dimensional plan entries, plan binding,
consumed-input hashing, typed edges, no capability-substitution promise, evidence
separate from baseline, a stronger milestone). The editor disagrees on four points.
For each, say whether you concede, hold, or refine, and why. Keep it under 900 words.

## 1. Custom syntax should not be cut

You proposed starting from a versioned JSON graph with no grammar. The editor's position:
the JSON graph is the contract and the milestone may drive the runner from hand-written
graph JSON first, but the *design* must still define a surface syntax now, because the
project's purpose is a reviewable human-authored artifact, and reviewers will not review
JSON. A TypeScript-frontend comparison is scheduled for a later round. Is there a concrete
technical reason the grammar work is premature, beyond scheduling?

## 2. `action` stays a core kind

You proposed demoting `action` to adapter vocabulary. The editor's position: `action` is
the only kind whose interface (input/output/errors) is framework-independent and called
from several transports; it is the main exercise for interface propagation and the
compatibility-group transaction you proposed. Without it, every interface edge is
framework-specific. Rebut if you think this is wrong.

## 3. No new `libraries:` field for the conformance rule

Instead of your `libraries: [authentication]`, the editor proposes: a node's *allowed
dependency set* is derived from references it already has. Code owned by node N may import
(a) code owned by any node N references through any typed edge, (b) the library bound to
any capability N references (e.g. `output: authentication.session` permits the
`better-auth` package), and (c) ambient imports its kind declares (e.g. `next/*` for a
`nextjs.page`). Anything else fails conformance. Does this leak? Give a concrete case
where it permits too much or too little.

## 4. The transaction-grouping rule

Proposed rule. Edges carry a relation declared by the kind field they come from (`calls`,
`reads`, `navigates`, `uses`, `mentions`...). Each relation declares (i) which projection
of the target it consumes (e.g. `calls` consumes the action's signature; `navigates`
consumes only the route) and (ii) whether it is an *ordering* edge. Planning:

1. For each changed node, compute which consumed projections changed.
2. Every dependent whose consumed projection changed gets an obligation (`verify` if the
   change is declared backward-compatible by the kind, otherwise `update`).
3. A changed node plus every dependent with an `update` obligation forms one
   *transaction*: one candidate worktree, one combined check run, atomic commit of code
   and all their state records. Overlapping groups merge. Strongly connected components
   of ordering edges also merge into one transaction.
4. `verify`-only dependents run as their own transactions after the group commits; if
   verify fails they escalate to `update` in a new transaction.
5. v0 runs transactions serially; each publishes only after checks pass on the full tree.

Attack this rule. Where does it produce groups that are too large (e.g. a widely-called
action), or fail to group things that must change together? What would you change?
