#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v rg >/dev/null 2>&1; then
  echo "error: 'rg' is required for docs style checks" >&2
  exit 2
fi

scan_paths=(
  "docs"
  "README.md"
  "rlm-core/README.md"
  "rlm-core/python/README.md"
)

exclude_globs=(
  "--glob=!docs/execution-plan/evidence/**"
)

echo "[docs-check] scanning for forbidden em dash punctuation..."
if rg -n "—" "${scan_paths[@]}" "${exclude_globs[@]}" >/tmp/loop_docs_em_dash_hits.txt; then
  echo "error: found typographic em dash punctuation in docs:" >&2
  cat /tmp/loop_docs_em_dash_hits.txt >&2
  echo "hint: replace with ASCII punctuation (hyphen/comma/colon)." >&2
  exit 1
fi

rm -f /tmp/loop_docs_em_dash_hits.txt
echo "[docs-check] pass: no em dash punctuation detected."
