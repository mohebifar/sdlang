#!/bin/sh
# The language arm's reviewer (TRIAL.md, Setup): the same model in its own session, which
# wrote none of the code. It reads, and writes only its verdict file. With TRIAL_LOG_DIR
# set, each session's record is kept there.
set -eu
out=/dev/stdout
if [ -n "${TRIAL_LOG_DIR:-}" ]; then out="$TRIAL_LOG_DIR/reviewer-$(date +%s)-$$.json"; fi
exec claude -p --model claude-sonnet-5-5 --strict-mcp-config --permission-mode acceptEdits \
  --allowedTools "Read,Glob,Grep,Write" --disallowedTools "Bash,Edit" --output-format json \
  --append-system-prompt "You are a reviewer. Write only the verdict file named in the order; do not change any other file." \
  > "$out"
