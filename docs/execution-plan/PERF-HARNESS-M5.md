# M5 Performance Harness

Repeatable REPL performance harness for `VG-PERF-001` and `VG-PERF-002`.

## Script

- Entry point: `scripts/run_m5_perf_harness.sh`
- Inner harness: `scripts/perf/repl_perf_harness.py`
- Comparison + gate output: `scripts/perf/compare_perf_runs.py`

## Default Run

```bash
cd /Users/rand/src/loop
LOOP_MIN_AVAILABLE_MIB=4096 EVIDENCE_DATE=2026-02-19 scripts/run_m5_perf_harness.sh
```

## Tunable Parameters

- `STARTUP_ITERS` (default `15`)
- `EXEC_ITERS` (default `80`)
- `SUBMIT_ITERS` (default `80`)
- `BATCH_ITERS` (default `80`)
- `BATCH_SIZE` (default `8`)
- `BUDGET_PCT` (default `10`)

## Outputs

- `M5-T01-baseline.json`
- `M5-T01-candidate.json`
- `M5-T01-VG-PERF-001.json`
- `M5-T01-VG-PERF-002.json`
- `M5-T01-perf-summary.md`
- `M5-T01-harness-run.log`
