#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/Users/rand/src/loop"
SAFE_RUN="$ROOT_DIR/scripts/safe_run.sh"
PY_REPL_DIR="$ROOT_DIR/rlm-core/python"
UV_CACHE_DIR_VALUE="${UV_CACHE_DIR:-$ROOT_DIR/.uv-cache}"
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
RUN_REPEATS="${RUN_REPEATS:-3}"
BUDGET_PCT="${BUDGET_PCT:-10}"
MIN_ABS_LATENCY_MS="${MIN_ABS_LATENCY_MS:-2.0}"
MIN_ABS_THROUGHPUT_DROP_OPS="${MIN_ABS_THROUGHPUT_DROP_OPS:-150.0}"
MAX_ERROR_RATE_DELTA="${MAX_ERROR_RATE_DELTA:-0.01}"
BASELINE_JSON_IN="${BASELINE_JSON_IN:-}"
ALLOW_SAME_COMMIT="${ALLOW_SAME_COMMIT:-0}"

mkdir -p "$EVIDENCE_DIR"
mkdir -p "$UV_CACHE_DIR_VALUE"

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
    "cd '$PY_REPL_DIR' && UV_CACHE_DIR='$UV_CACHE_DIR_VALUE' uv run python '$HARNESS' \
      --label '$label' \
      --output '$output' \
      --startup-iters '$STARTUP_ITERS' \
      --exec-iters '$EXEC_ITERS' \
      --submit-iters '$SUBMIT_ITERS' \
      --batch-iters '$BATCH_ITERS' \
      --batch-size '$BATCH_SIZE'"
  log "[harness:$label] wrote $output"
}

run_harness_repeated() {
  local label="$1"
  local output="$2"
  local run=1
  local run_outputs=()

  while [[ "$run" -le "$RUN_REPEATS" ]]; do
    local run_output="$output"
    if [[ "$run" -gt 1 ]]; then
      run_output="${output%.json}.run${run}.json"
    fi
    run_harness "${label}-run${run}" "$run_output" 1>&2
    run_outputs+=("$run_output")
    run=$((run + 1))
  done

  local joined
  joined="$(IFS=,; echo "${run_outputs[*]}")"
  echo "$joined"
}

validate_baseline_inputs() {
  local csv="$1"
  IFS=',' read -r -a files <<< "$csv"
  for file in "${files[@]}"; do
    if [[ ! -f "$file" ]]; then
      echo "missing baseline input file: $file" >&2
      return 1
    fi
  done
}

log "M5 perf harness run starting"
log "date: $(date -u '+UTC %Y-%m-%d %H:%M:%S')"
log "evidence_dir: $EVIDENCE_DIR"
log "loop_min_available_mib: $MIN_AVAILABLE_MIB"
log "run_repeats: $RUN_REPEATS"
log "budget_pct: $BUDGET_PCT"
log "min_abs_latency_ms: $MIN_ABS_LATENCY_MS"
log "min_abs_throughput_drop_ops: $MIN_ABS_THROUGHPUT_DROP_OPS"
log "max_error_rate_delta: $MAX_ERROR_RATE_DELTA"
log "allow_same_commit: $ALLOW_SAME_COMMIT"
log ""

if [[ -n "$BASELINE_JSON_IN" ]]; then
  validate_baseline_inputs "$BASELINE_JSON_IN"
  BASELINE_INPUTS="$BASELINE_JSON_IN"
  log "baseline input mode: external file(s)"
  log "baseline_inputs: $BASELINE_INPUTS"
else
  if [[ "$ALLOW_SAME_COMMIT" != "1" ]]; then
    echo "BASELINE_JSON_IN is required for distinct-tuple comparisons (or set ALLOW_SAME_COMMIT=1 for calibration)." >&2
    exit 1
  fi
  BASELINE_INPUTS="$(run_harness_repeated "baseline" "$BASELINE_JSON")"
  log "baseline input mode: generated in-place"
  log "baseline_inputs: $BASELINE_INPUTS"
fi

CANDIDATE_INPUTS="$(run_harness_repeated "candidate" "$CANDIDATE_JSON")"
log "candidate_inputs: $CANDIDATE_INPUTS"

COMPARE_ARGS=(
  --baseline "$BASELINE_INPUTS"
  --candidate "$CANDIDATE_INPUTS"
  --vg-perf-001-out "$VG_PERF_001"
  --vg-perf-002-out "$VG_PERF_002"
  --summary-out "$SUMMARY_MD"
  --budget-pct "$BUDGET_PCT"
  --min-abs-latency-ms "$MIN_ABS_LATENCY_MS"
  --min-abs-throughput-drop-ops "$MIN_ABS_THROUGHPUT_DROP_OPS"
  --max-error-rate-delta "$MAX_ERROR_RATE_DELTA"
)
if [[ "$ALLOW_SAME_COMMIT" == "1" ]]; then
  COMPARE_ARGS+=(--allow-same-commit)
fi

python3 "$COMPARE" "${COMPARE_ARGS[@]}"

log ""
log "M5 perf harness run completed"
log "summary: $SUMMARY_MD"
