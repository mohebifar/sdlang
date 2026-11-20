# int lang: provisional extensions

These parts of the design are **not needed by the first milestone** (SPEC.md, First
milestone). They are kept because the worked example and later adapters need them, but
they are provisional: nothing here joins the core until the milestone has reported and a
later round accepts it. `tools/check_examples.py` parses the `spec` blocks here like
SPEC.md's.

Contents: Rules and selectors · Package annotations · Contributions · Bindings and
retained artifacts · Runtime state (entity changes, migrations and backfills, environments
and observations, rollout handoff) · Upgrades · Test first and journeys · Tooling plan.

## Rules and selectors

Rules select the declarations they apply to:

```spec
rule app_requires_session {
  applies_to: select nextjs.page where route matches "/app/**"
  expect {
    redirects {
      text: "unauthenticated visitors are redirected to @{auth.login}"
      verify: "tested"
    }
  }
}
```

`select` takes `*`, a kind, or a list of kinds, then an optional `where` predicate built
from `and`, `or`, `not`, parentheses and tests. A test is `exists field` or
`field op operand` with `==`, `!=`, `<`, `<=`, `>`, `>=`, `matches`, `in` or `contains`.
A bare name is a field of the candidate; `meta.module`, `meta.name` and `meta.kind` are
its metadata. `matches` is a full-string, case-sensitive glob in which `*` matches within
one path segment and `**` matches zero or more whole segments, so `/app/**` matches `/app`.
`contains` is collection membership, not substring search.

| Operator | Field type | Operand | A missing field |
| --- | --- | --- | --- |
| `==`, `!=` | any scalar, reference, or string-literal union | a literal or path of the same type; `null` | false for both |
| `<`, `<=`, `>`, `>=` | number, integer, date, timestamp | a literal of the same type | false |
| `matches` | string, `route_path` | a string glob | false |
| `in` | any scalar or reference | a list of operands of that type | false |
| `contains` | a list | one element of the list's element type | false |
| `exists` | any | none | false (and `not exists` true) |

A comparison whose operand type does not fit the field is a compile error. With several
selected kinds, a field must have the same type in every kind that declares it; kinds that
lack the field simply do not match a test on it.

To keep attachment cheap and deterministic, a selector sees only the candidate's authored
fields and schema defaults. It cannot follow references, read other rules, evidence,
files or state, or call anything. A rule applies to matching managed declarations in its
own module and below; a root rule applies project-wide. A selector that matches nothing is
a warning, since rules also cover declarations added later.

## Package annotations

An annotation definition, as a package writes it:

Each annotation is a declaration with a definition, never free-form metadata:

```spec
export annotation index {
  targets: ["entity_field"]
  projection: "public"
  repeatable: false
  parameters {
    method: "btree" | "hash" = "btree"
  }
}
```

A definition has `targets` (any of `entity_field`, `schema_field`, `kind_field`,
`declaration`), `projection` (`public`, `body` or `guidance`: where an application is
hashed), `repeatable` (default `false`) and an ordered `parameters` schema matched
positionally. The compiler resolves the name like any path (`@database.index("hash")`),
checks the target, the field type and the arguments, and rejects repeats unless allowed. Packages export annotations; their meaning is
consumed by the package's own checks and generators, never invented by the compiler.

## Contributions

Some files are shared by nature: navigation, middleware chains, route registries, message
catalogs, schema indexes. Each has one owner whose kind declares what it **accepts** and how
contributions combine. Contributors target an owner instance by name:

```spec
nextjs.nav main_nav {
  summary: "Top navigation"
}

nextjs.page login {
  route: "/login"
  contributes {
    nav: main_nav {
      slot: "account.sign_in"
      label: "Sign in"
      to: self
      order: 20
    }
  }
}
```

- **Identity and checking.** `auth.login#contributes.nav` is the contribution's identity.
  `main_nav { ... }` is checked against the owner kind's accepts schema.
- **Aggregation contract.** The owner kind declares ordering (explicit `order`, ties broken
  by contributor address), exclusive keys (two contributions claiming `slot:
  "account.sign_in"` fail validation), and any domain rules (middleware order, route
  precedence, duplicate message keys). Without such a contract, a "contribution" would just
  be an arbitrary patch in disguise; that case is handled by giving the file one owner and
  planning an integration task for it.
