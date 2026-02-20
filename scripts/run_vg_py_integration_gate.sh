#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
MIN_MIB="${LOOP_MIN_AVAILABLE_MIB:-3072}"
PYTHON_BIN="$ROOT_DIR/rlm-core/.venv/bin/python"

TMP_OUT="$(mktemp -t vg-py-integration.XXXXXX)"
trap 'rm -f "$TMP_OUT"' EXIT

if [[ ! -x "$PYTHON_BIN" ]]; then
    echo "VG-PY-INTEGRATION-001: missing $PYTHON_BIN (run: cd rlm-core && uv sync --extra dev)" >&2
    exit 1
fi

CMD="cd '$ROOT_DIR/rlm-core' && '$PYTHON_BIN' -m pytest -q tests/integration/test_upgrade_compatibility.py"

echo "[VG-PY-INTEGRATION-001] running"
if ! LOOP_MIN_AVAILABLE_MIB="$MIN_MIB" "$SAFE_RUN" bash -lc "$CMD" | tee "$TMP_OUT"; then
    exit 1
fi

SUMMARY_LINE="$(rg -N '={3,} .* in [0-9.]+s ={3,}' "$TMP_OUT" | tail -n 1 || true)"
if [[ -z "$SUMMARY_LINE" ]]; then
    echo "VG-PY-INTEGRATION-001: failing because pytest summary was not found" >&2
    exit 1
fi

if [[ "$SUMMARY_LINE" == *"no tests ran"* ]]; then
    echo "VG-PY-INTEGRATION-001: failing because no tests ran" >&2
    exit 1
fi

if [[ "$SUMMARY_LINE" == *" skipped"* ]] && [[ "$SUMMARY_LINE" != *" passed"* ]]; then
    echo "VG-PY-INTEGRATION-001: failing because run is all-skipped" >&2
    echo "summary: $SUMMARY_LINE" >&2
    exit 1
fi

if [[ "$SUMMARY_LINE" != *" passed"* ]]; then
    echo "VG-PY-INTEGRATION-001: failing because no passing tests were observed" >&2
    echo "summary: $SUMMARY_LINE" >&2
    exit 1
fi

echo "VG-PY-INTEGRATION-001: PASS"
