The draft fixes positional identity, but still conflates **an object’s name, its continuity across edits, and evidence about its current meaning**. Named keys solve the first problem. Explicit moves solve the second. Neither establishes the third.

The current files already expose that split: expectations are named, but [Evidence](/Users/mohebifar/dev/intentlang/SPEC.md:114) uses `auth.login#redirects`, while the [state record](/Users/mohebifar/dev/intentlang/SPEC.md:527) stores only `"redirects"`. Define one canonical identity model before extending either representation.

**A. Attacks on the six positions**

**1. Sub-addresses: accept, but make them schema-defined addresses.**

`decl#section.key` is a useful shape. Widening the grammar alone leaves its meaning undefined:

- Does `#fields.name` identify a persisted field, an arbitrary record key, or either?
- Can a package define an unrelated section called `expect`?
- Can nested fields receive addresses?
- Can an ordered list item be addressed by its index, recreating the positional-ID problem?

Require each kind’s schema to declare its addressable member collections. Reserve core collections such as `expect` and `outputs`. Reject duplicate keys and scalar/section collisions. Addressable collections require explicit keys; indexes and text hashes cannot supply identity.

For example:

```text
auth.login#expect.redirects
core.Workspace#fields.name
core.create_workspace#input.name
core.create_workspace#errors.name_taken
auth.login#outputs.page
```

A member’s key is unique within its collection, not across the declaration. `#fields.name` and `#expect.name` are distinct.

A grammar sketch:

```ebnf
Moved        ::= 'moved' Address '->' Address
              | 'moved' 'module' ModulePath '->' ModulePath

Address      ::= DeclAddress
              | PackageAddress
              | CapabilityAddress

DeclAddress  ::= DeclPath ( '#' MemberPath )?
DeclPath     ::= Word ( '.' Word )*
ModulePath   ::= Word ( '.' Word )*
MemberPath   ::= Word '.' Word ( '.' Word )*

PackageAddress
             ::= 'package' PlainString '::' DeclAddress
CapabilityAddress
             ::= 'capability' Word
```

`PlainString` is a literal without mentions or interpolation. Add `capability` to the keyword set and `::` to the tokens.

The grammar admits a member path; the schema validates it. `#expect.redirects.foo` is invalid unless the expectation schema defines an addressable child there.

Do not silently reinterpret legacy `#redirects`. Diagnose it and offer `#expect.redirects`.

**2. Named expectations: accept identity continuity; reject automatic evidence continuity.**

Editing the text should preserve the test **binding**. It must invalidate the claim that the bound test covers the requirement.

Otherwise:

1. The old requirement says “redirect to home.”
2. The new requirement says “redirect to the original requested URL.”
3. The unchanged test still checks home and passes.
4. The system reports the new requirement as tested.

Re-running the old test cannot establish that it tests the revised requirement.

Record two separate things:

- **Coverage assessment:** this test definition covers this requirement revision.
- **Execution result:** this test passed against these implementation inputs.

After a requirement edit, a human or designated reviewer must reassess coverage against the new text. The implementation agent may propose test changes; the CLI runs the resulting test. `tested` requires both current coverage assessment and a passing run.

Hash the requirement’s semantic inputs: text, resolved mentions, relevant target context and verification policy. A prose edit that canonicalization considers immaterial need not invalidate evidence; the canonicalization rules must say exactly which edits qualify.

“Adding or removing an item changes only that item” should mean **other items retain their identities**. It cannot promise that code, shared tests or other evidence remain unaffected. Removing one expectation must not delete a test also bound to another expectation.

**3. `moved` at every level: accept continuity, qualify its effects.**

A move asserts that two addresses denote one continuing object. It does not establish that every physical representation can be renamed.

```spec
moved core.Workspace#fields.name -> core.Workspace#fields.title
```

This must produce a semantic `rename_field` change. The database package then lowers it using the recorded physical mapping:

- If physical column names follow field names, generate a column rename.
- If both fields map to an explicitly fixed column name, no column rename is needed.
- If the type also changes, plan both operations.
- If the generator cannot preserve the data, fail with an unsupported migration; never fall back silently to drop-and-add.

Generated migration code and a migration applied to a database are also separate facts. The current state model records the former.

Use explicit module syntax:

```spec
moved module auth -> account
```

The original `moved auth -> account` is ambiguous between a declaration and a module. Module matching must use path segments: `auth` covers `auth.login`, not `authentication.login`.

Define moves as a validated mapping before doing the ordinary diff:

