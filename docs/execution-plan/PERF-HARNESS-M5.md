# M5 Performance Harness

Repeatable REPL performance harness for `VG-PERF-001` and `VG-PERF-002`.

## Script

- Entry point: `scripts/run_m5_perf_harness.sh`
- Inner harness: `scripts/perf/repl_perf_harness.py`
- Comparison + gate output: `scripts/perf/compare_perf_runs.py`

## Default Run

```bash
cd /Users/rand/src/loop
LOOP_MIN_AVAILABLE_MIB=4096 \
EVIDENCE_DATE=2026-02-20 \
BASELINE_JSON_IN=/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-19/milestone-M5/M5-T01-baseline.json \
scripts/run_m5_perf_harness.sh
```

Methodology defaults:
- Distinct-tuple comparison is required (baseline and candidate commits must not overlap).
- Candidate verdict uses median aggregation across repeated runs (`RUN_REPEATS`, default `3`).
- Regression checks apply both percentage budget and absolute floors to avoid noise-only failures.

## Tunable Parameters

- `STARTUP_ITERS` (default `15`)
- `EXEC_ITERS` (default `80`)
- `SUBMIT_ITERS` (default `80`)
- `BATCH_ITERS` (default `80`)
- `BATCH_SIZE` (default `8`)
- `RUN_REPEATS` (default `3`)
- `BUDGET_PCT` (default `10`)
- `MIN_ABS_LATENCY_MS` (default `2.0`)
- `MIN_ABS_THROUGHPUT_DROP_OPS` (default `150.0`)
- `MAX_ERROR_RATE_DELTA` (default `0.01`)
- `BASELINE_JSON_IN` (comma-separated baseline run file(s), required unless `ALLOW_SAME_COMMIT=1`)
- `ALLOW_SAME_COMMIT` (default `0`; set `1` only for noise-calibration runs, not release claims)

## Outputs

- `M5-T01-baseline.json`
- `M5-T01-baseline.runN.json` (for `RUN_REPEATS > 1`)
- `M5-T01-candidate.json`
- `M5-T01-candidate.runN.json` (for `RUN_REPEATS > 1`)
- `M5-T01-VG-PERF-001.json`
- `M5-T01-VG-PERF-002.json`
- `M5-T01-perf-summary.md`
- `M5-T01-harness-run.log`
