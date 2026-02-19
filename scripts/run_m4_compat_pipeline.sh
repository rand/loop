#!/usr/bin/env bash
set -euo pipefail

# Deterministic M4 compatibility pipeline:
# 1) rlm-claude-code critical suite
# 2) loop-agent seam subset
# 3) io-rflx core compile
#
# Optional:
# - full loop-agent snapshot gate (advisory)

ROOT_DIR="/Users/rand/src/loop"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
RCC_DIR="${RCC_DIR:-/Users/rand/src/rlm-claude-code}"
LOOP_AGENT_DIR="${LOOP_AGENT_DIR:-/Users/rand/src/loop-agent}"
IO_RFLX_DIR="${IO_RFLX_DIR:-/Users/rand/src/io-rflx}"
RFLX_CARGO_TARGET_DIR="${RFLX_CARGO_TARGET_DIR:-/tmp/io-rflx-cargo-target}"

EVIDENCE_DATE="${EVIDENCE_DATE:-$(date +%F)}"
EVIDENCE_DIR_DEFAULT="$ROOT_DIR/docs/execution-plan/evidence/$EVIDENCE_DATE/milestone-M4"
EVIDENCE_DIR="${EVIDENCE_DIR:-$EVIDENCE_DIR_DEFAULT}"

MIN_AVAILABLE_MIB="${LOOP_MIN_AVAILABLE_MIB:-4096}"
RUN_LA_FULL_SNAPSHOT="${RUN_LA_FULL_SNAPSHOT:-0}"
PIPELINE_STRICT="${PIPELINE_STRICT:-1}"

mkdir -p "$EVIDENCE_DIR"

PIPELINE_LOG="$EVIDENCE_DIR/M4-T04-pipeline-run.log"
SUMMARY_MD="$EVIDENCE_DIR/M4-T04-pipeline-summary.md"

: >"$PIPELINE_LOG"
exec >>"$PIPELINE_LOG" 2>&1

echo "M4 compatibility pipeline starting"
echo "date: $(date -u '+UTC %Y-%m-%d %H:%M:%S')"
echo "evidence_dir: $EVIDENCE_DIR"
echo "loop_min_available_mib: $MIN_AVAILABLE_MIB"
echo "run_la_full_snapshot: $RUN_LA_FULL_SNAPSHOT"
echo "pipeline_strict: $PIPELINE_STRICT"
echo "rcc_dir: $RCC_DIR"
echo "loop_agent_dir: $LOOP_AGENT_DIR"
echo "io_rflx_dir: $IO_RFLX_DIR"
echo "rflx_cargo_target_dir: $RFLX_CARGO_TARGET_DIR"
echo

mkdir -p "$RFLX_CARGO_TARGET_DIR"

if [[ -z "${RCC_PYTEST_CMD:-}" ]]; then
  if [[ -x "$RCC_DIR/.venv/bin/pytest" ]]; then
    RCC_PYTEST_CMD="$RCC_DIR/.venv/bin/pytest"
  else
    RCC_PYTEST_CMD="uv run --offline pytest"
  fi
fi

if [[ -z "${LA_PYTEST_CMD:-}" ]]; then
  if [[ -x "$LOOP_AGENT_DIR/.venv/bin/pytest" ]]; then
    LA_PYTEST_CMD="$LOOP_AGENT_DIR/.venv/bin/pytest"
  else
    LA_PYTEST_CMD="uv run --offline pytest"
  fi
fi

LA_PYTHONPATH="${LA_PYTHONPATH:-}"
LA_ENV_PREFIX=""
if [[ -n "$LA_PYTHONPATH" ]]; then
  LA_ENV_PREFIX="PYTHONPATH='$LA_PYTHONPATH' "
fi

echo "rcc_pytest_cmd: $RCC_PYTEST_CMD"
echo "la_pytest_cmd: $LA_PYTEST_CMD"
if [[ -n "$LA_PYTHONPATH" ]]; then
  echo "la_pythonpath: $LA_PYTHONPATH"
fi
echo

