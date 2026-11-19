**Keep the own syntax, and ship no TypeScript frontend in v0.** Keep the milestone’s graph-JSON-first implementation order.

The strongest reason is the language’s deliberately restricted meaning. Named members establish identity; mentions establish edges; selectors attach requirements; annotations affect projections; imports establish visibility and authority. Those semantics need a compiler regardless of the spelling.

The custom frontend now carries substantial work: newline-sensitive parsing, prose dedent, embedded mentions, template interpolation, selectors, member collections, annotations and module resolution. TypeScript would supply parsing and basic editor support, but most of that semantic work would remain.

A TypeScript frontend has two credible forms:

| Form | Required restrictions | Consequence |
| --- | --- | --- |
| Statically interpreted subset | Literal records, named declarations, restricted imports and compiler-recognised constructors; reject arbitrary calls, spreads, getters and executable expressions | Preserves planning without executing authored code, but creates another language subset and its diagnostics |
| Evaluated JSON-producing builder | Explicit identities, pinned executable closure, controlled inputs, deterministic ordering, resource limits and isolation | A sandbox alone does not establish determinism. It also changes the current promise that planning runs no package code |

Either form must preserve explicit declaration addresses, named expectation keys, output identities and `moved` history. Neither object identity, array position nor evaluation order can supply identity. TypeScript imports must also be translated into intentlang visibility and import grants; host-language imports cannot become authority automatically.

For reviewers, the current syntax exposes the important changes directly: a requirement changed, a reference appeared, or an identity moved. A tightly restricted TypeScript form could preserve that visibility. A programmable builder would require reviewers to understand the computation producing the graph.

The cheapest credible tooling plan is:

1. Finish the canonical graph contract and graph-driven runner first.
2. Use one parser with source spans and preserved comments for compilation, diagnostics and formatting. Keep the EBNF and parser conformance cases together.
3. Ship syntax highlighting plus compiler diagnostics usable from an editor task.
4. Add a deterministic formatter that preserves prose content, mentions and comments.
5. Add a thin language server for diagnostics, completion, hover, definitions and references using the compiler’s resolver. Defer automated rename until it can generate the required `moved` statements correctly.

Reconsider a TypeScript authoring frontend after the milestone demonstrates demand. It should lower into the same graph contract and pass the same semantic fixtures. Building both frontends now would duplicate an unsettled boundary.

**The concrete audit findings follow.**

1. **Contribution metadata is simultaneously required and deferred.**  
   **Files/sections:** [SPEC.md — Packages / Kind definitions](/Users/mohebifar/dev/intentlang/SPEC.md:599), [Contributions](/Users/mohebifar/dev/intentlang/SPEC.md:1099), Worked example.

   Kind definitions have a closed vocabulary, then explicitly defer `accepts`, `collects` and retained-artifact metadata. Contributions nevertheless require owner-defined acceptance schemas, ordering and exclusive keys; the worked database binding requires `collects` and retained migrations.

   An adapter following the closed vocabulary cannot express the worked example. Either define these extensions or clearly separate the later target design from the supported v0 contract. The value/type matrix also needs to admit an **owner-instance constructor**: `main_nav { ... }` targets a declaration, whereas the matrix currently admits constructors of kinds or catalog types.

2. **Guidance-only edits can still trigger implementation work.**  
   **Files/sections:** [SPEC.md — Projections and hashes](/Users/mohebifar/dev/intentlang/SPEC.md:161), Plan entries, Propagation; Packages / Kind definitions.

   `body` includes `hint` and `skills`; an own-projection change triggers `update`. Elsewhere, guidance applies to future work only, and skills have guidance-only propagation. Thus changing a declaration’s own hint can dispatch an implementation task even though changing inherited guidance does not.

   State the exemption explicitly: guidance hashes may change without creating implementation obligations.

