# VG-PERF-003 Summary
Date: 2026-02-20
Scope: Full-system validation rerun

## Inputs
- Harness metrics: `M5-T01-baseline.json`, `M5-T01-candidate.json`
- Comparator outputs: `M5-T01-VG-PERF-001.json`, `M5-T01-VG-PERF-002.json`
- io-rflx calibration/roundtrip gate: `VG-RFLX-002.txt`

## Result
- Pass (`VG-PERF-003`)
- Throughput regression: `-8.2764%` (candidate is faster)
- Error-rate delta: `0.0`
- No >10% regression observed on affected paths.

## Calibration Note
- `VG-RFLX-002` passed with schema version `io_rflx_interop.v0` and calibration method `confidence_bucket_v1`.
