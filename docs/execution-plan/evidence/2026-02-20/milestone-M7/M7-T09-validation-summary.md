# M7-T09 Validation Summary
Date: 2026-02-20
Task: M7-T09 io-rflx adapter fixture + calibration delivery

## Required Gates
- VG-RFLX-001: pass
- VG-RFLX-002: pass
- VG-PERF-003: pass
- VG-CONTRACT-001: pass

## Key Results
- Added canonical `io_rflx_interop.v0` fixture corpus for provenance, trajectory, and verification envelopes.
- Added deterministic calibration policy (`confidence_bucket_v1`) and executable fixture validation script.
- Replaced non-informative `interop_fixture` filter gate with loop-owned fixture gate runner that also executes targeted io-rflx roundtrip serialization tests.
- Captured refreshed performance guardrail evidence and linked calibration review into `VG-PERF-003` output.

## Commands
1. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/io-rflx && CARGO_TARGET_DIR=/tmp/io-rflx-cargo-target cargo check -p rflx-core'`
2. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop && IO_RFLX_DIR=/Users/rand/src/io-rflx RFLX_CARGO_TARGET_DIR=/tmp/io-rflx-cargo-target ./scripts/run_rflx_interop_fixture_gate.sh'`
3. `UV_CACHE_DIR=/tmp/uv-cache LOOP_MIN_AVAILABLE_MIB=3000 EVIDENCE_DATE=2026-02-20 EVIDENCE_DIR=/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-20/milestone-M7 ./scripts/run_m5_perf_harness.sh`

## Artifacts
- `M7-T09-VG-RFLX-001.txt`
- `M7-T09-VG-RFLX-002.txt`
- `M7-T09-VG-PERF-003.txt`
- `M7-T09-VG-PERF-003.json`
- `M7-T09-VG-PERF-003-summary.md`
- `M7-T09-VG-CONTRACT-001.md`