3. **Annotation placement and schema rules contradict each other.**  
   **Files/sections:** [SPEC.md — Values and types](/Users/mohebifar/dev/intentlang/SPEC.md:309), [Annotations](/Users/mohebifar/dev/intentlang/SPEC.md:369).

   Values and types restrict annotations to schema records, but Annotations permits declaration annotations and defines `@deprecated`. The annotation example also uses a default inside `parameters`, which is absent from the enumerated schema contexts.

   Separate declaration annotations from schema-field annotations, and include annotation parameters among schema contexts.

4. **The review example points at pre-edit content.**  
   **Files/sections:** [examples/work-order.json — context.manifest](/Users/mohebifar/dev/intentlang/examples/work-order.json:331), [examples/review-order.json — context.manifest](/Users/mohebifar/dev/intentlang/examples/review-order.json:55); PROTOCOL.md / Context assembly.

   The work order binds `LoginForm.tsx` and its test file to immutable handles:

   - `ctx:code:3b42c43ea5ff`
   - `ctx:code:a9314433231f`

   The result claims to change both files, but the review order reuses those handles. Under the immutable-resource contract, they still identify the input bytes. Issue fresh candidate-bound handles for the review.

5. **Compiler-issued context subjects can violate the member schema.**  
   **Files/sections:** [schemas/review-order.schema.json — subjects[].subject](/Users/mohebifar/dev/intentlang/schemas/review-order.schema.json:77), schemas/common.schema.json / `$defs.member`.

   The prescribed identity is `<target>#context.<source-hash-prefix>`. Member keys must begin with a letter or underscore. A hash prefix beginning with a digit therefore produces an invalid subject. Use a form such as `#context.h_<hex>`.

6. **Some valid test identities cannot be moved.**  
   **Files/sections:** schemas/common.schema.json / `$defs.test`; [SPEC.md — Tests](/Users/mohebifar/dev/intentlang/SPEC.md:1257); GRAMMAR.ebnf / Address.

   `case` permits any nonempty string, including `valid-login`, while case moves require identifier-shaped address segments. Such a case is valid in the protocol but cannot appear in the documented move syntax. Restrict case tags consistently or define escaping for member-address segments.

7. **Root-rule addresses incorrectly include the filename.**  
   **Files/sections:** [SPEC.md — Evidence](/Users/mohebifar/dev/intentlang/SPEC.md:200), State record, Worked example.

   `app.app_requires_session` appears in evidence and context examples. The worked rule lives in root-level `spec/app.spec`; filenames carry no identity. Its address is `app_requires_session`, so its requirement is `app_requires_session#expect.redirects`.

8. **The binding prose describes fields that review orders cannot contain.**  
   **Files/sections:** [PROTOCOL.md — Binding](/Users/mohebifar/dev/intentlang/PROTOCOL.md:19); schemas/review-order.schema.json / binding.

   “Every order” is said to carry transaction, base commit and exact input snapshot. The closed review-order binding instead has `candidate_snapshot` and omits the base commit and explicit transaction.

   Describe the common binding guarantees, then distinguish coding inputs from review candidates. Adding redundant fields is unnecessary if the plan binding already supplies the provenance.

9. **Mandatory inline projections have a contradictory protocol representation.**  
   **Files/sections:** PROTOCOL.md / Context assembly; schemas/work-order.schema.json / context.references.

   The prose requires every consumed or observed projection inline. The schema explicitly supports `inline: false` with a resource instead. Choose one contract; currently an implementer can reasonably build either behaviour.

10. **The checker does not exactly implement the normative grammar.**  
    **Files/sections:** GRAMMAR.ebnf / Operand and List; tools/check_examples.py / `operand()`.

    The grammar permits a selector operand list containing record expressions:

    ```spec
    rule r {
      applies_to: select * where config in [{mode: "a"}]
    }
    ```

    `Operand → List → Expr → Record` accepts it; the checker rejects it because list operands recursively use the narrower `operand()` parser. Either narrow the grammar or broaden the parser. This is a syntax discrepancy regardless of subsequent type checking.

