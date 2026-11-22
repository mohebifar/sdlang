#!/bin/sh
# The language arm's coding agent (TRIAL.md, Setup): Claude Code headless with the pinned
# model. It edits files and uses the spec MCP tools named in the work order; it has no
# shell. With TRIAL_LOG_DIR set, each session's record (tokens, cost, turns) is kept there.
#
# spec apply --agent trial/tools/agent.sh --reviewer trial/tools/reviewer.sh
set -eu
out=/dev/stdout
if [ -n "${TRIAL_LOG_DIR:-}" ]; then out="$TRIAL_LOG_DIR/agent-$(date +%s)-$$.json"; fi
exec claude -p --model claude-sonnet-5-5 --mcp-config "$SPEC_MCP_CONFIG" --strict-mcp-config \
  --permission-mode acceptEdits \
  --allowedTools "Read,Write,Edit,Glob,Grep,mcp__spec__get_task,mcp__spec__get_declaration,mcp__spec__read_context,mcp__spec__get_diagnostic,mcp__spec__run_self_check,mcp__spec__validate_result" \
  --disallowedTools "Bash" --output-format json > "$out"
