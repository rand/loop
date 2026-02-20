#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOOP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IO_RFLX_DIR="${IO_RFLX_DIR:-/Users/rand/src/io-rflx}"
RFLX_CARGO_TARGET_DIR="${RFLX_CARGO_TARGET_DIR:-/tmp/io-rflx-cargo-target}"
FIXTURES_DIR="${FIXTURES_DIR:-$LOOP_ROOT/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0}"

mkdir -p "$RFLX_CARGO_TARGET_DIR"

python3 "$LOOP_ROOT/scripts/validate_rflx_interop_fixtures.py" --fixtures-dir "$FIXTURES_DIR"

run_roundtrip_test() {
  local filter="$1"
  echo "running io-rflx roundtrip test filter: $filter"
  (
    cd "$IO_RFLX_DIR"
    CARGO_TARGET_DIR="$RFLX_CARGO_TARGET_DIR" cargo test -p rflx-core "$filter"
  )
}

run_roundtrip_test "provenance_serialization_roundtrip"
run_roundtrip_test "jsonl_roundtrip"
run_roundtrip_test "verification_result_serialization"

echo "VG-RFLX-002 fixture gate: PASS"
echo "io_rflx_dir=$IO_RFLX_DIR"
echo "rflx_cargo_target_dir=$RFLX_CARGO_TARGET_DIR"
echo "fixtures_dir=$FIXTURES_DIR"
