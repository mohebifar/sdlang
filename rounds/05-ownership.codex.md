**Keep exclusive whole-file ownership for v0, but add explicit allocation, transfer, and integration rules.** Named slots solve multi-file bookkeeping; they do not yet solve shared responsibilities. “Owner plus contributions” works when the owner controls how contributions combine. It cannot promise deterministic, agent-free updates for arbitrary shared code.

The key distinction is:

| Concept | Meaning |
|---|---|
| Ownership | Authority to modify, move, or delete an artifact |
| Dependency | Reliance on an artifact’s contract or behaviour |
| Contribution | An input another declaration must incorporate |
| Evidence | A claim checked against particular code and requirements |

Many declarations can depend on, contribute to, or be tested through one file. Exactly one declaration owns its bytes.

**A. Attacks on the five positions**

1. **Output slots: keep the mechanism; change allocation and identity.**

   A page can own its page, tests, local components, and styles through slots and creation roots. That addresses [REVIEW.md flaw 10](/Users/mohebifar/dev/intentlang/REVIEW.md:18), provided the following cases are defined.

   - **A created component becomes shared.** `login` creates `_parts/AuthForm.tsx`; later `signup` needs it. Being under `login`’s root gives `signup` no write authority or import permission. Either `login` deliberately exposes it as a public entry point, accepting the lifecycle coupling, or an explicit transaction transfers it to a component declaration and updates both pages’ edges and imports. Do not silently reassign it because another page imports it.
   - **A route rename moves a directory.** Moving the page slot alone leaves created files behind. Each created output needs a locator rule: relative to a named creation root, or independently pinned. The plan must enumerate every resulting move, destination collision, and import repair.
   - **Path-derived identities undermine moves.** `#outputs.parts.<relative path>` makes the locator part of the identity, contrary to [Identity](/Users/mohebifar/dev/intentlang/SPEC.md:67). Give the created file a stable member key, such as `#outputs.login_form`; record its root and relative path separately. Renaming its file then preserves identity.
   - **An existing file occupies a proposed path.** Permission to create under a root must never mean permission to overwrite an unmanaged file, adopt it, or replace another owner’s file. These require separate planned operations.

   “Slot files exist from `create` on” should mean **required slots exist when the create transaction publishes**. Nothing needs to appear in the baseline before checks pass. Optional slots need declared presence conditions; empty placeholder files are not a useful ownership contract.

   Creation is provisional inside the candidate. The CLI validates allocations against the actual diff and publishes ownership with the files. It must reject duplicate claims, path escapes, and filesystem aliases that resolve to the same file. A creation root itself owns no directory contents.

2. **Granularity: whole files are a defensible limit; conventions cannot make sharing disappear.**

   “One action per file” helps new code. It does little for existing barrel files, global stylesheets, framework entry points, or test suites exercising several declarations.

   **Two declarations adding the same import** expose the weakness in the proposed symbol model. They might request:

   ```ts
   import { session } from "./session";
   ```

   Deduplication is easy. Resolving different local aliases, type-only versus runtime imports, module directives, or an import with side effects is a semantic integration problem. Serial execution avoids simultaneous writes but does not decide which declaration may change the frame.

   For v0, the file owner integrates the requests in one work order. Contributors do not receive direct permission to edit the shared file. Sharing ownership boundaries within a transaction should not make every task free to edit every transaction output.

3. **Contributions: good direction, but the aggregation contract is missing.**

   The named owner and stable contribution member are substantial improvements. The unresolved cases are:

   - **Owner deletion.** `main_nav` cannot disappear while retained contributions target it. The same transaction must remove those contributions, retarget them, or transfer the integration responsibility. Contributors themselves need not be deleted. Detaching the registry does not resolve dangling contribution references.
   - **Ordering.** Stable member identity is not display order. Declare an order policy: explicit ranks, ordering constraints, or a documented default. Define ties and reject ordering cycles. Sorting by declaration address can make generation reproducible without producing the intended navigation.
   - **Conflicts.** Two pages claiming the same exclusive nav slot must fail aggregate validation. The contribution’s identity and the nav slot it claims are different keys. A duplicate label need not be a conflict; a duplicate exclusive slot is.
   - **Referenced values.** `to: self` must remain a typed reference whose route projection the owner consumes. Otherwise a route change leaves an unchanged contribution payload pointing at an obsolete URL.
   - **Import authority.** A contribution containing a callable reference must grant the owner access to that callable’s public entry point. It must not grant unrestricted imports from the contributor’s files.

   There is also a propagation reversal: the written reference runs **contributor → owner**, but the owner consumes the contributor’s payload. Define an explicit derived aggregate input to the owner. A normal edge that merely consumes the target’s projection does not express this.

   Deterministic registry generation is worthwhile, with conditions: aggregate validation runs first, generation uses the **applied graph plus the current transaction**, and checks still run afterward. Generating from the entire desired graph could publish entries for pages whose transactions later fail.

   Under the existing transaction rule, changed contributors sharing an owner merge through that owner. Accept that coupling explicitly for v0; a generator does not remove it.

