#!/bin/sh
# The language arm's coding agent (docs/trial.md, Setup): Claude Code headless with the pinned
# model, run by sdc apply in the transaction's candidate. Its tools are the strong arm's
# (strong-agent.sh) plus the spec MCP tools named in the work order. Contained: Bash runs in
# Claude Code's sandbox (writes only under the working directory, no network), file edits
# are allowed only under the working directory, and nothing in this repository can be read.
# With TRIAL_LOG_DIR set, each session's record (tokens, cost, turns) is kept there.
#
# sdc apply --agent trial/tools/agent.sh --reviewer trial/tools/reviewer.sh
set -euf
repo=$(cd "$(dirname "$0")/../.." && pwd)
# Unreadable: this repository, and whatever TRIAL_DENY_READ lists (colon-separated), such as
# the other arms' workspaces.
deny_json="\"$repo\""; deny_rules="Read(/$repo/**) Edit(/$repo/**)"
for d in $(echo "${TRIAL_DENY_READ:-}" | tr ':' ' '); do
  deny_json="$deny_json,\"$d\""; deny_rules="$deny_rules Read(/$d/**) Edit(/$d/**)"
done
out=/dev/stdout
if [ -n "${TRIAL_LOG_DIR:-}" ]; then out="$TRIAL_LOG_DIR/agent-$(date +%s)-$$.json"; fi
exec claude -p --model claude-sonnet-5-5 --mcp-config "$SPEC_MCP_CONFIG" --strict-mcp-config \
  --settings "{\"sandbox\":{\"enabled\":true,\"autoAllowBashIfSandboxed\":true,\"allowUnsandboxedCommands\":false,\"filesystem\":{\"denyRead\":[$deny_json]}}}" \
  --permission-mode acceptEdits --allowedTools "Read,Glob,Grep,Bash,mcp__spec__get_task,mcp__spec__get_declaration,mcp__spec__read_context,mcp__spec__get_diagnostic,mcp__spec__run_self_check,mcp__spec__validate_result" \
  --disallowedTools $deny_rules --output-format json > "$out"