**Several used constructs still lack their defining contract.**

| Construct | File and section | What is missing |
| --- | --- | --- |
| Canonical graph JSON and projection hashing | SPEC.md / Projections and hashes; First milestone | No graph format is supplied. “Normalises prose whitespace” does not specify the transformation, and recursive public projections lack a defined treatment of legal reference cycles. The graph-first milestone needs this contract before either frontend. |
| Package discovery and binding schemas | SPEC.md / Scoping and resolution; Packages / Contents | The text does not explain how `import nextjs` discovers npm package `@intentlang/nextjs`. `provides` is described as a capability export map. Nor is there a defined place for a provider to publish the options schema that validates `use ... { options }`. |
| Annotation-definition metadata | SPEC.md / Annotations | Applications enter “the projection the definition names,” but no projection field, default or allowed values are defined. The complete target vocabulary and field-type restrictions are also unspecified. |
| Test-check selection | SPEC.md / Checks and generators; Tests | A check’s `purpose` selects either the build or conformance protocol. Tests introduce a third result protocol without defining how a check selects it. The example’s `tests:<output>` check identifiers also lack a construction rule. |
| Generator and runtime extension protocols | SPEC.md / Checks and generators; Migrations and backfills; Environments and observations | The “versioned core protocol” for generators is not supplied. Binding outputs, retained-artifact declarations and observer registration have no defined extension vocabulary. Agent-written backfill code also needs an explicit owner and producer arrangement alongside generator-exclusive outputs. |
| Configuration and derived status | SPEC.md / Policies; Tests | `test_first: true` has no declared location or scope. Summary labels are called derived, but no derivation handles mixed evidence such as current tests, failed conformance and missing approval. Define the function or omit the summary label. |
| Coverage digest | schemas/review-order.schema.json / coverage_candidates.tests[].definition_hash | Define this as the oracle-closure digest required by SPEC.md, or add that digest explicitly. “Definition hash” currently leaves open whether the reviewer assesses only the case body or its helpers, fixtures and configuration. |

These are missing definitions, rather than objections to the chosen design.

**REVIEW.md flaws 1–10 are mostly fixed in the current text, rather than merely recorded as fixed.**

| Flaw | Verdict | Current fixing section or remaining gap |
| --- | --- | --- |
| 1. Positional expectations | **Fixed** | SPEC.md / Identity, Named members and Moves establish named keys and preserve bindings across moves. |
| 2. No bug-fix flow | **Fixed** | Repair defines unchanged-spec updates, pre-fix reproduction and regression review. |
| 3. Prose misses propagation | **Fixed** | Projections and hashes adds `behaviour`; Propagation gives observing dependents `verify` and `review`. |
| 4. Undefined syntax | **Partially fixed** | Grammar, resolution, selectors, strings, version records and replacement annotation syntax are defined. Annotation metadata, provider options and contribution schemas remain incomplete as identified above. Removed forms such as `@default` need no definition. |
| 5. Kind schema mixed with metadata | **Fixed** | Packages / Kind definitions separates `fields` from output and execution metadata. The extension-vocabulary conflict is a separate remaining defect. |
| 6. Runtime state absent | **Fixed at the semantic level** | Runtime state separates source publication, observations and rollout prerequisites; unreachable means unknown. The package registration/protocol gaps remain. |
| 7. Capability collisions | **Fixed** | Scoping and resolution puts bindings in one table, rejects collisions and supplies import aliases. |
| 8. Fuzzy verification and cost | **Fixed in the rules** | Plan entries, Propagation, Verification and PROTOCOL.md / Reviews define verdicts, escalation and batching. The worked output still omits the promised review accounting. |
| 9. Magical reference cleanup | **Fixed** | Delete explicitly identifies former consumers, contribution owners, enumerated outputs and the guarantee’s limits. |
| 10. Multi-file ownership absent | **Fixed** | Outputs defines slots, creation roots, registered identities and transfers; ownership is explicitly whole-file in v0. |

