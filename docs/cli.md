# CLI reference

`sdc <command> --help` prints the same. Every command that takes a project directory defaults to the current one.

## sdc graph

Compile the spec tree to the canonical graph. Use it to check that the spec compiles.

```text
Usage: sdc graph [DIR]

Arguments:
  [DIR]  The project directory (holding spec/ and, optionally, sd.lock) [default: .]

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## sdc plan

Compare spec, state and files on disk, print the plan, and save it as `plan.json`. Runs no model.

```text
Usage: sdc plan [OPTIONS] [DIR]

Arguments:
  [DIR]  The project directory (holding spec/, sd.lock and .spec/) [default: .]

Options:
      --id <ID>       The plan id; defaults to p-<UTC date>-<nn>, the day's first number not yet applied
      --out <OUT>     Where to write the plan document; defaults to plan.json in the project
      --json          Print the plan document to stdout instead of writing it
      --adopt <PATH>  Resolve a drifted file with `adopt`: its edited bytes become the baseline and the declaration is verified and reviewed against them (repeatable)
  -h, --help          Print help
  -V, --version       Print version
```

## sdc apply

Run a saved plan transaction by transaction. See [Connecting an agent](agents.md).

```text
Usage: sdc apply [OPTIONS] [DIR]

Arguments:
  [DIR]  The project directory (the top of its git work tree) [default: .]

Options:
      --plan <PLAN>                    The plan to apply; defaults to plan.json in the project
      --agent <AGENT>                  Shell command run in the candidate for each coding dispatch. It reads the work order on stdin (Markdown) or from $SPEC_WORK_ORDER and writes its result to $SPEC_RESULT
      --reviewer <REVIEWER>            Shell command run for each review. It reads $SPEC_REVIEW_ORDER and writes $SPEC_VERDICT; it must not have written any of the code it reviews
      --profile <PROFILE>              The execution profile checks run under [default: local]
      --max-attempts <MAX_ATTEMPTS>    Coding attempts per transaction [default: 3]
      --agent-timeout <AGENT_TIMEOUT>  Seconds an agent or reviewer may run before its process group is stopped [default: 1800]
      --check-timeout <CHECK_TIMEOUT>  Seconds a check or generator may run [default: 900]
  -h, --help                           Print help
  -V, --version                        Print version
```

## sdc check

The read-only gate for CI. Exits non-zero on unapplied changes, drift, ownership problems or stale evidence.

```text
Usage: sdc check [OPTIONS] [DIR]

Arguments:
  [DIR]  The project directory (the top of its git work tree) [default: .]

Options:
      --allow <KIND>                   Report findings of this kind without failing (repeatable)
      --strict                         Fail on unspecified declarations too
      --rerun                          Also rerun every test run the evidence relies on, in a scratch worktree at HEAD
      --profile <PROFILE>              The execution profile evidence was recorded under [default: local]
      --check-timeout <CHECK_TIMEOUT>  Seconds a rerun check may run [default: 900]
      --json                           Print the findings as a JSON array
  -h, --help                           Print help
  -V, --version                        Print version
```

## sdc import

Adopt existing code: propose which files each declaration owns. See [Getting started](getting-started.md#4-adopt-the-existing-code).

```text
Usage: sdc import [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Files or directories to adopt, relative to the project

Options:
      --dir <DIR>  The project directory (holding spec/ and .spec/) [default: .]
      --accept     Write the proposed libs and .spec/import.json instead of only printing the proposal
  -h, --help       Print help
  -V, --version    Print version
```

## sdc lsp

The language server, started by editors. See [Editors](../editors/README.md).

```text
Usage: sdc lsp [OPTIONS]

Options:
      --stdio    Accepted for clients that pass it; stdio is the only transport
  -h, --help     Print help
  -V, --version  Print version
```

## sdc mcp

The MCP server for one dispatch. `sdc apply` configures it for the agent through `SPEC_MCP_CONFIG`; you don't run it yourself.

```text
Usage: sdc mcp [OPTIONS] --dispatch <DISPATCH>

Options:
      --dispatch <DISPATCH>  The dispatch id from the work order
      --dir <DIR>            The candidate worktree; defaults to the current directory [default: .]
  -h, --help                 Print help
  -V, --version              Print version
```

## sdc parse

Parse `.sd` files and report syntax errors.

```text
Usage: sdc parse [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Files or directories; directories are searched for *.sd files

Options:
      --json     Print one JSON object per file (its path and tree, or its error) instead of text
  -h, --help     Print help
  -V, --version  Print version
```

## sdc hash

Canonical JSON, digests and projection hashes, for tooling and debugging.

```text
Usage: sdc hash [OPTIONS] <FILE>

Arguments:
  <FILE>  A JSON file

Options:
      --canonical  Print the canonical JSON text instead of its digest
      --graph      Treat the file as a canonical graph and print each declaration's projection hashes
  -h, --help       Print help
  -V, --version    Print version
```
