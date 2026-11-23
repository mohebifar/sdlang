# sdc: the sdlang CLI

A Rust implementation of the `sdc` tool described in `../docs/spec.md`. Its own plan is
written in sdlang: `spec/` is the spec tree, and `spec-packages/rust/` is a local package
that maps the language onto a Cargo workspace (a module is a crate under `crates/`, a `lib`
is one Rust module file, and a `command` is one clap subcommand).

```bash
cargo test --workspace
```

```bash
cargo run -p sdlang-cli -- parse ..
```

```bash
cargo run -p sdlang-cli -- hash --graph ../examples/graph.after.json
```

```bash
cargo run -p sdlang-cli -- graph .
```

```bash
cargo run -p sdlang-cli -- plan <project> --json
```

```bash
cargo run -p sdlang-cli -- apply <project> --agent "<command>" --reviewer "<command>"
```

```bash
cargo run -p sdlang-cli -- check <project>
```

```bash
cargo run -p sdlang-cli -- import src tests --dir <project> [--accept]
```

```bash
cargo run -p sdlang-cli -- lsp
```

`sdc lsp` is the language server editors start; `../editors/` has the VS Code extension
and setup for other editors.

## Where things are

| Spec | Code |
| --- | --- |
| `spec/syntax/` | `crates/syntax`: logos lexer, string scanning and prose dedent, syntax tree, parser, diagnostics |
| `spec/cli/` | `crates/cli`: the `sdc` binary (clap) and its commands |
| `spec/canon/` | `crates/canon`: canonical JSON, digests and projection hashes |
| `spec/graph/` | `crates/graph`: project and package loading, scopes and resolution, kind schemas, selectors, lowering to the canonical graph |
| `spec/plan/` | `crates/plan`: output materialisation, state records and the applied store, compatibility, the planner |
| `spec/apply/` | `crates/apply`: git candidates, work and review orders, the check protocol, evidence, the transaction engine |
| `spec/gate/` | `crates/gate`: `sdc check`'s findings, evidence currency, the ownership index and `--rerun` |
| `spec/adopt/` | `crates/adopt`: `sdc import`'s scan, proposal and acceptance |
| `spec/lsp/` | `crates/lsp`: `sdc lsp`, the language server: analysis, hover, navigation, completion, rename, diagnostics |

The checks are the ones the local package declares: `cargo build --locked`, `cargo clippy`
with warnings denied, and `cargo test`. The syntax crate's corpus test parses every
`spec` block in the design documents and every `.sd` file in the repository, and keeps its
grammar cases in step with `tools/check_examples.py`.

## Roadmap

Each phase is specified in `spec/` before it is built; later phases are only sketched there.

1. **Syntax** (done): `sdc parse [--json] <paths>`.
2. **Canon** (done): canonical JSON, digests and projection hashes, reproducing
   `examples/hash-vectors.json` and `examples/hashes.json`; `sdc hash [--canonical |
   --graph] <file>`. Interface fields come from the graph's `kinds` table, so a graph
   alone is enough to hash.
3. **Graph** (done): module discovery, packages, imports, resolution and lowering to the
   canonical graph, reproducing `examples/graph.before.json` and `graph.after.json` from
   `examples/worked/` (lock digest aside); `sdc graph [dir]`. `sdc graph .` lowers this
   CLI's own spec.
4. **State and plan** (done): state records and the applied store, drift, compatibility,
   propagation and transactions; `sdc plan [dir]`. Planning `examples/worked/after`
   against a baseline recorded from `examples/worked/before` reproduces
   `examples/plan.json`, digests aside. State is written only by a library function
   until phase 5 publishes through it.
5. **Apply**, in two parts. With phases 4 and 5 this is the runner `docs/trial.md` needs, and
   phase 3 removes the trial's hand-written graphs.
   - 5a (done): `sdc apply` with candidate worktrees, generators, work orders and
     results, envelope enforcement, build, test and conformance checks, review orders and
     verdicts, evidence, retries, one commit per transaction and the journal. Tested on a
     toy project with scripted agent, reviewer, generator and checks.
   - 5b (done): the MCP tool server (`sdc mcp`) and resource serving, the TypeScript
     import rule, process groups and timeouts, infrastructure reruns, and the
     `@sdlang/nextjs` adapter's checks, test reporter and install generator
     (`../adapters/nextjs`). Context packing beyond the inline part is still a fixed
     budget record. A real agent (`claude -p`) has applied the toy project end to end
     (docs/decisions.md C.17).
6. **Check and import**, in two parts.
   - 6a (done): `sdc check`, the read-only gate (`crates/gate`): unapplied spec, drift,
     ownership, evidence currency recomputed from input digests, reconcile and
     unspecified, with `--allow`, `--strict`, `--json` and `--rerun`.
   - 6b (done): `sdc import`, brownfield adoption (`crates/adopt`): proposals for
     existing files (slots, created outputs, new libs, grants, unresolved imports, test
     bindings from case tags, collisions), `--accept` into `.spec/import.json`, and
     strategy `acquire` in the planner and apply, whose unverified evidence the next plan
     obliges.

Nothing in `../docs/extensions.md` is in scope, except the two pieces docs/spec.md's worked example
needs: entity reader and writer compatibility, and a `use` binding's outputs and generator.
