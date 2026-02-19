#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/Users/rand/src/loop"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
PY_REPL_DIR="$ROOT_DIR/rlm-core/python"
HARNESS="$ROOT_DIR/scripts/perf/repl_perf_harness.py"
COMPARE="$ROOT_DIR/scripts/perf/compare_perf_runs.py"

EVIDENCE_DATE="${EVIDENCE_DATE:-$(date +%F)}"
EVIDENCE_DIR_DEFAULT="$ROOT_DIR/docs/execution-plan/evidence/$EVIDENCE_DATE/milestone-M5"
EVIDENCE_DIR="${EVIDENCE_DIR:-$EVIDENCE_DIR_DEFAULT}"

MIN_AVAILABLE_MIB="${LOOP_MIN_AVAILABLE_MIB:-4096}"
STARTUP_ITERS="${STARTUP_ITERS:-15}"
EXEC_ITERS="${EXEC_ITERS:-80}"
SUBMIT_ITERS="${SUBMIT_ITERS:-80}"
BATCH_ITERS="${BATCH_ITERS:-80}"
BATCH_SIZE="${BATCH_SIZE:-8}"
BUDGET_PCT="${BUDGET_PCT:-10}"

mkdir -p "$EVIDENCE_DIR"

LOG_FILE="$EVIDENCE_DIR/M5-T01-harness-run.log"
: > "$LOG_FILE"

log() {
  echo "$*" | tee -a "$LOG_FILE"
}

BASELINE_JSON="$EVIDENCE_DIR/M5-T01-baseline.json"
CANDIDATE_JSON="$EVIDENCE_DIR/M5-T01-candidate.json"
VG_PERF_001="$EVIDENCE_DIR/M5-T01-VG-PERF-001.json"
VG_PERF_002="$EVIDENCE_DIR/M5-T01-VG-PERF-002.json"
SUMMARY_MD="$EVIDENCE_DIR/M5-T01-perf-summary.md"

run_harness() {
  local label="$1"
  local output="$2"
  log "[harness:$label] running..."
  LOOP_MIN_AVAILABLE_MIB="$MIN_AVAILABLE_MIB" "$SAFE_RUN" bash -lc \
    "cd '$PY_REPL_DIR' && uv run python '$HARNESS' \
      --label '$label' \
      --output '$output' \
      --startup-iters '$STARTUP_ITERS' \
      --exec-iters '$EXEC_ITERS' \
      --submit-iters '$SUBMIT_ITERS' \
      --batch-iters '$BATCH_ITERS' \
      --batch-size '$BATCH_SIZE'"
  log "[harness:$label] wrote $output"
}

log "M5 perf harness run starting"
log "date: $(date -u '+UTC %Y-%m-%d %H:%M:%S')"
log "evidence_dir: $EVIDENCE_DIR"
log "loop_min_available_mib: $MIN_AVAILABLE_MIB"
log "budget_pct: $BUDGET_PCT"
log ""

run_harness "baseline" "$BASELINE_JSON"
run_harness "candidate" "$CANDIDATE_JSON"

python3 "$COMPARE" \
  --baseline "$BASELINE_JSON" \
  --candidate "$CANDIDATE_JSON" \
  --vg-perf-001-out "$VG_PERF_001" \
  --vg-perf-002-out "$VG_PERF_002" \
  --summary-out "$SUMMARY_MD" \
  --budget-pct "$BUDGET_PCT"

log ""
log "M5 perf harness run completed"
log "summary: $SUMMARY_MD"
