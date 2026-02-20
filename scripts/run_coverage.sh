#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/coverage"
MIN_LINES="${COVERAGE_MIN_LINES:-80}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required for coverage execution" >&2
  exit 1
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  cat >&2 <<'EOF'
error: coverage gate unavailable locally because cargo-llvm-cov is not installed.

Install when network access is available:
  cargo install cargo-llvm-cov --locked

Then rerun:
  make coverage
EOF
  exit 2
fi

mkdir -p "${OUT_DIR}"

pushd "${ROOT_DIR}/rlm-core" >/dev/null
cargo llvm-cov clean --workspace
cargo llvm-cov \
  --workspace \
  --no-default-features \
  --features gemini \
  --lcov \
  --output-path "${OUT_DIR}/lcov.info"

cargo llvm-cov report \
  --summary-only \
  --fail-under-lines "${MIN_LINES}" \
  > "${OUT_DIR}/summary.txt"
popd >/dev/null

echo "coverage: wrote ${OUT_DIR}/lcov.info and ${OUT_DIR}/summary.txt"