| Case | Required behavior |
|---|---|
| Rename without `moved` | Delete plus create. A suggestion may be offered; no automatic continuity. |
| `a -> b`, later `b -> c` | Resolve to terminal `c`; baselines containing either `a` or `b` migrate there. |
| `a -> b` and `b -> a` | Compile error, even when neither source currently has state. |
| Two live baseline objects reach one target | Error. `moved` cannot merge identities. |
| Source has state; target declaration exists only in desired spec | Normal rename case. |
| Source and target both have distinct state | Collision error; do not overwrite target state. |
| Source absent | No migration occurs. Report that local fact without claiming global obsolescence. |
| Source and target member families differ | Error: an expectation cannot become a persisted field through `moved`. |

For overlapping module, declaration and member moves, specify deterministic composition. My recommendation is:

- Use the most specific matching source at each rewrite step.
- Continue rewriting to a terminal address.
- Reject conflicting mappings, cycles and prefix rewrites that cannot terminate.
- Require the resulting mapping to be injective over baseline objects.

That permits a retained declaration rename followed by a later module rename without making statement order significant.

Normalize old references through the same move mapping before comparing logical contracts. Separately account for changed import paths, filenames and other generated interfaces.

**Moving an item between declarations needs explicit semantics.**

```spec
moved auth.login#expect.redirects -> auth.sign_in#expect.redirects
```

This transfers the requirement’s history and candidate test binding. Because its target changed, coverage must be reassessed. Update both owners in one transaction.

For fields, moving between entities is more than a column rename. It may require joins, key mappings or backfills. Preserve the logical correspondence, but require a supported data-transfer operation.

A move is one-to-one. Splitting one expectation into two, or consolidating several fields, needs an explicit transformation rather than multiple `moved` statements.

**Branch merges require three-way reconciliation.**

Suppose the merge base contains `auth.login`; one branch renames it and the other edits its expectations. The reconciler should:

1. Recover continuity from the rename branch.
2. Apply the other branch’s edits to the renamed declaration.
3. Reconcile outputs and state using that correspondence.
4. Invalidate evidence against the merged inputs.

If branches rename the same object to different destinations, report an identity conflict. If the destination was independently created, report a collision.

[“State files conflict where the code does”](/Users/mohebifar/dev/intentlang/SPEC.md:952) is insufficient: Git can merge spec, code and state text cleanly while leaving duplicate ownership or incompatible histories.

**4. Lifetime of `moved`: retain history, remove the “safe to remove” claim.**

The local planner cannot know about every branch, checkout or archived state snapshot.

A valid message is:

> Source absent from this baseline; this move performed no work here.

It cannot infer:

> Safe to remove.

Retained move sources must remain reserved. Otherwise a historical move can steal a newly created object:

```spec
moved auth.login -> auth.sign_in
// Later someone creates an unrelated auth.login.
```

Reject that reuse while the move remains active.

For v0, keep move history indefinitely. If pruning becomes necessary, make it an explicit compatibility boundary: advance an identity epoch, record that epoch in state, and refuse to plan older state until its migration history has been replayed. Simply deleting old move statements makes old state look like ordinary delete-and-create.

**5. Rule requirements: use a composite identity, not a fabricated target suffix.**

`dashboard.home#rule.app_requires_session` loses:

- The rule’s module.
- The identity of an individual requirement within the rule.
- The distinction between a requirement definition and its application to a target.

Two rules named `app_requires_session` in different modules must not collide.

Name requirements inside rules:

```spec
rule app_requires_session {
  applies_to: select nextjs.page where route matches "/app/**"

  expect {
    redirects {
      text: "Unauthenticated visitors are redirected to @{auth.login}"
      verify: tested
    }
  }
}
```

The requirement definition is:

```text
policies.app_requires_session#expect.redirects
```

An evidence subject is the pair:

```json
{
  "target": "dashboard.home",
  "requirement": "policies.app_requires_session#expect.redirects"
}
```

A rule rename re-keys the requirement component; a target rename re-keys the target component.

Changing the requirement invalidates its applications. Changing the selector adds or removes applications. If a target stops matching, its old evidence becomes historical; it must not satisfy a requirement if the target later rejoins without checking the inputs again.

Rules may remain output-free context declarations, but they still need identity and canonical snapshots. **No owned files** must not mean **no recorded identity**.

**6. Templates: argument stability is insufficient, and blanket body changes are wrong.**

A fixed expansion namespace stabilizes names, but not meaning:

```spec
expand crud workspace_pages(entity: core.Workspace)
```

Changing the argument to `billing.Invoice` retains every address while potentially replacing the feature’s entire purpose. That requires an explicit rebinding policy, especially for persistence-bearing members.

