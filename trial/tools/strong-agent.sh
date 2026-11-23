#!/bin/sh
# The strong arm's coding agent (docs/trial.md, Setup): Claude Code headless with the pinned
# model, given the operator's instruction on stdin, in the arm's workspace. It may read,
# edit and run the project's tests, typecheck and build. Contained as the language arm's
# agent (agent.sh): sandboxed Bash with no network, edits only under the working directory,
# nothing in this repository readable. With TRIAL_LOG_DIR set, each session's record is
# kept there.
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
exec claude -p --model claude-sonnet-5-5 --strict-mcp-config \
  --settings "{\"sandbox\":{\"enabled\":true,\"autoAllowBashIfSandboxed\":true,\"allowUnsandboxedCommands\":false,\"filesystem\":{\"denyRead\":[$deny_json]}}}" \
  --permission-mode acceptEdits --allowedTools "Read,Glob,Grep,Bash" \
  --disallowedTools $deny_rules --output-format json > "$out"
