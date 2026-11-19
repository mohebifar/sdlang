**Scores: architecture 8, drift and brownfield 8, verification 8, grammar 9, agent interop 8, package ecosystem 8.**

These are design-readiness scores. I give no credit or penalty for implementation status. I did not open anything under `rounds/`.

**1. Architecture — 8/10**

Strongest reasons:

- Declaration, member and output identities are stable; paths are treated as locators.
- Relation-specific consumption, observation, ordering and import grants give propagation a useful foundation.
- Transactions, applied-graph semantics, contingent envelopes and journaled publication address partial progress explicitly.

To reach 9:

- Add a normative precedence table distinguishing **authored changes** from **inherited changes**. Inherited prompts enter a declaration’s `behaviour` hash, but they are supposed to trigger review before editing. The reference derivation currently treats a module-prompt-only edit as immediate updates to `auth.login`, `auth.sign_in` and `auth.signup`. Specify the required plan for that case and align the reference example.
- Finish the canonical byte contract: numeric serialization, NFC normalization, source-location exclusion at each hash boundary, and annotation placement by declared projection. Include expected hash vectors. The reference canonicalizer currently gives different hashes to composed and decomposed spellings of the same NFC-normalized string.

**2. Drift and brownfield — 8/10**

Strongest reasons:

- Whole-file ownership is a clear, supportable boundary, with explicit integration-owner, `extern` and extraction choices.
- Creation roots confer no authority over existing files; registered outputs retain identity through relocation.
- Adoption correctly separates baseline acceptance from verification evidence.

To reach 9:

- Define plan fingerprints as **the bytes observed when planning**, separately from the accepted baseline. In [examples/plan.json](/Users/mohebifar/dev/intentlang/examples/plan.json:11), signup’s fingerprint equals its old baseline, while the same plan records different actual bytes. The specified apply-time binding check would reject the plan, contradicting the example’s claim that signup drift blocks only signup.
- State an explicit publication exception for `create/acquire`. Brownfield adoption publishes ownership with `unverified` evidence, whereas the general transaction rule requires every policy to be met. Define which checks acquisition requires, which obligations remain pending, and what subsequent operation discharges them.

**3. Verification — 8/10**

Strongest reasons:

- Conjunctive policies avoid confusing tests, review, checks and approval with interchangeable levels.
- Coverage judgement is separate from test execution, with oracle digests, complete discovery and retained attempts.
- Currency and adoption invalidation are substantially specified, and the milestone tests coverage-review quality using deliberately weak tests.

To reach 9:

- Extend run-input closure beyond owned outputs reachable through graph relations. An unmanaged Next.js root layout or middleware can affect a page’s integration behaviour without appearing in that closure. Require adapters to declare implicit framework inputs, with a conservative whole-tree fallback when the closure is incomplete.
- Define check-input binding by scope: project checks must include all relevant project inputs, including unmanaged files. State how those changes create verification work.
- Define whether each conformance rule is required or inapplicable, and exactly how `unsupported` and `error` affect publication. Reporting those outcomes is specified more clearly than their acceptance consequences.

**4. Grammar and syntax — 9/10**

Strongest reasons:

- The EBNF is sufficient to start a parser: statement forms, precedence, separators, continuation points and lexical rules are explicit.
- Resolution, visibility, collisions, structural recursion and alias-cycle rejection have concrete rules.
- Strings, mentions, interpolation, annotations and selector operand types are substantially defined.

Remaining questions are minor enough for implementation. This score covers the language contract; it does not imply that parsing examples proves semantic validity.

**5. Agent interop — 8/10**

Strongest reasons:

- Orders and replies have explicit bindings, per-subject authority and structured result contracts.
- Retries, blocker classification and recovery distinguish agent claims from runner decisions.
- Frozen-candidate reviews, independent reviewer provenance and mandatory context are strong boundaries.

To reach 9:

- Make the worked protocol examples agree with their graph inputs. For example, the work order’s `auth.sign_in` public hash differs from the corresponding value in `hashes.json`, although that declaration is unchanged between the graphs. Define which canonical object each hash names and derive those fixture values from it.
- Align import grants with the adapter. The work order grants ambient module `next`, while the adapter declares exact subpaths such as `next/link` and `next/navigation`.
- Add a normative cross-document validation table covering exact subjects and assessments, coverage candidates, regression subjects, output ownership claims and binding consistency. The schemas deliberately leave relational checks to the runner; consolidate those requirements so implementers need not reconstruct them from several sections.

**6. Package ecosystem — 8/10**

Strongest reasons:

- Static imports, component hashes, pinned execution closure and declarative upgrade maps constrain package authority.
- Core-kind implementations and the one-implementer rule close an important ownership gap.
- One narrow adapter, local packages and bare capabilities provide a plausible bootstrap without requiring a large registry.

To reach 9:

- Reconcile the milestone’s check set with its required adapter: the milestone says only typecheck and build, while the adapter also requires tests and conformance.
- Extend the generator contract to represent deletion and relocation explicitly. A generator subject’s `after` currently cannot be null, and the envelope lacks deletion/move operations, despite generated declarations and output conventions being changeable.
- Put route-to-path mapping into the package contract. The adapter explains a transformation in comments, while the closed kind vocabulary provides templates but no declared transformation mechanism. Either specify a static mapping primitive or restrict templates to an explicitly defined substitution-and-normalization algorithm.

All six areas can reach 9 on paper for the bounded milestone. That would establish implementation readiness, not demonstrate that agents or reviewers perform reliably.

**The original ten flaws**

1. **Fixed:** named expectations and member moves replace positional identities.
2. **Fixed:** `spec repair` handles unchanged-spec bugs and requires requirement-derived regression evidence.
3. **Fixed:** behaviour projections and observing relations propagate normative prose changes.
4. **Fixed:** the previously undefined syntax has grammar, resolution and semantic rules; removed forms have explicit replacements.
5. **Fixed:** kind field schemas and operational metadata are separate, with a closed vocabulary.
6. **Fixed:** source state, observation receipts and rollout readiness are separate; unreachable environments remain unknown.
7. **Fixed:** one binding table and collision errors prevent silent namespace overlap.
8. **Partly fixed:** review responsibility, outcomes and costs are explicit, but inherited-change planning still conflicts with the reference derivation.
9. **Fixed:** deletion enumerates outputs, identifies former consumers and contribution owners, and bounds its cleanup guarantee.
10. **Fixed:** named slots, registered created outputs and ownership transfers cover multi-file features.

**Defects to resolve before implementing the first milestone’s contract**

Three require resolution rather than an implementer silently choosing an interpretation:

- **Check-set conflict:** decide whether tests and conformance belong to the milestone, then make its description and mandatory adapter agree.
- **Drift-binding contradiction:** bind observed bytes so an unrelated drifted declaration does not make the entire example plan immediately inapplicable.
- **Inherited-change classification conflict:** specify review-first propagation consistently with the graph-derived plan contract.

These do not prevent scaffolding, but I would resolve them before treating the runner’s expected behaviour as settled.

Validation was limited: the checker’s grammar checks passed, but schema validation could not run because `jsonschema` is absent. I left the environment unchanged; the findings above include direct, read-only checks of the supplied artifacts.