Other unresolved identities include:

- Renaming a template-local member across every expansion.
- Renaming an expansion instance.
- Moving between a real module and an expansion namespace.
- Changing a template’s package or parameter defaults.
- Removing a template member that has local overrides.
- Detaching a member while its template continues defining it.

“Editing the template changes every member’s body” also discards the projection model. A change to one local declaration should affect that declaration and its dependents. Template provenance and context changes should be recorded separately.

Finally, expansion namespaces need resolver rules: the current resolver can traverse modules and packages, while declarations are not records. The grammar admits templates, but the [statement table](/Users/mohebifar/dev/intentlang/SPEC.md:168) does not explain their namespace behavior.

**B. Other identities the draft needs**

Not everything needs an opaque ID. Anything independently mapped, migrated, verified or owned needs a stable key.

| Subject | Recommended identity and consequence |
|---|---|
| Action inputs, output members and errors | Schema-defined member addresses. Error identity should survive changes to message text or wire code. Input renames still require caller updates. |
| Entity constraints and indexes | Named keys, not identities derived from their participating fields. Changing a constraint’s fields then becomes an update. |
| Enum variants and union cases | Named cases when code or migrations track them independently. |
| Package catalog symbols | Canonical package identity plus export path, independent of import alias and version. If multiple package instances are supported, include a stable instance key. |
| Capability bindings | A distinct identity such as `capability authentication`. Provider and options are mutable, hashed inputs. |
| Generated outputs | Owner plus named output slot; pathname is a mutable locator. |
| Tests and checks | Stable check identity and test-case locator, plus definition hashes. Test title or line number alone is insufficient. |
| Shared-file contributions | Named contribution keys; otherwise inserting an anonymous contribution recreates positional identity. |
| `extern` | Stable local declaration address; source file and symbol are mutable locators. |
| Context declarations | Canonical identities and snapshots even when they own no outputs. |

For action errors, prefer:

```spec
errors {
  name_taken {
    code: "NameTaken"
    summary: "The workspace name is already in use"
  }
}
```

`#errors.name_taken` is identity; `"NameTaken"` is public contract content.

For packages, resolving both `ui.input` and an alias such as `widgets.input` should produce the same canonical symbol, for example:

```text
package "npm:ui-sdk"::input
```

The installed version belongs in the lock data and consumed hashes. Consumers must not invent moves inside a package’s catalog; package authors supply export migration information.

Capability resolution should retain both the binding and the resolved catalog symbol. Rebinding `authentication` to another provider preserves the binding’s identity, but does not establish provider compatibility.

**Generated file paths are especially important.**

For a declaration whose route changes from `/login` to `/sign-in`:

```text
Output identity: auth.login#outputs.page
Old locator:    src/app/login/page.tsx
New locator:    src/app/sign-in/page.tsx
```

That is one continuing output with a changed path. Plan both paths in the write envelope, check destination ownership, and update references. A path-derived identity would turn an ordinary route edit into file deletion and creation with lost history.

For `extern`, changing `source` must invalidate consumed evidence even when the local address remains stable. Missing or ambiguous source symbols must not silently retain the previous hash. Changing managed code to `extern` also needs an explicit ownership transition; the existing `replace` behavior would be inappropriate if it deleted the code.

The `consumed` map needs more structure too. One target can supply several projections through different relations:

```json
{
  "relation": "types",
  "target": "core.Workspace",
  "projection": "public",
  "hash": "77e2..."
}
```

A single `target -> hash` entry cannot represent those independently.

**C. Concrete template recommendation**

**Remove live templates and expansions from v0. Replace them with scaffolding that emits ordinary spec declarations.**

For example:

```sh
spec scaffold crud --entity core.Workspace --module workspace_pages
```

The command creates ordinary declarations under `spec/workspace_pages/`, such as `index`, `new`, `edit` and supporting actions. This command is a proposed authoring tool, not a new language construct.

After creation:

- Users edit the declarations directly.
- Entity references provide normal dependency propagation.
- Shared runtime components reduce implementation duplication.
- There is no override precedence, detachment operation or hidden template writer.

This meets the concrete need—CRUD pages per entity—without creating a second model of authorship and ownership.

Remove `TemplateDef` and `Expansion` from `Exportable` in v0. Reserve their keywords if useful, but diagnose their use as unsupported. Reconsider live templates when repeated scaffold maintenance demonstrates a need that ordinary references and components cannot satisfy.

**D. Make `expect` a section of named requirement blocks**

