# Re-evaluation of the design (v0.2)

Written on 2026-11-19 from a full reread of `SPEC.md`, `PROTOCOL.md`, the grammar, the
schemas and the examples. It is a second opinion on the twelve-round result, not a thirteenth
round: nothing in the design was changed. Nothing here is validated either; like every
score before it, this judges a paper design.

Overall: about 7 out of 10 as a design, and unknown as a bet. The round scores of 8 to 9
are fair for what they measured, which was internal consistency in six areas. They did not
measure simplicity, cost, or how a team would live with the tool, and that is where the
design is weakest.

## Scores

| Area | Rounds (editor / Codex) | Now | Why it moved |
| --- | --- | --- | --- |
| Architecture | 9 / 8 | 8 | Coherent and checked on the worked example. Carries more mechanism than the evidence justifies (finding 1). |
| Drift and brownfield | 9 / 9 | 7 | Detection is well defined. Nothing forces it to run, the merge story is thin, and shared code has no owner kind (findings 2, 3). |
| Verification | 9 / 8 | 7 | The currency model is the best idea in the design. The guarantees rest on an LLM coverage judgement and are silent where no requirement was written (finding 4). |
| Grammar and syntax | 9 / 9 | 8 | Complete, and every example parses. The parser mirrors the EBNF by hand, and agents have never seen the language. |
| Agent interop | 9 / 8 | 8 | Results as claims and frozen candidates are right. Assumes a sandboxing runner that is not designed. |
| Package ecosystem | 9 / 8 | 6 | Two adapters exist on paper. Each real adapter needs an export-level import checker, oracle digests and a test reporter (finding 6). |
| Simplicity | not scored | 5 | Finding 1. |
| Cost to build and to run | not scored | 5 | Finding 5. |
| Team workflow (pull requests, CI, merges) | not scored | 5 | Finding 2. |

## Findings

1. **The process rewarded adding mechanism, and nothing pushed back.** Every round closed a
   hole by adding a concept. A user must now hold five projections, six relations with four
   properties each, five changes, four strategies, two obligations, seven kinds of evidence,
   a seven-by-five currency table, four drift resolutions, three lifecycles, contingent
   envelopes and amended plans. `SPEC.md` went from 574 lines to 1852, plus about 3500 lines
   of schema. No area in the rubric scored whether a person can learn this or whether it is
   worth building, so no round ever removed anything.

2. **There is no workflow around the tool.** Drift, ownership and evidence are checked only
   when someone runs `spec plan`. Missing: a read-only CI gate; who runs `spec apply` and on
   which branch; how textual merge conflicts in `.spec/state/*.json` are resolved before
   `--reconcile` can run; who reruns checks on the merged tree. `provenance.commit` and the
   journal name commits that a squash merge or rebase makes unreachable, which degrades
   every later change classification to "incompatible". Evidence is an unsigned JSON file,
   so it is only as trustworthy as the machine that wrote it.

3. **Shared code has no home.** The kinds are pages, endpoints, actions, entities and UI
   components. A date formatter, a React hook or a shared type module used by two pages
   fits none of them, and the import rule forbids one page importing another page's
   `_parts/`. The only answer in the document is a local one-slot kind (`acme.lib` in the
   adoption walk-through). Any refactor that moves code between declarations needs a spec
   edit and a `moved` statement, which contradicts "the spec tree follows features, not
   code layout".

4. **Verification is silent where the spec is silent, and the spec's own example shows it.**
   - `auth.sign_in` has no `prompt` and no `expect`, in a project set to `verify: "tested"`.
     It has no requirements, so nothing about its behaviour is ever tested or reviewed, and
     no report says so.
   - Declared `errors` are part of the contract but are not evidence subjects, so nothing
     requires a test that `invalid_credentials` is ever returned.
   - `request_reset` carries "Never reveal whether the email exists" in a `hint`, which is
     non-normative by definition. A security requirement is filed where changes to it cause
     no obligation. If the document's author misfiles one, users will too.
   - `tested` means "an LLM reviewer judged the tests sufficient". The milestone measures
     this, which is right, but until then every `tested` label is a model's opinion.

5. **Running cost is not modelled.** Transactions are serial and each must pass
   project-scope checks, so the four-transaction worked example runs `next build` four
   times. An inherited prompt edit costs one review per declaration beneath it. When an
   adapter cannot compute a per-case oracle closure it falls back to the whole harness, so
   one changed test helper makes every coverage judgement stale and due for re-review. The
   plan prints these counts, but the design never asks whether they are affordable.

6. **Everything depends on adapters that are expensive to write.** O.1 (the check input
   and result schemas) is still open, so an adapter author cannot yet write a conformance
   check from the documents alone.

7. **The milestone's control arm is too weak.** It compares against "the same edits as
   plain instructions with no graph or state". The real alternative is a current coding
   agent with the repository, its tests, and a Markdown spec file. Beating the weaker
   baseline would not show the language earns its cost.

8. **Small defects in the documents.**
   - `SPEC.md` line 196: the `context` row of the projections table sits after a paragraph
     and renders as stray text.
   - Lines 30 and 42 say evidence names "the tree it ran on"; line 1385 says tree identity
     plays no part.
   - Line 637 lists four package-only forms; there are seven.
   - The `page` kind excerpt (lines 662 to 685) lists three ambient imports and three
     checks; `examples/nextjs-adapter.int` has four of each.
   - Line 1234 says `next.config.js`; the adapter owns `next.config.ts`.
   - Line 1852 still says the language's name is undecided.
   - The manifest example in `SPEC.md` has a `provides` entry that `examples/package.json`
     lacks.

## What holds up

- Stable identity with `moved`, including members and output files.
- Evidence kept apart from the accepted code, and kept current by the inputs it used.
- The agent's result treated as a claim, verified on a frozen candidate by a separate
  session.
- The split between an author's own edit, an inherited edit and guidance.
- `acquire` for adopting existing code without pretending it was verified.
- The tables that say what each kind of evidence does not establish.
- A first milestone that is much smaller than the design and has a control arm at all.

## Recommendation

Stop designing and run the milestone. Before that, four things are worth doing, in order:

1. Fix the defects in finding 8.
2. Strengthen the control arm (finding 7), since it decides whether the rest matters.
3. Mark everything outside the milestone as provisional, in one place, so the core a reader
   must learn is a few pages. Do not add to it until the trial reports.
4. Record findings 2, 3 and 4 as open design items next to O.1 to O.6: a CI gate and merge
   workflow, a kind for shared code, and a report of declarations with no requirements.
