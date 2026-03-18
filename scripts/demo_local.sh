#!/usr/bin/env bash
# Demo: Run cloudshift CLI transform on all fixture files to prove patterns work locally.
#
# Usage: from repo root:
#   ./scripts/demo_local.sh
#
# Requires: cargo, patterns/ and tests/patterns/ in repo. Uses CLOUDSHIFT_CATALOGUE_PATH=./patterns
# (or discovers ./patterns from CWD).

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
PATTERNS="$REPO_ROOT/patterns"
FIXTURES="$REPO_ROOT/tests/patterns"

if [ ! -d "$PATTERNS" ]; then
  echo "Patterns dir not found: $PATTERNS"
  exit 1
fi
if [ ! -d "$FIXTURES" ]; then
  echo "Fixtures dir not found: $FIXTURES"
  exit 1
fi

export CLOUDSHIFT_CATALOGUE_PATH="$PATTERNS"
CLI="cargo run -p cloudshift-cli --quiet --"

echo "Demo local transform (catalogue: $PATTERNS)"
echo ""

for dir in "$FIXTURES"/python/*/; do
  [ -d "$dir" ] || continue
  name=$(basename "$dir")
  before="$dir/before.py"
  if [ ! -f "$before" ]; then
    echo "[SKIP] $name — no before.py"
    continue
  fi
  echo "--- $name ---"
  if $CLI transform "$before" --source aws 2>&1 | head -80; then
    echo "[OK] $name"
  else
    echo "[FAIL] $name"
  fi
  echo ""
done

echo "Done. Each fixture should show a diff when patterns match."
