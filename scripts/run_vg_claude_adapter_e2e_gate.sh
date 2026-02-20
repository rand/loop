#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
MIN_MIB="${LOOP_MIN_AVAILABLE_MIB:-3072}"
TMP_OUT="$(mktemp -t vg-claude-adapter.XXXXXX)"
trap 'rm -f "$TMP_OUT"' EXIT

CMD="cd '$ROOT_DIR/rlm-core' && cargo test --no-default-features --features gemini test_execute_e2e_"

echo "[VG-CLAUDE-ADAPTER-E2E-001] running"
if ! LOOP_MIN_AVAILABLE_MIB="$MIN_MIB" "$SAFE_RUN" bash -lc "$CMD" | tee "$TMP_OUT"; then
    exit 1
fi

SUMMARY_LINE="$(rg -N 'test result: ok\.' "$TMP_OUT" | tail -n 1 || true)"
if [[ -z "$SUMMARY_LINE" ]]; then
    echo "VG-CLAUDE-ADAPTER-E2E-001: failing because cargo summary was not found" >&2
    exit 1
fi

if [[ "$SUMMARY_LINE" == *"0 passed"* ]]; then
    echo "VG-CLAUDE-ADAPTER-E2E-001: failing because zero scenario tests ran" >&2
    echo "summary: $SUMMARY_LINE" >&2
    exit 1
fi

PASSED_COUNT="$(echo "$SUMMARY_LINE" | sed -E 's/.*: ok\. ([0-9]+) passed;.*/\1/' || true)"
if [[ -z "$PASSED_COUNT" ]] || ! [[ "$PASSED_COUNT" =~ ^[0-9]+$ ]]; then
    echo "VG-CLAUDE-ADAPTER-E2E-001: failing because passed-test count could not be parsed" >&2
    echo "summary: $SUMMARY_LINE" >&2
    exit 1
fi

if (( PASSED_COUNT < 2 )); then
    echo "VG-CLAUDE-ADAPTER-E2E-001: failing because expected >=2 scenario tests, got $PASSED_COUNT" >&2
    echo "summary: $SUMMARY_LINE" >&2
    exit 1
fi

echo "VG-CLAUDE-ADAPTER-E2E-001: PASS"
