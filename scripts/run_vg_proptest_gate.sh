#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
MIN_MIB="${LOOP_MIN_AVAILABLE_MIB:-3072}"

# Deterministic and reproducible proptest configuration.
PROPTEST_CASES_VALUE="${PROPTEST_CASES:-96}"
PROPTEST_RNG_SEED_VALUE="${PROPTEST_RNG_SEED:-424242}"
PROPTEST_RNG_ALGORITHM_VALUE="${PROPTEST_RNG_ALGORITHM:-cc}"

run_scope() {
    local scope="$1"
    local tmp_out
    tmp_out="$(mktemp -t vg-proptest.XXXXXX)"
    trap 'rm -f "$tmp_out"' RETURN

    local cmd
    cmd="cd '$ROOT_DIR/rlm-core' && PROPTEST_CASES='$PROPTEST_CASES_VALUE' PROPTEST_RNG_SEED='$PROPTEST_RNG_SEED_VALUE' PROPTEST_RNG_ALGORITHM='$PROPTEST_RNG_ALGORITHM_VALUE' cargo test --no-default-features --features gemini '$scope'"

    echo "[VG-PROPTEST-001] scope=$scope"
    if ! LOOP_MIN_AVAILABLE_MIB="$MIN_MIB" "$SAFE_RUN" bash -lc "$cmd" | tee "$tmp_out"; then
        return 1
    fi

    local summary
    summary="$(rg -N 'test result: ok\.' "$tmp_out" | tail -n 1 || true)"
    if [[ -z "$summary" ]]; then
        echo "VG-PROPTEST-001: failing because no cargo test summary was found for scope '$scope'" >&2
        return 1
    fi

    if [[ "$summary" == *"0 passed"* ]]; then
        echo "VG-PROPTEST-001: failing because scope '$scope' executed zero tests" >&2
        echo "summary: $summary" >&2
        return 1
    fi
}

run_scope "epistemic::proptest::tests::"
run_scope "signature::validation::tests::prop_"
run_scope "signature::fallback::tests::prop_"
run_scope "llm::router::tests::prop_"

echo "VG-PROPTEST-001: PASS (cases=$PROPTEST_CASES_VALUE seed=$PROPTEST_RNG_SEED_VALUE rng=$PROPTEST_RNG_ALGORITHM_VALUE)"