4. **Special files: these require managed owners, not permanent exceptions.**

   The manifest proposal contradicts the current model: [`project` is a context declaration that owns nothing](/Users/mohebifar/dev/intentlang/SPEC.md:53). Introduce a managed tooling owner, or explicitly change that classification.

   `use` also needs enough vocabulary to describe build tools, test dependencies, workspace dependencies, and resolution settings—not only runtime capability providers. Otherwise routine implementation repeatedly reaches `blocked` without an expressible fix.

   Returning `blocked` is appropriate when the agent discovers missing authority. It should identify a concrete amendment: dependency requirement, affected manifest and lockfile, owner, and required operation. It must not grant permission to run an unrestricted package-manager command.

   Two further failures:

   - **Unowned configuration is an adoption barrier.** A feature requiring a compiler or framework configuration change becomes impossible through the language. Give configuration explicit tooling ownership, using generators where reliable and scoped edits elsewhere.
   - **Migration ownership cannot follow ordinary deletion.** Deleting an entity must not delete its published migration history. A migration can also implement changes involving several entities. Preserve published migrations through a schema/history owner or an explicit retained-artifact policy.

   Freezing migrations at transaction publication is conservative and workable, but it means published mistakes require subsequent corrective migrations. Publication must not be described as proof that a database ran them.

5. **Formatting noise: separate drift classification from exact input binding.**

   Formatter-normalised hashes should not be the only hashes.

   A formatter upgrade or configuration change can otherwise reinterpret old baselines. Unsupported or unparsable files cannot be normalised reliably. Formatting commands can also touch files outside the transaction envelope.

   Store an exact byte digest for plan binding, publication, and evidence inputs. A pinned formatter may provide a second comparison that classifies an edit as formatting-only. That classification can avoid an implementation task; it must not silently preserve a test result recorded against different bytes.

   Comparing baseline and current code requires the same formatter version and configuration, with baseline bytes recoverable from history. Never run formatting merely to inspect drift in the working tree.

**B. Where owner plus contributions is enough**

It is enough as an **authority model**. Deterministic materialisation additionally needs a domain-specific composition contract.

| Shared surface | Required composition rules |
|---|---|
| Middleware | Execution order, matching scope, short-circuit behaviour, incompatible combinations |
| Route registry | Normalised route keys, collision detection, precedence, callable references |
| i18n catalogs | Namespace and locale ownership, duplicate-key policy, fallback, interpolation contracts |
| Database schema indexes | Cross-entity references, uniqueness, dependency ordering, migration coordination |
| Dependency injection | Binding keys, multiplicity, scopes, overrides, startup ordering and cycles |

For example, accepting a list of middleware names is insufficient if their order determines whether a request reaches authentication or terminates first.

A global CSS file can still have one owner, even when no natural structured contribution exists. That owner receives an integration task covering the relevant requirements. Feature changes that need stylesheet edits must explicitly include that owner in the plan.

Do not disguise arbitrary code patches as typed contributions merely to retain the “no agent” claim. Where practical, extract independently owned stylesheets and let the integration owner control their inclusion and order. Where extraction would distort the code, retain one owner and acknowledge the wider transaction.

**C. Symbol ownership: later, with a substantially narrower promise**

The host-frame model is workable only if an adapter owns the **editing protocol**, not merely symbol hashing.

| Construct | What breaks |
|---|---|
| Re-exports | The public entry point lives outside the defining symbol; renames and export changes cross ownership boundaries |
| Default exports | Anonymous exports lack a stable source name; export form and underlying declaration can change independently |
| Decorators | Symbol changes may require imports, metadata, configuration, and evaluation-order changes |
| Generated code within a file | A generator may rewrite the frame or neighbouring symbols, violating another owner’s boundary |
| Shared module state | Several symbols can depend on one initialiser whose ownership cannot be inferred from text containment |

Normalised AST hashes also need to preserve significant directives, comments, and module context. They are not a general proof of semantic equivalence.

Blaming frame drift on the host is especially misleading when another declaration was authorised to add the import. Record responsibility for the accepted frame operation; otherwise drift reports accuse the wrong owner.

**Ship whole-file ownership in v0.** Consider symbol ownership later for a specific language adapter with stable export identities, explicit shared declarations, structured import requests, and one adapter-mediated writer per file. Unsupported constructs should force whole-file ownership. A universal “top-level symbol plus host frame” mechanism is too weak.

**D. Drift and brownfield adoption**

Ownership must be established before drift can be attributed. Baseline acceptance does not prove conformance.

For an existing file serving three proposed declarations, `spec import` should report the overlap and offer concrete mappings:

- **One whole-file integration owner.** Other declarations depend on its public contracts. Multiple requirements can map tests to the same file without owning it.
- **External references.** Use `extern` for the existing symbols while leaving the file unmanaged.
- **A planned extraction.** Split the file into separately owned outputs, update imports, and publish the ownership assignments with the checked refactor.

