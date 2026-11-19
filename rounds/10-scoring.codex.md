The design is substantially stronger than v0, but I would not yet hand it to a team with only minor questions remaining. The main gaps concern the graph input contract, evidence currency, and how adapters materialise core kinds.

I read the requested files without opening anything under `rounds/`. These scores assess the design on paper; package availability and implementation progress do not affect them.

| Area | Score |
|---|---:|
| Architecture (graph, identity, state, plan actions) | **7/10** |
| Drift and brownfield | **8/10** |
| Verification | **8/10** |
| Grammar and syntax | **8/10** |
| Agent interop | **8/10** |
| Package ecosystem | **7/10** |

All six areas can reach 9 on paper for the stated milestone. That would require precise contracts and worked counterexamples, not proof that agents always implement prose correctly.

1. **Architecture — 7/10**

   The strongest reasons are:

   - Stable declaration, member and output identities make renames and relocations tractable.
   - Relation-specific projections, bound plans and atomic transactions form a coherent model for coordinated changes.
   - The canonical graph and implementation-impact rules still leave consequential choices to implementers.

   **To reach 9:** Specify the graph encoding precisely: discriminate each `$type` variant, define how signature fields relate to addressable members, specify ordering for set-valued arrays, and state whether source locations participate in semantic hashes. Add complete graph fixtures with expected projection hashes and plans. The [graph schema](/Users/mohebifar/dev/intentlang/schemas/graph.schema.json:109) currently permits forms such as `{"$type":"named"}` without a name.

   Also generalise [implementation-impact propagation](/Users/mohebifar/dev/intentlang/SPEC.md:890) beyond updates with *no* projection change. An action changing only `uses`, or a generator changing its implementation, can affect callers while leaving its public and behaviour projections unchanged. State explicitly when every published code change—including adoption—causes dependent verification.

2. **Drift and brownfield — 8/10**

   The strongest reasons are:

   - One owner per file gives write authority an enforceable meaning.
   - Named slots and registered created outputs handle multi-file features without treating directories as ownership grants.
   - Integration owners, `extern` and extraction are credible brownfield choices, but their initial ownership transitions remain underspecified.

   **To reach 9:** Add a complete import-and-extraction example showing the starting unmanaged file, reviewed ownership acquisition, exact envelopes, extraction edits, resulting state and failure behaviour. Currently, [planned extraction](/Users/mohebifar/dev/intentlang/SPEC.md:1201) must be reconciled with the rule that agents cannot write unmanaged files.

   Define “affected evidence” for adoption: which owner and dependent evidence becomes stale, which checks must rerun, and how adoption interacts with frozen declarations. These should be explicit planning rules rather than an interpretation of “the edit is fine.”

3. **Verification — 8/10**

   The strongest reasons are:

   - Evidence kinds have appropriately limited promises, and policies combine obligations rather than pretending they form a trust ladder.
   - Requirement-revision coverage, oracle closures, complete discovery and retained attempts address several common sources of false confidence.
   - Currency and reuse across successive transactions lack a sufficiently exact definition.

   **To reach 9:** Add an invalidation table covering implementation bytes, requirement text, inherited normative context, guidance, oracle closure, check definition and execution profile. For each, identify whether it invalidates coverage, conformance review, test runs or approval.

   Define the input set behind every evidence digest and the rule for reusing evidence after another transaction publishes. [“Digest of the checked inputs”](/Users/mohebifar/dev/intentlang/SPEC.md:1025) leaves a central choice unresolved: whole-tree binding makes earlier evidence stale after unrelated edits; narrower binding requires a specified dependency closure. Include a two-transaction example demonstrating the intended outcome.

4. **Grammar and syntax — 8/10**

   The strongest reasons are:

   - The EBNF, lexical scanning rules and explicit continuation points are sufficient foundations for a parser.
   - Binding, visibility, annotations and selector restrictions now explain most previously implicit syntax.
   - A few normative contradictions and missing typing rules remain.

   **To reach 9:** Resolve the contradiction between [SCC hashing of mutually recursive types](/Users/mohebifar/dev/intentlang/SPEC.md:179) and [the prohibition on type cycles](/Users/mohebifar/dev/intentlang/SPEC.md:374). If structural recursion is allowed but alias cycles are forbidden, define that distinction with positive and negative examples.

   Add a selector operator/type matrix, including heterogeneous selected kinds, missing fields, `null` and invalid comparisons. Align the checker with the grammar: `select []` is accepted by the checker, while [the EBNF requires a nonempty kind list](/Users/mohebifar/dev/intentlang/GRAMMAR.ebnf:80).