The record field works for `key -> text`, and could grow into `key -> metadata record`. But expectations already have compiler-owned meaning, evidence and lifecycle. A section expresses that better and follows the existing `fields { ... }` precedent.

```spec
nextjs.page login {
  route: "/login"

  expect {
    redirects {
      text: "Valid credentials redirect to @{dashboard.home}"
      verify: tested
      test: {
        output: "behaviour_tests"
        case: "valid_credentials"
      }
    }

    generic_error {
      text: "Invalid credentials show one generic error"
      verify: reviewed
    }
  }
}
```

Here `output` identifies a named test output owned by this declaration, and `case` is a package-defined stable runner locator. Handwritten tests can use a separate locator form. Neither locator determines requirement identity.

Make verification policy explicit. `accepted`, `tested` and `judged` are not inherently a total ordering: human acceptance does not imply a passing test. Define which evidence satisfies each policy, and calculate the effective policy from project, rule and item requirements without allowing accidental weakening.

I would not make every expectation a top-level declaration in v0. Local requirements belong naturally to their owner; rules already provide reusable requirement definitions with per-target applications.

A grammar fragment:

```ebnf
ExpectSection ::= 'expect' '{' Gap
                  ( ExpectItem ( Sep ExpectItem )* Sep? )?
                  Gap '}'

ExpectItem    ::= Word Block
```

The expectation block schema defines `text`, `verify` and optional `test`. Reserve `expect` at declaration-body level so kinds cannot reinterpret it.

The existing generic `Member`/`Block` productions already parse this shape. Special grammar productions are useful documentation, but schema validation can enforce it without duplicating parser machinery.

**State and plan consequences**

Keep one state file per managed declaration, with keyed members inside it. Add a graph manifest for output-free identities and retained migration history.

An illustrative state record:

```json
{
  "address": "auth.login",
  "identity_epoch": 1,
  "kind": "nextjs.page",
  "hashes": {
    "public": "public-hash",
    "body": "body-hash",
    "context": "context-hash"
  },
  "members": {
    "expect.redirects": {
      "contract_hash": "requirement-hash"
    }
  },
  "outputs": {
    "page": {
      "path": "src/app/login/page.tsx",
      "hash": "page-hash"
    },
    "behaviour_tests": {
      "path": "tests/auth/login.test.ts",
      "hash": "test-file-hash"
    }
  },
  "evidence": [
    {
      "subject": {
        "target": "auth.login",
        "requirement": "auth.login#expect.redirects"
      },
      "coverage": {
        "contract_hash": "requirement-hash",
        "test_definition_hash": "test-definition-hash",
        "decision": "covers",
        "reviewer": "reviewer-identity"
      },
      "run": {
        "test": {
          "output": "auth.login#outputs.behaviour_tests",
          "case": "valid_credentials"
        },
        "test_definition_hash": "test-definition-hash",
        "check_definition_hash": "check-hash",
        "inputs_digest": "covered-inputs-hash",
        "tree": "git-tree-id",
        "result": "passed"
      },
      "status": "tested"
    }
  ]
}
```

Treat `status` as a derived summary. An old assessment and passing run remain audit records after a text edit, but no longer establish current `tested` status. For v0, binding execution to the full relevant tree is simpler than claiming precise coverage dependencies that the system cannot yet compute.

Plan entries can remain declaration-level work units, but need member-level changes:

```json
{
  "address": "core.Workspace",
  "identity": "same",
  "change": "update",
  "members": [
    {
      "from": "core.Workspace#fields.name",
      "to": "core.Workspace#fields.title",
      "identity": "moved",
      "operation": "rename_field"
    }
  ]
}
```

A cross-owner member move must join both declaration entries into one transaction. Re-key state, evidence subjects and ownership records only when that transaction succeeds.

**Prioritized changes**

1. **Define canonical typed addresses and move semantics:** chains, cycles, collisions, prefix composition, reserved sources and cross-owner moves.
2. **Separate requirement identity, coverage assessment and test execution.** Include the current requirement contract in evidence validity.
3. **Adopt named `expect` blocks with explicit verification policy and optional test locators.**
4. **Give output slots stable identities independent of file paths.**
5. **Record rule applications as `(target, requirement)` pairs and retain identities for context declarations.**
6. **Replace “safe to remove” with local migration diagnostics; define retained history and an explicit retirement boundary.**
7. **Specify three-way identity reconciliation for branch merges.**
8. **Define action-member, catalog, capability, contribution and `extern` identity rules before implementing their state consumers.**
9. **Defer live templates; provide CRUD scaffolding that emits ordinary declarations.**