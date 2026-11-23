#!/bin/sh
# The language arm's reviewer (docs/trial.md, Setup): the same model in its own session, which
# wrote none of the code. It reads the candidate, and writes only its verdict file; edits
# are allowed only under the working directory and nothing in this repository can be read.
# With TRIAL_LOG_DIR set, each session's record is kept there.
set -euf
repo=$(cd "$(dirname "$0")/../.." && pwd)
# Unreadable: this repository, and whatever TRIAL_DENY_READ lists (colon-separated), such as
# the other arms' workspaces.
deny_json="\"$repo\""; deny_rules="Read(/$repo/**) Edit(/$repo/**)"
for d in $(echo "${TRIAL_DENY_READ:-}" | tr ':' ' '); do
  deny_json="$deny_json,\"$d\""; deny_rules="$deny_rules Read(/$d/**) Edit(/$d/**)"
done
out=/dev/stdout
if [ -n "${TRIAL_LOG_DIR:-}" ]; then out="$TRIAL_LOG_DIR/reviewer-$(date +%s)-$$.json"; fi
exec claude -p --model claude-sonnet-5-5 --strict-mcp-config \
  --permission-mode acceptEdits --allowedTools "Read,Glob,Grep" --disallowedTools Bash $deny_rules \
  --output-format json \
  --append-system-prompt "You are a reviewer. Write only the verdict file named in the order; do not change any other file." \
  > "$out"
