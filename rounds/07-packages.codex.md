**Keep packages as a distribution mechanism, but make their authority much smaller.** The draft currently treats a schema, an implementation convention, a compatibility proof, and permission to execute code as one package contract. Those need separate rules.

The strongest existing decisions already help: one file owner, conservative propagation, generator output validation, and immutable retained artifacts. The package design should preserve those rules. It should also respect the [first milestone’s existing scope](/Users/mohebifar/dev/intentlang/SPEC.md:1387): graph JSON, one Next.js adapter, typecheck/build, and scoped updates. That milestone does not require a package ecosystem.

All syntax below is proposed syntax.

**A. Attacks on the six positions**

**1. An npm package is a reasonable container; it is not a sufficient package contract.**

Concrete failures:

- The npm tarball contains JS but omits its `.spec` files or skill folders. Development works through a workspace symlink; the published package fails.
- Package evaluation follows Node’s executable exports or install scripts. Merely importing a kind now executes code, violating the spec’s requirement that planning requires no arbitrary execution.
- Two spec packages require different versions of the same adapter or runtime library. A package name and version alone do not identify the resolved dependency instance.
- An ordinary exported managed declaration in a package has outputs. Does importing it create project state and claim files? Catalog installation must not silently instantiate managed declarations.
- “Conformance analyser” is executable, but none of the five proposed forms identifies its protocol. Likewise, position 5 introduces a sixth form, `capability`; skill exports still lack a defined representation.

Use a static manifest entry, with an explicitly versioned format:

```json
{
  "name": "@intentlang/nextjs",
  "intentlang": {
    "format": 1,
    "entry": "./spec/index.spec",
    "skills": {
      "app_router": "./skills/app-router"
    }
  }
}
```

Importing reads data only. Package imports never create project declarations. For v0, catalog exports should be types, constants, skill resources, and `extern` declarations describing existing runtime exports. Reject package-owned managed instances until instantiation semantics exist.

**2. Sections fix the visual ambiguity, but the semantic ambiguity remains.**

Concrete failures in the example:

- `fields.input: { name: string }` could describe a record **value** accepted by an action declaration, or a record **type** describing values accepted by the generated action. Those are different schema levels.
- `uses` is already a universal declaration field. Can a package replace its schema or its relation?
- The relations table grants public entry points, but the kind never declares which output and exported symbol form an entry point.
- A URL route is not necessarily a filesystem path. `/`, dynamic segments, route groups, and parallel routes need explicit handling.
- `"static" | "server" | "client"` mixes rendering strategy with component boundaries. Next.js’s client boundary affects the imported module graph; client components also participate in the initial rendered HTML. These are not three interchangeable modes. [Next.js documentation](https://nextjs.org/docs/app/getting-started/server-and-client-components)
- A removed output slot could accidentally become permission to delete an existing file.
- `ambient.imports` can grant an entire framework surface without saying which version or whether server/client restrictions still apply.

The fix is a closed metadata schema, a separate schema model for type-valued contracts, and explicit public exports. Do not let package authors invent section semantics.

**3. Variance is useful, but “compatibility by variance” is too broad.**

For callable contracts, the direction is correct:

| Contract change | Static compatibility for existing callers |
| --- | --- |
| Input accepts more values | Compatible |
| Input accepts fewer values | Incompatible |
| Output guarantees a more specific value | Compatible |
| Output permits additional alternatives | Incompatible |

But several qualifications matter.

**First, record compatibility needs an exact definition.** Adding an optional output field is compatible under open-record width subtyping. It may fail a consumer that rejects unknown keys. Making an existing required output field optional is incompatible. “Adding a field” is not a compatibility rule.

**Second, compare projected contracts, not the schemas of metadata fields.** Changing `route: "/login"` to `route: "/sign-in"` changes a literal contract even though both values have type `route_path`.

**Third, entities have several consumers.**

```spec
// Before
fields {
  id: uuid @id
}

// After
fields {
  id: uuid @id
  tenant_id: uuid
}
```

Existing readers might tolerate the additional field. Existing writers cannot necessarily supply it. Existing database rows cannot satisfy a new non-null constraint without an appropriate default or backfill. PostgreSQL initializes an added column from its default, or null when none exists; constraints must hold for the resulting data. [PostgreSQL documentation](https://www.postgresql.org/docs/current/ddl-alter.html)

Thus an entity needs at least:

- Read-contract compatibility.
- Write-contract compatibility.
- Storage-transition safety.

Mutable collections and references can also require invariance. One `@interface(out)` on “entity fields” cannot answer all three questions.

**Fourth, behaviour remains separate.** Changing an action from charging cents to charging dollars preserves `number -> Receipt`. Changing an optional input’s default can preserve its type while changing caller assumptions.

Keep the existing [behaviour propagation rule](/Users/mohebifar/dev/intentlang/SPEC.md:724): observed normative changes cause `verify` and `review`, regardless of structural compatibility.

My v0 recommendation:

- Implement structural compatibility for the **core action signature**, whose input/output/error positions the compiler understands.
- Keep ordinary package `@interface` fields invariant.
- Treat unsupported comparisons as incompatible.
- Defer entity compatibility to the entity/migration design.
- Define “compatible” narrowly: it avoids mandatory caller editing; it still requires verification.

This avoids arbitrary classifier code without pretending every domain has structural compatibility.

**4. Relations, checks, and generators do not have equivalent authority.**

A relation is static graph policy. A check executes code. A generator proposes mutations. Giving all three similarly shaped definitions obscures that difference.

Concrete failures:

- A package defines a relation that grants imports but omits ordering or behaviour observation. It can weaken propagation merely by renaming `calls`.
- Two generators match `select entity`. Which wins? Installation order must not decide.
- An entity generator writes a schema file owned by the entity and migration history owned by a database binding. `writes: ["outputs.schema", "retained.migrations"]` does not identify those owners.
- A project check edits `package.json`, reads credentials, or contacts a service. `scope: "project"` describes neither its permissions nor its inputs.
- A generator reports success without changing the required output. Exit status alone establishes nothing.

For v0, keep relations fixed in core. Add custom relations only when a real adapter needs one.

Bind generators directly to producer kinds instead of selecting them globally:

```spec
export generator registry {
  run: ["node", "./gen/registry.js"]
}

export kind registry_file {
  fields {
    entries: [string]
  }
  outputs {
    registry: "src/generated/registry.ts"
  }
  generator: registry
  checks: [typecheck]
}
```

The generator is the exclusive producer for this kind’s outputs. Its definition does not grant write authority; the compiler-issued work envelope does. Mixed agent/generated ownership within one declaration can wait.

**“Pure function of declared inputs” needs an operational definition.**

Drizzle’s documented generation process reads schema files and previous migration snapshots, can prompt for renames, and writes timestamped migration directories. A declaration diff alone is insufficient. [Drizzle documentation](https://orm.drizzle.team/docs/drizzle-kit-generate)

A reproducible adapter needs an input closure such as:

```text
result = F(
  generator-and-dependency-digests,
  runtime-image,
  old-and-new-canonical-graph,
  baseline-artifacts,
  candidate-input-files,
  explicit-rename-decisions,
  fixed-generation-identifier
)
```

The runner must provide those inputs and block undeclared ambient inputs. The adapter must resolve prompts from explicit decisions or return `blocked`. It must control tool-generated names and timestamps, including references inside metadata.

If a pinned Drizzle version cannot meet that contract, classify the adapter as non-reproducible or leave it unsupported. Shelling out does not itself violate purity; uncontrolled subprocess inputs do.

**5. A capability with names only is a namespace contract.**

Concrete failure: two providers both export `User`, `session`, and `skills`, but one exposes `session` as a callable and another as request middleware. Even validating the exported kinds would not establish compatible calling conventions or behaviour.

Retain the no-substitution promise. Describe the feature as an **export namespace**, and derive each export’s actual contract from its target.

A manifest alias map is sufficient before a dedicated language form is justified:

```json
{
  "provides": {
    "authentication": {
      "User": "catalog.User",
      "session": "catalog.session",
      "skills": "catalog.skills"
    }
  }
}
```

Require every target to exist and be exported. A provider change remains an `update` for affected consumers, as already decided in D1.7.

**6. Bootstrapping contains one strong idea and two weak shortcuts.**

Project-local packages are the strong idea. However, `spec/_packages/` is currently inside a tree where every directory is a project module. Define an explicit exclusion boundary; otherwise package sources may enter project evaluation.

Local packages must use the same manifest, validator, and identity rules as published packages. Their contents must be snapshotted and hashed; a path is not a pin.

The weak shortcuts:

- `unit` can accumulate arbitrary outputs, generic checks, and prose while still appearing as strongly modelled as an action.
- A documentation skill provides guidance, not a callable contract, conformance check, or compatibility guarantee. Updating documentation cannot silently change normative obligations.
- A bare library capability must not mean unlimited subpath imports. Record its resolved library identity and import grant explicitly.
- Without an options schema, reject options on a bare `use`.

Shipping entities, migration support, and a broad Next.js package immediately would contradict the existing milestone boundary.

**B. The exact v0 kind vocabulary**

I would permit the following closed set. Unknown fields and sections are errors. All omitted maps/lists default to empty; `generator` defaults to absent.

Package-defined kinds are managed in v0. Core context constructs and catalog `extern`s retain their fixed semantics.

| Section or field | Type | Semantics |
| --- | --- | --- |
| `fields { … }` | Map of field names to value schemas | The declaration’s package-defined authored fields. Supports the existing type expressions, defaults, and valid annotations. Defaults participate in canonicalization. |
| `outputs { … }` | Map `SlotKey -> template` | Required whole-file output slots. Keys are identities; evaluated paths are locators. Every slot must exist at publication. |
| `creates { … }` | Map `RootKey -> template` | Roots permitting creation of new, individually registered outputs. Grants no authority over existing files. |
| `exports { … }` | Map `EntryKey -> ExportDescriptor` | Public code entry points that dependency edges may grant. |
| `ambient { imports: … }` | `imports: [ModuleSpecifier]` | Exact external module specifiers permitted without declaration edges. Names resolve through the pinned runtime dependency tree. |
| `checks: …` | `[CheckRef]` | Required checks, added to applicable core/project checks. A declaration cannot remove these requirements. |
| `generator: …` | `GeneratorRef?` | Exclusive deterministic producer for the declaration’s outputs; otherwise implementation is agent work. |
| `skills: …` | `[SkillRef]` | Additional guidance resources mounted into work orders. Their content is hashed; edits follow the existing guidance-only propagation rule. |

An `ExportDescriptor` has exactly:

| Field | Type | Meaning |
| --- | --- | --- |
| `output` | `SlotKey` | The fixed output containing the export. |
| `symbol` | Identifier or `"default"` | The actual exported symbol. |
| `mode` | `"value" \| "type"` | Whether runtime or type-only import authority can be granted. |

For example:

```spec
export kind widget {
  fields {
    label: string @interface
    calls: [ref action] @relation("calls")
  }

  outputs {
    component: "src/components/widget.tsx"
  }

  exports {
    component {
      output: "component"
      symbol: "Widget"
      mode: "value"
    }
  }

  ambient {
    imports: ["react", "react/jsx-runtime"]
  }

  checks: [typecheck, component_conformance]
}
```

This deliberately describes a singleton convention. A reusable package would add a safe locator field or a separately specified naming convention.

Additional fixed rules:

- Core declaration fields cannot be replaced by kind definitions. In particular, the example’s redeclaration of universal `uses` should be rejected. A package can introduce a differently named, more specific reference field.
- Reference-bearing value fields require an explicit relation. Prose mentions and core type positions retain their existing automatic relations.
- `@interface` adds a canonical field value to `public`; changed values are conservatively incompatible.
- Package value schemas cannot masquerade as type-valued or addressable member collections. Core `action.input`, `action.output`, `action.errors`, and future entity fields need explicit compiler schema primitives.
- Templates may interpolate only supported scalar fields. No functions, environment reads, or implicit URL-to-filesystem conversion.
- Paths are repository-relative, normalized, and checked for escapes, aliases, collisions, and existing ownership.
- A v0 managed package kind requires at least one fixed output slot.
- Export declarations grant nothing until the declared symbol is validated in the candidate code.
- Ambient imports are an import-policy allowance, not an execution sandbox.

A `check` needs only:

```spec
export check typecheck {
  purpose: "build"
  scope: "project"
  run: ["pnpm", "exec", "tsc", "--noEmit"]
}
```

Its closed fields are:

- `purpose: "build" | "conformance"` — selects the result protocol; success does not automatically satisfy every prose requirement.
- `scope: "declaration" | "project"` — selects the subject/input envelope.
- `run: [string]` — nonempty argv, executed without a shell, using pinned executable resolution.

Package-relative executable paths resolve against the package root; project commands run against the candidate project. This distinction must be specified by the runner protocol.

A `generator` needs only `run`; its inputs, result format, and ownership enforcement come from a versioned core protocol. Resource limits belong to runner policy.

**Cut from the v0 package API:**

| Proposed surface | Disposition |
| --- | --- |
| `sort` | Fixed by declaration category; package kinds are managed. |
| Custom `relation` definitions | Keep core relations until an adapter demonstrates a missing relation. |
| Custom compatibility classifiers | No executable planning hooks. |
| General `@interface(in/out)` | Reserve for a later contract-schema model; core actions already have known positions. |
| `accepts`, `collects`, aggregation policy | Defer with contributions, as the milestone already does. |
| Retained-artifact creation metadata | Defer with migrations; preserve existing retained artifacts regardless. |
| Generator `handles`, `changes`, `writes` | Replace global selection and self-granted writes with direct producer attachment and compiler-issued envelopes. |
| Kind inheritance, arbitrary hooks, custom sections | Exclude. |

When contributions arrive, `accepts` alone will be insufficient: payload schema, exclusive keys, ordering, selector membership, referenced projections, and aggregate-owner publication must be specified together. D5.6 already requires that contract; package syntax should not suggest the schema alone implements it.

**C. Replace `unit` with cheap local kinds**

I would omit `unit` from the core prelude.

Its problem is not arbitrary file ownership—that already exists through local kinds. Its problem is making an opaque integration look like a recognised domain contract.

Make the replacement inexpensive:

```spec
// spec/_packages/acme/spec/index.spec
export kind cli {
  fields {
    command: string @interface
  }
  outputs {
    main: "src/cli.ts"
  }
  checks: [typecheck]
}
```

```spec
import package "./_packages/acme" as acme

acme.cli tool {
  command: "acme"
  prompt: "Print usage when invoked without arguments."
}
```

Provide `spec package init` to scaffold this.

A local kind can still remain shallow forever. Naming it does not create conformance. Reports must therefore show actual coverage separately: schema validated, ownership enforced, build passed, domain conformance unavailable, behaviour judged/tested.

For untouched existing code, use `extern`. For managed integration files, use an explicit local integration kind. Neither requires pretending the integration has stronger guarantees.

**D. Trust, locking, and package upgrades**

**Pin the executable closure, not just the top-level package.**

`spec.lock` should record:

| Pinned item | Why |
| --- | --- |
| Canonical package identity and exact resolution | Import aliases and version ranges are insufficient identities. |
| Artifact integrity | Detect changed bytes. |
| Resolved transitive dependencies and peer bindings | The same entry script can execute different dependency code. |
| Package format and executor protocol | Prevent interpretation under an incompatible ABI. |
| Separate schema, skill, check, generator, convention digests | Preserve the existing component-specific invalidation rules. |
| Local-package content snapshot | Prevent edits between plan and apply. |

The plan additionally binds the project dependency lock, tool/runtime versions or image digest, execution policy, input snapshot, and generator/check identities.

Use the package manager’s resolved tree as the dependency-resolution authority; `spec.lock` can bind its digest and package instances rather than implementing a competing resolver. npm’s lock format already distinguishes resolved locations and artifact integrity. [npm lock documentation](https://docs.npmjs.com/cli/v11/configuring-npm/package-lock-json/)

**Separate static evaluation from execution.**

| Operation | Execution policy |
| --- | --- |
| Parse/import/plan | No package JS. Static files and declarative migration maps only. |
| Check | Isolated candidate inputs, scratch/cache writes, no publication rights. |
| Generator | Isolated inputs plus staging area; only validated, authorized output operations can publish. |
| Dependency installation | Dedicated toolchain operation; separate network and install-script policy. |

Default executor isolation should exclude host home directories, credentials, agent sockets, cloud metadata endpoints, and undeclared network access. Resource limits and child-process isolation apply to subprocesses too.

Build tools may write caches and generated scratch files. Those writes need disposable locations; they do not become declaration outputs automatically.

Disable dependency lifecycle scripts during ordinary package acquisition unless an explicit execution policy permits them. `ignore-scripts` does not prevent an explicitly requested `npm run` command from running its target script. [npm configuration documentation](https://docs.npmjs.com/cli/v11/using-npm/config/#ignore-scripts)

Filesystem diff validation remains necessary, but it cannot undo credential disclosure. Conversely, sandboxing cannot establish that a malicious check honestly verified anything. Evidence must identify the check producer and digest.

For reproducibility, v0 can conservatively hash the whole mounted candidate input tree. Precise read declarations and finer caching can come later.

**An upgrade must compare two interpretations, not reinterpret history.**

Suppose package v2 renames `page.route` to `page.pathname`, removes `rendering`, and renames output slot `page` to `entry`.

Required behaviour:

1. Load the baseline using its old lock and old definitions.
2. Evaluate the proposed project using the new definitions.
3. Diagnose invalid authored declarations before generating implementation work.
4. Preserve existing state and ownership until a successful transaction publishes.
5. Never infer that an invalid declaration was deleted.
6. Never infer that a removed output slot authorizes file deletion.

A minimal declarative package migration could contain:

```json
{
  "from_version": "1.4.0",
  "to_version": "2.0.0",
  "kind": "page",
  "rename_fields": {
    "route": "pathname"
  },
  "rename_outputs": {
    "page": "entry"
  }
}
```

Mappings must be injective and reject collisions. They preserve identity; they do not certify semantic compatibility.

A migration preview should show authored source edits, identity mappings, changed defaults, output relocation, changed grants, changed checks, and regenerated outputs. Removed fields with meaningful values require explicit resolution; they are not silently discarded.

General transformations should initially produce a reviewable project patch outside planning. Do not introduce arbitrary package migration code into graph evaluation.

Existing propagation rules then apply:

- Changed defaults affect canonical declarations.
- Changed output locators cause planned moves.
- Changed generators regenerate their generated outputs.
- Changed checks invalidate runs.
- Changed relations require recomputing grants and propagation.
- Removed slots require explicit disposition of their owned files.
- Published retained migrations remain immutable and retained.

If the old interpretation cannot be recovered, an upgrade cannot safely claim a field rename or ownership migration. Stop that migration rather than manufacturing a new baseline.

**E. The first milestone needs one adapter, not an ecosystem**

The [current milestone](/Users/mohebifar/dev/intentlang/SPEC.md:1392) already provides the right starting point. Its smallest package-shaped surface is:

| Needed now | Concrete content |
| --- | --- |
| Core callable model | Action input/output/errors, public and behaviour projections, fixed relations. |
| Next.js page descriptor | Supported static routes, resolved output slots, creation roots if exercised. |
| Implementation convention | One action per file, explicit callable entry points. |
| Toolchain baseline | One pinned Next.js/React/TypeScript/package-manager combination. |
| Checks | Typecheck and build. |
| Guidance | One pinned instruction bundle. |
| Execution boundary | Existing work-order/result protocol and ownership enforcement. |

The adapter may be bundled and hard-coded. **Do not make npm distribution, package-source parsing, capabilities, entities, migration generators, contribution registries, or `unit` prerequisites.**

Keep core semantic constructs in the compiler/prelude initially. Publishing `@intentlang/core` must not make `action` semantics replaceable by an npm upgrade. A later core package can distribute version-matched resources.

Validate the Next.js adapter against a deliberately narrow support statement: one App Router layout, static routes, and an exact tested dependency combination. Reject unsupported route constructs explicitly.

Use the existing ten-edit trial and control arm, with adapter-specific acceptance cases:

| Case | Required observation |
| --- | --- |
| Create page and action | Expected files, route behaviour, callable binding, and ownership. |
| Break an action input | Caller joins the required transaction. |
| Widen an action input compatibly | Caller verifies without an unconditional implementation task. |
| Change normative action prose | Observing caller receives review and verification obligations. |
| Move a route | Output identity survives; owned files relocate; references are repaired. |
| Occupied destination | No unmanaged overwrite or ownership theft. |
| Change an export or import grant | Stale bindings and unauthorized imports are detected. |
| Adopt a hand edit | Baseline renews while affected evidence becomes stale. |
| Attempt an out-of-envelope write | Publication is refused. |
| Change check/package inputs | Old evidence cannot satisfy the new binding. |

Acceptance tests must be independently authored and outside the implementation agent’s write set, as the milestone already requires. Exercise actual route behaviour; passing typecheck/build alone does not validate routing or caller assumptions.

Before calling the adapter a distributable package, also test its packed artifact in a clean project. That catches missing `.spec` files, undeclared dependencies, workspace-only resolution, and absent skill resources.

The general package API should be extracted from this working adapter. The evidence that would justify expanding it is a concrete adapter requirement that the closed vocabulary cannot express.

**Prioritised changes**

1. **Separate caller compatibility, behavioural obligations, and storage-transition safety.** Use variance for understood callable positions; keep unknown changes incompatible.
2. **Publish the closed kind metadata schema.** Include explicit code exports, reserved core fields, path semantics, and a clear boundary between value schemas and contract schemas.
3. **Specify the executor protocol and isolation boundary.** Packages cannot grant themselves writes; checks and generators cannot execute during planning.
4. **Define upgrades using old and new package interpretations.** Preserve state, output identity, and retained artifacts; reject silent field loss and inferred deletion.
5. **Keep the first milestone limited to the existing graph-driven Next.js trial.** Prove scoped updates against independent acceptance tests and the control.
6. **Support local packages with immutable snapshots and cheap scaffolding.** Omit core `unit`; expose missing conformance honestly.
7. **Defer custom relations, generic variance annotations, provider interfaces, contributions, and migration generators until a working adapter establishes their requirements.**