# spec: the int lang CLI

A Rust implementation of the `spec` tool described in `../SPEC.md`. Its own plan is
written in int lang: `spec/` is the spec tree, and `spec-packages/rust/` is a local package
that maps the language onto a Cargo workspace (a module is a crate under `crates/`, a `lib`
is one Rust module file, and a `command` is one clap subcommand).

```bash
cargo test --workspace
```

```bash
cargo run -p intlang-cli -- parse ..
```

```bash
cargo run -p intlang-cli -- hash --graph ../examples/graph.after.json
```

```bash
cargo run -p intlang-cli -- graph .
```

```bash
cargo run -p intlang-cli -- plan <project> --json
```

```bash
cargo run -p intlang-cli -- apply <project> --agent "<command>" --reviewer "<command>"
```

## Where things are

| Spec | Code |
| --- | --- |
| `spec/syntax/` | `crates/syntax`: logos lexer, string scanning and prose dedent, syntax tree, parser, diagnostics |
| `spec/cli/` | `crates/cli`: the `spec` binary (clap) and its commands |
| `spec/canon/` | `crates/canon`: canonical JSON, digests and projection hashes |
| `spec/graph/` | `crates/graph`: project and package loading, scopes and resolution, kind schemas, selectors, lowering to the canonical graph |
| `spec/plan/` | `crates/plan`: output materialisation, state records and the applied store, compatibility, the planner |
| `spec/apply/` | `crates/apply`: git candidates, work and review orders, the check protocol, evidence, the transaction engine |

The checks are the ones the local package declares: `cargo build --locked`, `cargo clippy`
with warnings denied, and `cargo test`. The syntax crate's corpus test parses every
`spec` block in the design documents and every `.int` file in the repository, and keeps its
grammar cases in step with `tools/check_examples.py`.

## Roadmap

Each phase is specified in `spec/` before it is built; later phases are only sketched there.

1. **Syntax** (done): `spec parse [--json] <paths>`.
2. **Canon** (done): canonical JSON, digests and projection hashes, reproducing
   `examples/hash-vectors.json` and `examples/hashes.json`; `spec hash [--canonical |
   --graph] <file>`. Interface fields come from the graph's `kinds` table, so a graph
   alone is enough to hash.
3. **Graph** (done): module discovery, packages, imports, resolution and lowering to the
   canonical graph, reproducing `examples/graph.before.json` and `graph.after.json` from
   `examples/worked/` (lock digest aside); `spec graph [dir]`. `spec graph .` lowers this
   CLI's own spec.
4. **State and plan** (done): state records and the applied store, drift, compatibility,
   propagation and transactions; `spec plan [dir]`. Planning `examples/worked/after`
   against a baseline recorded from `examples/worked/before` reproduces
   `examples/plan.json`, digests aside. State is written only by a library function
   until phase 5 publishes through it.
5. **Apply**, in two parts. With phases 4 and 5 this is the runner `TRIAL.md` needs, and
   phase 3 removes the trial's hand-written graphs.
   - 5a (done): `spec apply` with candidate worktrees, generators, work orders and
     results, envelope enforcement, build, test and conformance checks, review orders and
     verdicts, evidence, retries, one commit per transaction and the journal. Tested on a
     toy project with scripted agent, reviewer, generator and checks.
   - 5b: the MCP tool server (`spec mcp`), context packing and resource serving, the
     TypeScript import rule, timeouts, the `@intlang/nextjs` adapter's checks, test
     reporter and generators, and a run with a real agent.
6. `spec check` and `spec import`.

Nothing in `../EXTENSIONS.md` is in scope, except the two pieces SPEC.md's worked example
needs: entity reader and writer compatibility, and a `use` binding's outputs and generator.
