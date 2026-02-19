# M2-T03 Efficacy Scenario Matrix
Date: 2026-02-19
Task IDs: M2-T03
VG IDs: VG-EFFICACY-001
Command(s):
- `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q tests/test_repl.py'`
- `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini submit_result_roundtrip -- --ignored'`
Result: pass
Notes: Combined Python server scenarios with Rust roundtrip integration checks.

## Cross-Boundary Scenarios

- [x] Rust receives `SubmitResult::Success` for valid SUBMIT execution.
- [x] Rust receives `SubmitResult::ValidationError` for invalid SUBMIT payload.
- [x] Validation errors remain structured (`missing_field`, `type_mismatch`, `no_signature_registered`, `multiple_submits`).