- **Propagation.** The edge is written contributor → owner, but the owner consumes an
  *aggregate input*: every contribution payload plus the projections of declarations they
  reference (`to: self` carries `login`'s route, so a route change reaches the
  registry). A contribution that references a callable grants the owner an import of that
  callable's public entry point only.
- **Implicit membership.** An owner kind may also declare `collects: select entity` to
  aggregate declarations by selector, with the same restrictions as rule selectors. This is
  how a schema index lists every entity without each entity writing a contribution.
- **Materialisation.** Preferably the owner's package generator writes the aggregate into a
  registry file the owner owns (`src/generated/nav.ts`), from the applied graph plus the
  current transaction, after aggregate validation and before checks. A contribution change
  is then generator work. Contributors that change in one plan merge into one transaction
  through their shared owner.
- **Owner deletion.** An owner cannot be deleted while contributions target it; the same
  plan must remove or retarget them, or transfer ownership.

## Bindings and retained artifacts

Owner kinds may add two sections, `accepts { }` (the contribution schema, with `order` and
`exclusive` keys) and `collects: select ...` (implicit membership, with rule-selector
restrictions); see Contributions. A package defines what a `use` binding owns with the
package-only form `binding <capability> { }`.

A `binding` takes `options` (the type `use` options must satisfy), `outputs`, `creates`,
`collects`, `retains`, `generator`, `observer`, `checks` and `skills`. `retains` maps a root
key to `{ path, producer }`, where `producer` is `"generator"` (the binding's generator
writes each artifact, as with migrations) or `"agent"` (an agent dispatch writes exactly one
new file under the root, as with backfills); retained artifacts are immutable once
published. `observer` names an `observer` definition (package-only, like `generator`, with a
`run` argv) that reads an environment for `spec observe`. `examples/drizzle-binding.int`
writes the worked example's database provider out in full: its `binding database`,
`implements entity`, annotations, check, generator and observer.

- **A `use` binding owns its integration files** when its package defines a `binding` for
  that capability (above): the database binding owns
  `drizzle.config.ts`, the schema index (`collects: select entity`) and the retained
  migration and backfill roots. Its address is its capability name. Bindings whose package
  defines no `binding` remain context.
- **Retained artifacts.** A published migration or backfill is retained: it is immutable,
  never deleted (not even when its entity is deleted), and recorded in the state of the
  binding that owns its root. Each retained root names its producer: migrations are written
  by the binding's generator, backfills by an agent dispatch whose envelope is one new file
  under the backfill root. A mistake in a published migration is corrected by a later migration.
  Publishing a migration file says nothing about whether a database has run it (see Runtime state).

## Runtime state

The CLI produces source: code, migrations, backfill code, all retained and reviewed. It
**observes** environments. It never runs migrations or deploys. Three facts are kept
apart: the source transaction verified; the rollout prerequisites for a target; and whether
that target is ready, which is often unknown.

### Entity changes

Each entity change is classified on separate dimensions:

```json
{
  "entity": "core.Workspace",
  "change": "add_field",
  "field": "archived_at",
  "compatibility": { "existing_readers": "compatible", "existing_writers": "compatible",
                     "assumptions": ["named-column access"] },
  "storage": { "data_loss": "none", "backfill": "not_required", "operational_cost": "unknown" },
  "rollout": { "requires": ["migration:workspace_archived_at"] }
}
```

Readers (`reads` edges) consume reader compatibility; writers (`writes` edges) consume writer
compatibility; a declaration that does both consumes both. Adding a required field with no
database default is writer-incompatible; making a field optional is reader-incompatible;
unknown stays unknown and is treated as incompatible. The plan's summary words
(`additive`, `backfill`, `destructive`) are labels over these, not the classification.
`--allow-destructive` authorises a destructive operation; it proves neither readiness nor
recoverability. A field rename is `rename_field` through `moved` (SPEC.md, Moves).

### Migrations and backfills

Migrations are retained artifacts of the database binding, each with an immutable identity,
checksum and dependencies. A backfill is an explicit operation, not prose:

