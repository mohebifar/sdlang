# Round 13: adversarial review of the second-review changes

You are an independent, adversarial reviewer of a spec-language design called int lang.
Read-only; answer in Markdown. Do **not** open any file under `rounds/`.

## Background

After twelve design rounds, a second review (`REVIEW-2.md`) rated the design about 7/10 and
listed eight findings. Finding 8 (document defects) was fixed first. Findings 1 to 7 were
then addressed in one pass, logged in `DECISIONS.md` as R2.2 to R2.8. Your job is to check
that pass. It was done by one author with no outside review, so assume it contains mistakes.

Read: `REVIEW-2.md`, `DECISIONS.md` (especially R2.1 to R2.8 and O.1 to O.6), `SPEC.md`,
`EXTENSIONS.md` (new), `PROTOCOL.md`, `GRAMMAR.ebnf`, `schemas/*.json` (especially the new
`schemas/check.schema.json`), `examples/*` (especially the `check.*.json` files and the two
`.int` adapters), and `tools/check_examples.py`. You can see the exact change with
`git show fd1ff8f` and `git show 6b3921e`.

## What to answer

1. **Per finding (1 to 7):** is it addressed, partly addressed, or not addressed? One short
   paragraph each, citing the text. Say plainly where a fix is only words (a promise with no
   mechanism), where it moved a problem rather than solving it, or where it made the design
   bigger in a way that works against finding 1.
2. **New defects introduced by the pass.** Look in particular for:
   - contradictions between the new sections and older text (for example plan-cadence
     checks versus "a transaction publishes only when every obligation is satisfied",
     `lib` versus the relations table and Compatibility, `errors` as requirements versus
     the evidence and plan schemas, `.spec/applied/` versus the state record and currency
     rules, `spec check` findings versus the Verification status labels);
   - references to sections that moved to `EXTENSIONS.md` or no longer exist;
   - the check protocol: gaps an adapter author would hit, and whether the examples and
     the checker's new cross-check actually prove what they claim;
   - whether the core / extensions split is coherent: does anything in `SPEC.md`'s core or
     its first milestone depend on something now marked provisional?
3. **Scores.** Score the six original areas from `REVIEW.md` plus the three areas
   `REVIEW-2.md` added (simplicity, cost to build and run, team workflow), 1 to 10, as a
   design draft. One line of reasons each. For any area below 9, name the concrete document
   changes that would raise it.
4. **The three most important things to change next**, in order.

Be calibrated and specific: cite file and line or section for every claim. Do not inflate,
and do not invent defects you cannot point to.
