#!/bin/sh
# Makes one arm's workspace for one run (TRIAL.md, Arms and procedure): the baseline app,
# the arm's overlay (spec tree and adapter for language, docs/specs for strong, nothing for
# plain), a fresh git repository with one baseline commit, and dependencies installed from
# the lockfile. The acceptance tests are never copied in.
#
# Usage: tools/make-arm.sh <language | strong | plain> <destination>
set -eu
arm="$1"
dest="$2"
here=$(cd "$(dirname "$0")/.." && pwd)
case "$arm" in language | strong | plain) ;; *) echo "unknown arm: $arm" >&2; exit 2 ;; esac
if [ -e "$dest" ]; then echo "$dest exists" >&2; exit 1; fi
mkdir -p "$dest"
# The baseline without installed or built files.
(cd "$here/baseline" && find . -type f \
  ! -path './node_modules/*' ! -path './.next/*' ! -path './test-results/*' \
  ! -name 'next-env.d.ts' ! -name '*.tsbuildinfo' -print) |
  while read -r f; do mkdir -p "$dest/$(dirname "$f")"; cp "$here/baseline/$f" "$dest/$f"; done
if [ -d "$here/arms/$arm" ]; then cp -R "$here/arms/$arm/." "$dest/"; fi
cd "$dest"
git init -q
git add -A
git -c user.name=operator -c user.email=operator@localhost -c commit.gpgsign=false \
  commit -q -m "baseline ($arm arm)"
pnpm install --frozen-lockfile --reporter=silent
echo "made $arm arm at $dest ($(git rev-parse --short HEAD))"
