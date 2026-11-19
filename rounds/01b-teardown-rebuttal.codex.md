**1. Custom syntax — concede.**

There is no concrete technical reason to defer *designing* a surface syntax. My recommendation conflated validating the runner with validating the human artifact. Hand-written graph JSON can test orchestration; it cannot test whether people can author and review specifications effectively.

There is a technical reason to avoid **stabilizing the full grammar** prematurely: unresolved graph semantics can force incompatible syntax changes. Reference resolution, type identity, capability namespaces, and whether kind definitions distinguish schema from execution policy are examples. But that argues for a provisional, narrow grammar, not for removing syntax from the design.

Define declarations, literals, references, prose, and their lowering to graph JSON now. Require source locations for diagnostics and fixtures showing exactly what each example emits. Defer selectors, templates, and package-authoring syntax until their semantics are exercised. I withdraw the blanket cut.

**2. Core `action` — concede.**

The editor’s argument is sound. A transport-independent operation is a useful domain primitive and a particularly good test of propagation across heterogeneous callers. Demoting it would save little while making the milestone less representative.

My remaining concern is its boundary. “Server-side behaviour” should not silently prescribe deployment, invocation protocol, serialization, or framework bindings. Define `action` through its callable contract and declared requirements; adapters map that contract into HTTP handlers, framework actions, or other transports.

Keep it core. Its presence does not require special treatment in the transaction engine: compatibility, ownership, and checks should remain mechanisms available to other kinds.

**3. Derived dependency permissions — refine.**

I concede that a separate `libraries:` field is unnecessary. However, **every reference granting import permission leaks across relation boundaries**.

Concrete case: a public page mentions `admin.billing` in acceptance prose to describe a redirect. Under clause (a), that mention authorizes importing *any code owned by* `admin.billing`, potentially including its server-side billing implementation. The page needed a route value; it received implementation access. A `navigates` edge has the same problem.

Clause (b) also overgrants: `output: authentication.session` might require only a type, yet authorizes runtime imports across the bound package, including unrelated server or administrative entry points. An import allowlist cannot establish operation-level authorization, but it should preserve declared architectural boundaries.

Derive permissions from existing references, with **relation-specific grants**:

- `mentions` and `navigates` grant no implementation imports.
- Callable references grant the target’s declared public entry points.
- Type references grant type imports.
- Explicit `uses` references can grant declared runtime package exports or subpaths.

Do not authorize every owned file: ownership includes private helpers and tests. Each relation should declare import grants separately from consumed projections and ordering. This preserves the editor’s simpler surface without conflating reference visibility with implementation access.

**4. Transaction grouping — refine; hold on publication before required verification.**

The rule is substantially better, but needs four changes.

**Compute propagation to a fixed point over both old and new graphs.** Suppose action B returns shared type T, and page C calls B. Changing T can change B’s effective signature without changing B’s declaration. A single pass from explicitly changed nodes misses C. Deleted references and targets also require the old graph; otherwise the edge needed to find a former consumer has disappeared. Specify resolved contract projections, including cycle handling.

**Separate permission to edit from participation in acceptance checks.** Step 4 is unsafe for required compatibility checks. An ostensibly compatible change can break a caller’s required behavioral test. If “checks on the full tree” includes that test, step 5 already prevents publication and step 4 cannot occur as written. If it means selected checks executed against the full tree, the producer may publish before discovering the failure.

Run required affected-consumer checks against the candidate before publication. A passing consumer needs no implementation edit. A failing consumer expands the candidate transaction’s edit scope, subject to the existing plan-binding rules. Optional later verification may remain separate, with explicitly provisional evidence.

**Ordering cycles do not inherently require atomic edits.** Mutually recursive actions with unchanged signatures can receive independent body fixes. Merging their entire ordering SCC conflates scheduling with compatibility. Merge cycles that require joint changes; stable contracts can satisfy ordinary ordering dependencies.

**Some large groups are correct.** A breaking signature change to a widely called action really may require every caller in one transaction. Serial execution does not reduce that obligation. Coarse whole-signature projections can overgroup callers, so narrower consumption information may help where compatibility is mechanically provable. Otherwise retain the conservative group. To reduce it, explicitly stage an additive contract, migrate callers, then remove the old contract.

Finally, shared output owners must participate too: a route change and its generated registry may need joint publication even when no caller signature changes. Derive those constraints from ownership and output policy alongside typed references.