```spec
export entity Workspace {
  fields {
    tenant_id: uuid? @database.backfill("tenant_from_owner_v1")
  }
}
```

The binding retains its manifest beside the code it runs: the migration it requires, the
artifact digest, the execution mode (batched, one transaction per batch, a resume key, an
idempotent retry contract) and a completion check that validates the data, including rows
written during the backfill. A cursor reaching the last key is not completion. An agent
writes the backfill code; checks and review judge it against the manifest. Progress lives in
the environment, never in committed state, and an applied schema migration never implies a
completed backfill.

### Environments and observations

```spec
environment production {
  database {
    binding: database
    connection: "DATABASE_URL"
    expected_resource: "acme-prod-primary"
  }
}
```

`connection` names a variable, not a database, so the observer also checks
`expected_resource`. `spec observe --env production` runs the binding's observer, with
narrowly scoped read credentials supplied by the runner, and writes an immutable
**observation receipt**: environment, resource id and generation (to notice restores), the
observer's digest, the time, the ledger digest, applied migrations with checksums, and
backfill completion. An unreachable environment yields `unknown`, never "not applied".
Different environments are evaluated separately against migration dependencies. A ledger
is evidence about the ledger, not proof that schema and data still match it.

Receipts are bound into a plan revision by digest; observing again never silently refreshes
an existing plan's authority.

### Rollout handoff

Source transactions never wait for a production migration: the migration has to be
published before anyone can run it. Instead, `spec rollout --target production` writes a
machine-readable handoff: artifact digests, migration and backfill dependencies, target
resources, and the preconditions to re-check at execution time (for a dropped column: the
expand migration applied, the backfill complete, no running release still reading the
column). The deployment runner revalidates those under its own concurrency control, runs
the operations and returns receipts. Without a participating runner, readiness is reported
as unknown; the CLI never claims a rollout is safe.

## Upgrades

An upgrade compares two interpretations: the baseline under its old lock, and the
proposed project under the new one. Invalid authored declarations are diagnosed before any
work is planned; existing state and ownership stay until a transaction publishes; an
invalid declaration is never inferred to be deleted, and a removed output slot never
authorises deleting its file. A package may ship a declarative migration map
(`{ "kind": "page", "rename_fields": { "route": "pathname" }, "rename_outputs": { "page": "entry" } }`),
which must be injective and preserves identity only; it certifies nothing about behaviour.
Removed fields that carried values need an explicit resolution. If the old interpretation
cannot be recovered, the upgrade stops rather than inventing a baseline.

## Test first and journeys

**Test first (optional).** With `test_first: true` on the project (or on a module header,
for that module), a new or changed requirement revision
whose policy includes tests gets its test written first, by a separate dispatch whose
envelope covers the test output and the support files it needs. The CLI records the initial
outcome honestly (assertion failure, setup failure, already passing, not executable yet)
and then protects the approved oracle digest: the implementer cannot change it. If the
pre-written test is wrong (a fixture with the wrong session, say), the implementer proposes
an **oracle amendment**, an exact patch with old and new oracle digests that a reviewer
approves or rejects; approval requires fresh coverage and runs, and the rejected oracle stays
in history. Test-first limits who edits the oracle; it does not prove the oracle reads the
requirement correctly, which is why coverage is still reviewed.

**Requirements that span declarations.** An `expect` member may describe a journey ("a
visitor signs up, lands on @{dashboard.home} and stays signed in after reloading"). Its test
is owned by the declaration that states it and exercises the running application; coverage
review names any gap (a mocked redirect covers part of a journey, not all of it). Such suites
should run under `verify_scope: "all"`, because mentions alone do not propagate every change a
journey depends on. A reusable `flow` kind is deferred; it would add structure, not a new
evidence system.

## Tooling plan

One parser with source spans and
preserved comments serving compiler, diagnostics and formatter; syntax highlighting; a
formatter that never rewrites prose; then a thin language server (diagnostics, completion,
hover, go to definition, references) on the compiler's resolver, with rename deferred until
it can emit the `moved` statements a rename needs. A TypeScript frontend can come later if
the milestone shows demand; it would lower to the same graph and pass the same fixtures.