**The worked example’s main transaction structure is consistent, subject to the missing adapter contracts above. Its output and explanation need these corrections.**

- **Show the actual expectation edit.** “The change” specifies a prompt line and `ui.link`, but the plan additionally reports `expect + forgot_link`. The protocol work order contains that expectation. Include it in the authored delta or display the revised login declaration.
- **Keep tx-1’s two owners.** `Workspace` owns its generated schema output; `database` owns the migration and schema index. Joining them is correct.
- **Keep `dashboard.home` verify-only initially.** The nullable addition is reader-compatible under the stated assumptions. However, a failed check does not automatically authorise a page edit. A confirmed violation escalates to `update`; because the page’s write envelope is outside tx-1’s listed owners, Apply requires an amended plan. The sentence saying it simply “joins tx-1” omits that step.
- **Keep tx-2 before tx-3.** The `calls` edge orders `request_reset` before its page.
- **Keep tx-4 unordered with respect to tx-3.** The mention supplies a desired route projection and grants no import. That is consistent with the work-order `viewpoint` rule.
- **Correct the task accounting.** Three is the number of ordinary coding dispatches. Required coverage and conformance reviews also use agent dispatches. Print review subjects, batches and cost separately, as Propagation promises.
- **Distinguish change counts from obligations.** `2 create, 3 update (2 by generator)` is consistent. `1 verify` means one **verify-only declaration**; it cannot mean only one declaration undergoes verification.
- **Keep rollout separate.** Publishing the migration verifies source. Environment readiness remains unknown without the runner’s receipts. No production migration belongs inside tx-1.

The summary should therefore read along these lines:

```text
Changes: 2 create, 3 update (2 generator), 1 verify-only, 1 drift-blocked
Coding dispatches: 3
Review subjects and batches: <enumerated by the planner>
Contingency: dashboard.home nonconformance requires an amended write envelope
Rollout: workspace_archived_at prerequisite; target readiness unknown
```

The supplied material does not determine an exact review-batch count, so that number should not be invented.

**The three most useful readability cuts are:**

1. **Core model / Evidence duplicates Verification / Tests.** Keep the baseline-versus-evidence distinction and subject identity in Core model. Move the detailed coverage/run currency explanation into Verification and replace the duplicate with a link.
2. **Ownership / Outputs repeats the package-kind example.** Delete its second `export kind page` block and refer to Packages / Kind definitions. Keep the ownership semantics in Outputs.
3. **Agent interface repeats most of PROTOCOL.md.** Replace the seven load-bearing bullets with one paragraph establishing CLI authority, frozen-candidate verification and independent review, plus links to the protocol sections. Keep the distinct material about agents editing specs.

Read-only checks confirmed the 20 `spec` blocks parse, the five embedded JSON blocks parse, and both order hashes and reply bindings recompute. Full schema validation could not be rerun in this environment because `jsonschema` is absent.

**Prioritised fix list:**

1. **P1 — Define the implementation boundary:** canonical graph/hashing, package discovery, binding schemas and check/generator protocols. Explicitly distinguish milestone support from later extensions.
2. **P1 — Repair verification provenance:** fresh candidate resource handles, explicit oracle digests and schema-valid context-subject identities.
3. **P1 — Correct the worked plan:** show `forgot_link`, require amendment for the page escalation, and separate coding, verification and review accounting.
4. **P2 — Reconcile language rules:** contribution vocabulary, guidance-only changes, annotation placement, movable case identities, root-rule addresses and order-specific binding/context rules.
5. **P2 — Align the checker with the EBNF, then make the three editorial cuts.**
6. **P2 — Close the frontend question:** own syntax for v0; no TypeScript frontend in the milestone.