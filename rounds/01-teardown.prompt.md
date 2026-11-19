# Round 1: whole-design teardown

You are a critic and co-designer for a spec programming language (working name
"intentlang"). Read these two files in full before answering:

- `SPEC.md` — the current design draft (v0).
- `REVIEW.md` — the author's self-review: a score table, ten concrete flaws, and a
  prioritised fix list.

You are read-only. Do not modify files. Answer in Markdown.

## Question

What is wrong or missing in this design that REVIEW.md did **not** already catch?

Do not repeat REVIEW.md's ten flaws unless you think the review misjudged one (wrong
severity, wrong fix). Look in particular at:

1. **The core bet.** Is "spec graph + per-node state + plan/apply + scoped agent edits"
   coherent? Where does the Terraform/Pulumi analogy break down when the "provider" is a
   non-deterministic agent writing code that humans also edit?
2. **Identity and hashing** (SPEC.md "Core model", "State record"). Are the interface /
   body / context hash boundaries well defined? What changes fall between them?
3. **Plan semantics** ("State, plan and apply"). Find concrete scenarios where the action
   table gives the wrong answer, no answer, or two answers. Consider ordering, waves,
   partial failure, merge of parallel worktrees, and branch merges of state files.
4. **Kinds and the core/package split** ("Definition kinds", "Packages and providers").
   Is the v0 kind list right? What would you cut from v0? What is missing?
5. **The worked example.** Is it internally consistent with the rest of the spec (names,
   imports, scoping, fields used versus fields defined, the plan output)?
6. **Scope creep.** Which parts of v0 should be cut or deferred so the first milestone
   tests the riskiest assumption?

## Format

For each issue:

- **Title** and a severity (critical / major / minor).
- **Where**: the SPEC.md section.
- **Problem**: one or two sentences, with a concrete failure scenario.
- **Proposal**: a concrete alternative, with example syntax or JSON where relevant.

Then end with:

- **Cut list**: what you would remove or defer from v0, and why.
- **Top 5**: the five changes you would make first, in order.

Be adversarial and specific. Prefer ten sharp issues over thirty vague ones.
