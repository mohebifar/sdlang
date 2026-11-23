#!/bin/sh
# Makes one arm's workspace for one run (docs/trial.md, Arms and procedure): the baseline app,
# the arm's overlay (spec tree and adapter for language, docs/specs for strong, nothing for
# plain), a fresh git repository with one baseline commit, and dependencies installed from
# the lockfile. The acceptance tests are never copied in.
#
# With "scratch", the app code and its tests are left out: the workspace holds only the
# toolchain files and the root layout, so the agent builds the app from the arm's spec.
#
# Usage: tools/make-arm.sh <language | strong | plain> <destination> [scratch]
set -eu
arm="$1"
dest="$2"
from="${3:-baseline}"
case "$from" in baseline | scratch) ;; *) echo "unknown start: $from" >&2; exit 2 ;; esac
here=$(cd "$(dirname "$0")/.." && pwd)
case "$arm" in language | strong | plain) ;; *) echo "unknown arm: $arm" >&2; exit 2 ;; esac
if [ -e "$dest" ]; then echo "$dest exists" >&2; exit 1; fi
mkdir -p "$dest"
# The baseline without installed or built files.
(cd "$here/baseline" && find . -type f \
  ! -path './node_modules/*' ! -path './.next/*' ! -path './test-results/*' \
  ! -name 'next-env.d.ts' ! -name '*.tsbuildinfo' -print) |
  if [ "$from" = scratch ]; then grep -v -e '^\./tests/' -e '^\./src/' -e '^\./\.gitignore$' || true; echo ./.gitignore; echo ./src/app/layout.tsx; else cat; fi |
  while read -r f; do mkdir -p "$dest/$(dirname "$f")"; cp "$here/baseline/$f" "$dest/$f"; done
if [ -d "$here/arms/$arm" ]; then cp -R "$here/arms/$arm/." "$dest/"; fi
cd "$dest"
git init -q
git add -A
git -c user.name=operator -c user.email=operator@localhost -c commit.gpgsign=false \
  commit -q -m "baseline ($arm arm)"
pnpm install --frozen-lockfile --reporter=silent
echo "made $arm arm at $dest ($(git rev-parse --short HEAD))"