run_gate() {
  local gate_id="$1"
  local shell_cmd="$2"
  local artifact="$3"

  echo "[$gate_id] running..."
  if LOOP_MIN_AVAILABLE_MIB="$MIN_AVAILABLE_MIB" "$SAFE_RUN" bash -lc "$shell_cmd" >"$artifact" 2>&1; then
    echo "[$gate_id] pass -> $artifact"
    return 0
  fi

  echo "[$gate_id] fail -> $artifact"
  return 1
}

REQUIRED_FAILURE=0
VG_RCC_STATUS="fail"
VG_LA_STATUS="fail"
VG_RFLX_STATUS="fail"
VG_LA_FULL_STATUS="not_run"

if run_gate \
  "VG-RCC-001" \
  "cd '$RCC_DIR' && $RCC_PYTEST_CMD -q tests/unit/test_memory_store.py tests/unit/test_complexity_classifier.py tests/unit/test_smart_router.py" \
  "$EVIDENCE_DIR/M4-T04-VG-RCC-001.txt"; then
  VG_RCC_STATUS="pass"
else
  REQUIRED_FAILURE=1
fi

if run_gate \
  "VG-LA-001" \
  "cd '$LOOP_AGENT_DIR' && ${LA_ENV_PREFIX}$LA_PYTEST_CMD -q tests/test_router.py tests/test_trajectory.py tests/test_set_backend_propagation.py tests/test_sensitivity_wiring.py" \
  "$EVIDENCE_DIR/M4-T04-VG-LA-001.txt"; then
  VG_LA_STATUS="pass"
else
  REQUIRED_FAILURE=1
fi

if [[ "$RUN_LA_FULL_SNAPSHOT" == "1" ]]; then
  echo "[VG-LA-002] running advisory full-suite snapshot..."
  if LOOP_MIN_AVAILABLE_MIB="$MIN_AVAILABLE_MIB" "$SAFE_RUN" bash -lc "cd '$LOOP_AGENT_DIR' && ${LA_ENV_PREFIX}$LA_PYTEST_CMD -q" >"$EVIDENCE_DIR/M4-T04-VG-LA-002.txt" 2>&1; then
    echo "[VG-LA-002] advisory snapshot is green"
    VG_LA_FULL_STATUS="pass"
  else
    echo "[VG-LA-002] advisory snapshot has failures (expected on active branch)"
    VG_LA_FULL_STATUS="fail"
  fi
fi

if run_gate \
  "VG-RFLX-001" \
  "cd '$IO_RFLX_DIR' && CARGO_TARGET_DIR='$RFLX_CARGO_TARGET_DIR' cargo check -p rflx-core" \
  "$EVIDENCE_DIR/M4-T04-VG-RFLX-001.txt"; then
  VG_RFLX_STATUS="pass"
else
  REQUIRED_FAILURE=1
fi

PIPELINE_RESULT="pass"
if [[ "$REQUIRED_FAILURE" -ne 0 ]]; then
  PIPELINE_RESULT="fail"
fi

cat >"$SUMMARY_MD" <<EOF
# M4-T04 Pipeline Summary
Date: $(date -u '+%Y-%m-%d')
Result: $PIPELINE_RESULT
Pipeline: \`scripts/run_m4_compat_pipeline.sh\`
LOOP_MIN_AVAILABLE_MIB: \`$MIN_AVAILABLE_MIB\`
PIPELINE_STRICT: \`$PIPELINE_STRICT\`

## Required Gates

- \`VG-RCC-001\`: $VG_RCC_STATUS (\`M4-T04-VG-RCC-001.txt\`)
- \`VG-LA-001\`: $VG_LA_STATUS (\`M4-T04-VG-LA-001.txt\`)
- \`VG-RFLX-001\`: $VG_RFLX_STATUS (\`M4-T04-VG-RFLX-001.txt\`)

## Optional Advisory Gates

- \`VG-LA-002\`: $VG_LA_FULL_STATUS (enabled when \`RUN_LA_FULL_SNAPSHOT=1\`)
EOF

echo
if [[ "$PIPELINE_RESULT" == "pass" ]]; then
  echo "M4 compatibility pipeline finished successfully"
else
  echo "M4 compatibility pipeline completed with required gate failures"
fi
echo "summary: $SUMMARY_MD"

if [[ "$PIPELINE_STRICT" == "1" && "$PIPELINE_RESULT" != "pass" ]]; then
  exit 1
fi