5. **Agent interop — 8/10**

   The strongest reasons are:

   - Dispatch binding, frozen candidates, independently derived diffs and protected check definitions correctly treat results as claims.
   - Retry continuation, blocker classification and mandatory versus optional context are unusually concrete.
   - Review context and multi-subject repair results are less complete than coding dispatches.

   **To reach 9:** Give review orders a defined context contract containing inherited normative text, resolved references and relevant behaviour diffs. The [review-order context](/Users/mohebifar/dev/intentlang/schemas/review-order.schema.json:213) currently contains only resource-handle strings; the example supplies code/test handles but no explicit reference-resolution or inherited-context payload.

   Make regression results attributable to individual repair subjects. Work orders can contain several repair subjects, but [the result has one top-level regression object](/Users/mohebifar/dev/intentlang/schemas/result.schema.json:49). Also define how `not_automatable` enters review: the current review regression shape requires pre-fix and candidate run receipts even when automated reproduction is unavailable.

6. **Package ecosystem — 7/10**

   The strongest reasons are:

   - Static imports, executable-closure pinning and separated component hashes make packages inspectable and upgrades explainable.
   - One narrow adapter, local packages and bare capabilities provide a sensible bootstrap without requiring an ecosystem first.
   - The attachment mechanism between packages and managed **core** kinds is incomplete.

   **To reach 9:** Specify how an adapter supplies output slots, creation roots, exports and checks for core `action`, and how packages contribute toolchain outputs to `stack`. Package-kind definitions cover package kinds; `binding` covers capabilities; neither clearly supplies this contract for core actions. Keeping action semantics transport-independent does not remove the need to select implementation conventions.

   Document the milestone’s hard-coded adapter profile, including those mappings and conflict rules. For the wider package draft, provide a static manifest schema and distinguish fields implementers may rely on now from deferred extensions. This requires no published packages.

**Disposition of REVIEW.md’s ten flaws**

| # | Status | Assessment |
|---|---|---|
| 1 | **Fixed** | Named expectations and explicit member moves replace positional identity. |
| 2 | **Fixed** | `repair` supplies an unchanged-spec bug-fix flow with regression requirements. |
| 3 | **Fixed** | Behaviour projections and observing relations propagate action prose changes to callers. |
| 4 | **Partly fixed** | Previously undefined constructs now have rules, but recursive-type legality and some selector semantics remain unresolved. |
| 5 | **Fixed** | Kind field schemas are separated from output, check and generator metadata. |
| 6 | **Fixed on paper** | Observations, resource identity, migration ledgers and rollout handoffs distinguish source state from runtime state. |
| 7 | **Fixed** | One binding table, collision errors and aliases resolve namespace ambiguity. |
| 8 | **Fixed in its original scope** | Deterministic verification, independent three-valued review and visible fanout/cost replace fuzzy `verify`; evidence-currency gaps remain separately. |
| 9 | **Fixed** | Deletion enumerates owned outputs, updates consumers and shared owners, and states its unmanaged-code limits. |
| 10 | **Fixed** | Multiple slots, created-output identities and single-owner contributions cover multi-file features. |

**Defects to resolve before implementing the milestone’s core behaviour**

- **The graph-first input contract is incomplete.** Supply a complete, semantically valid graph fixture and canonicalisation rules. The current example references `dashboard.home` without including it, and the checker explicitly exempts missing `mentions` targets, despite the language declaring dangling references erroneous. Define whether partial graphs are supported; if so, define their external interface boundary.
- **Core actions lack a materialisation contract.** The milestone includes action creation and caller updates, so its adapter needs explicit output and export conventions before work orders and import grants can be generated faithfully.
- **Publication needs an evidence-currency rule.** Specify which prior evidence survives each transaction and which dependent checks publication requires. Otherwise two implementations can publish different trust claims for the same sequence of edits.

These need not prevent scaffolding work, but they should not become undocumented implementation decisions.

The checker passed its grammar cases and Markdown JSON parsing. Full schema validation was unavailable because `jsonschema` is not installed; independently recomputed work-order and review-order binding hashes matched.