# M5-T03 Comparative Analysis Report
Date: 2026-02-19
Scope: Baseline vs candidate performance/efficacy comparison

## Inputs

- Performance baseline: `M5-T01-baseline.json`
- Performance candidate: `M5-T01-candidate.json`
- Gate comparisons: `M5-T01-VG-PERF-001.json`, `M5-T01-VG-PERF-002.json`
- Efficacy suite: `M5-T02-VG-EFFICACY-001.txt`

## Methodology

- Harness: `scripts/run_m5_perf_harness.sh`
- Config: startup=15, execute=80, submit=80, batch=80, batch_size=8
- Budget: 10% regression threshold
- Environment: macOS arm64, Python 3.12.11, loop commit `50cd8cfe95f3179a4f15a445199fa9b1d1fe91f9`

## Performance Comparison

| Metric | Baseline | Candidate | Regression |
|---|---:|---:|---:|
| Startup P50 (ms) | 111.29 | 96.18 | -13.57% |
| Startup P95 (ms) | 153.73 | 103.38 | -32.75% |
| Execute no-submit P50 (ms) | 0.0594 | 0.0566 | -4.70% |
| Execute no-submit P95 (ms) | 0.1417 | 0.1276 | -9.92% |
| Execute submit P50 (ms) | 0.0665 | 0.0614 | -7.80% |
| Execute submit P95 (ms) | 0.1516 | 0.1143 | -24.58% |
| Batch throughput (ops/s) | 3964.76 | 5265.08 | +32.80% |
| Batch error rate | 0.00 | 0.00 | +0.00 |

`VG-PERF-001`: pass  
`VG-PERF-002`: pass

## Efficacy Result

- `VG-EFFICACY-001`: pass (`45 passed`, deterministic typed-signature/fallback scenario suite)

## Regression Assessment

- No metric exceeded the >10% regression threshold.
- No efficacy regression observed in scenario suite outcomes.

## Residual Risk and Mitigation

- Baseline and candidate runs were captured on the same commit (noise-control baseline), so this report confirms harness stability but is not a cross-commit regression study.
- For release-go/no-go comparisons, retain this baseline method and rerun candidate on the target commit with identical harness config.