It must never create three whole-file claims, or assign ownership according to which declaration was imported first. A core `module` declaration cannot be the integration owner under the current spec because it is context-only; this needs a managed kind.

Import should produce a reviewable mapping of paths, proposed owners, public entry points, unresolved dependencies, and collisions. Accepted outputs start with exact baselines and **unverified** evidence. It should claim neither neighbouring files nor an entire directory merely because discovery scanned them.

After adoption:

- A manual edit to a shared file is drift of its owner. Consumers may require verification; they do not acquire ownership.
- Adopting that edit renews the baseline and stales affected evidence, as the spec already states.
- A generated registry edited by hand should ordinarily be restored or have its source inputs changed. Accepting arbitrary bytes as valid generated output breaks reproducibility.
- Moving a file between owners is an explicit transfer, with collision checks, history, import changes, and evidence reassessment. It is not an `adopt` shortcut.
- Unmanaged dependencies and configuration that affect the work must still participate in plan binding and verification inputs.

Also reconcile the current lifecycle wording: [Plan entries](/Users/mohebifar/dev/intentlang/SPEC.md:683) says drift blocks a declaration, while the drift section exempts frozen declarations. State one authoritative rule.

**E. Proposed state representation**

The following shows ownership fragments from two declaration records. Digest strings are placeholders. The contribution is recorded with its contributor; the registry has exactly one file owner.

```json
{
  "ownership_version": 1,
  "records": [
    {
      "address": "auth.login",
      "ownership": {
        "outputs": {
          "page": {
            "origin": {
              "kind": "slot",
              "name": "page"
            },
            "path": "src/app/login/page.tsx",
            "writer": "agent",
            "baseline": {
              "bytes_sha256": "<page-digest>"
            }
          },
          "login_form": {
            "origin": {
              "kind": "created",
              "root": "parts"
            },
            "locator": {
              "root": "parts",
              "relative_path": "LoginForm.tsx"
            },
            "path": "src/app/login/_parts/LoginForm.tsx",
            "writer": "agent",
            "baseline": {
              "bytes_sha256": "<form-digest>"
            }
          }
        },
        "contributions": {
          "nav": {
            "owner": "app.main_nav",
            "accepts": "items",
            "payload": {
              "slot": "account.sign_in",
              "label": "Sign in",
              "to": {
                "ref": "auth.login"
              },
              "order": 20
            },
            "payload_sha256": "<contribution-digest>",
            "materialized_in": [
              "app.main_nav#outputs.registry"
            ]
          }
        }
      }
    },
    {
      "address": "app.main_nav",
      "ownership": {
        "outputs": {
          "registry": {
            "origin": {
              "kind": "slot",
              "name": "registry"
            },
            "path": "src/generated/nav.ts",
            "writer": "generator",
            "baseline": {
              "bytes_sha256": "<registry-digest>"
            },
            "materialization": {
              "generator": "nextjs.nav.registry",
              "generator_sha256": "<generator-digest>",
              "inputs_sha256": "<resolved-aggregate-digest>",
              "contributions": [
                "auth.login#contributes.nav"
              ]
            }
          }
        }
      }
    }
  ]
}
```

Here, `outputs.login_form` determines stable identity; its path does not. The resolved aggregate digest includes contribution payloads, referenced route projections, composition policy, and other materialisation inputs.

Creation grants belong to the compiled kind and bound plan; they are not file claims. A repository-wide ownership index can be derived from these records and checked for collisions, avoiding a second authoritative ownership database.

Generator identity must be an invalidating input for generated artifacts. That requires an exception to the current rule that [generator-definition changes affect future work only](/Users/mohebifar/dev/intentlang/SPEC.md:738).

Prioritised changes:

1. **P0 — Define the ownership invariant and operations.** One file owner; creation grants confer no existing-file authority; allocation, transfer, move, detach, and deletion publish atomically.
2. **P0 — Give created outputs stable keys.** Specify root-relative relocation, destination collisions, and import repair during route changes.
3. **P0 — Complete contribution semantics.** Define reverse aggregate consumption, referenced projections, ordering, conflicts, import grants, owner deletion, and transaction membership.
4. **P0 — Separate exact fingerprints from drift classification.** Bind plans and evidence to exact inputs; treat formatter comparison as supplementary.
5. **P1 — Make brownfield import an explicit ownership-mapping operation.** Surface overlaps and support integration owners, external references, or checked extraction.
6. **P1 — Add managed tooling and retained-history ownership.** Resolve the `project` contradiction, configuration changes, dependency amendments, and migration retention.
7. **P1 — Validate these rules in the milestone.** Include shared-component extraction, route relocation, occupied creation paths, hand-edited registries, and conflicting contributions.
8. **P2 — Defer symbol ownership.** Require a proven adapter-mediated editing protocol before introducing shared-frame permissions.