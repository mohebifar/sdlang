# Connecting an agent

`sdc apply` doesn't include a model. You give it two shell commands: a coding agent and a
reviewer. Any tool that can run headless works.

```bash
sdc apply --agent ./agent.sh --reviewer ./reviewer.sh
```

## The coding agent

The agent runs once per transaction, inside a fresh git worktree (the candidate), with:

| Input | What it is |
| --- | --- |
| stdin | the work order rendered as Markdown: the declarations to create or change, their requirements, the files they own and where new files go |
| `SPEC_WORK_ORDER` | the same order as JSON (`schemas/work-order.schema.json`) |
| `SPEC_RESULT` | where to write an optional result (`schemas/result.schema.json`) |
| `SPEC_MCP_CONFIG` | an MCP config that starts `sdc mcp` for this dispatch |
| `SPEC_DISPATCH` | the dispatch id |

The agent edits files and exits. It doesn't need to report what it changed: `sdc` diffs the
worktree itself. A result is only needed for what the diff can't say, such as `blocked`
with a reason, or a test bound to a requirement whose key its name doesn't carry. Test
cases are matched to requirements by tags like `[case:stored]` in the test name.

The MCP server gives the agent read access to the bound graph and context, and lets it run
the checks marked `self_check`:

| Tool | Does |
| --- | --- |
| `get_task` | the order's obligations, ownership and entry points |
| `get_declaration` | a projection of any declaration, with its hash |
| `read_context` | a context resource from the order's manifest |
| `get_diagnostic` | full logs of an earlier failure |
| `run_self_check` | runs a self-check, sandboxed; advisory only |
| `validate_result` | checks a draft result before writing it |

None of the tools write state or record evidence. Only `sdc` does that, after its own checks.

## The reviewer

The reviewer runs for each review, and must not be the session that wrote the code:

| Input | What it is |
| --- | --- |
| stdin | the review order rendered as Markdown |
| `SPEC_REVIEW_ORDER` | the review order as JSON (`schemas/review-order.schema.json`): each requirement, the code and the tests that claim to cover it |
| `SPEC_VERDICT` | where to write the verdict (`schemas/review-verdict.schema.json`) |

For each subject it answers whether the code conforms, and for tests whether they really
cover the requirement. An insufficient verdict sends the reason back to the agent.

## Example: Claude Code

These wrappers run Claude Code headless. The trial uses longer versions in
[trial/tools](../trial/tools) that also sandbox the agent.

`agent.sh`

```sh
#!/bin/sh
exec claude -p --mcp-config "$SPEC_MCP_CONFIG" --strict-mcp-config \
  --permission-mode acceptEdits \
  --allowedTools "Read,Glob,Grep,Bash,mcp__spec__get_task,mcp__spec__get_declaration,mcp__spec__read_context,mcp__spec__get_diagnostic,mcp__spec__run_self_check,mcp__spec__validate_result"
```

`reviewer.sh`

```sh
#!/bin/sh
exec claude -p --strict-mcp-config --permission-mode acceptEdits \
  --allowedTools "Read,Glob,Grep" --disallowedTools Bash \
  --append-system-prompt "You are a reviewer. Write only the verdict file named in the order; do not change any other file."
```

Both orders arrive on stdin, which `claude -p` reads as its prompt. `sdc` rejects a review
that changes any file other than the verdict. The full contract, including retries,
blocking and context assembly, is in the [protocol](protocol.md).
