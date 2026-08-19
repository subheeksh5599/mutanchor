#!/usr/bin/env bash
# Publish a generated mutation report to the demo site.
#
# Copies the CLI's publishable report.json (the exact shape the /dashboard
# panel renders) into frontend/public/ so the Vercel build serves it at the
# site root. Run AFTER `mutanchor run`.
#
# Usage:
#   scripts/publish-report.sh [path-to-report.json]  (default: target/mutanchor/report.json)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${1:-$REPO_ROOT/target/mutanchor/report.json}"
DEST="$REPO_ROOT/frontend/public/report.json"

if [[ ! -f "$SRC" ]]; then
  echo "error: no report at $SRC — run \`mutanchor run\` first" >&2
  exit 1
fi

mkdir -p "$(dirname "$DEST")"
cp "$SRC" "$DEST"
echo "published $SRC -> $DEST"
echo "deploy frontend/ to Vercel; /dashboard will render this real report"
