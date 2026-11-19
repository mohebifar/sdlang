#!/usr/bin/env bash
# Run one Codex design round, read-only, fresh ephemeral session.
# Usage: tools/run-round.sh rounds/NN-topic
set -euo pipefail
base="$1"
cd "$(dirname "$0")/.."
codex exec -m gpt-6.1-sol -c model_reasoning_effort='"high"' -s read-only --ephemeral \
  -C "$PWD" -o "$base.codex.md" - < "$base.prompt.md" > "$base.log" 2>&1
echo "exit=$? -> $base.codex.md ($(wc -l < "$base.codex.md") lines)"
