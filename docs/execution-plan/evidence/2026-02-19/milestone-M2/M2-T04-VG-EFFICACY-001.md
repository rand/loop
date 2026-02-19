# M2-T04 Efficacy Scenario Matrix
Date: 2026-02-19
Task IDs: M2-T04
VG IDs: VG-EFFICACY-001
Command(s):
- `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q tests/test_repl.py'`
- `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini submit_result_roundtrip -- --ignored'`
Result: pass
Notes: Scenario suite now covers all required typed-submit cases across Python and Rust boundaries.

## Required Scenarios

- [x] Registered signature + valid submit.
- [x] Registered signature + missing field.
- [x] Registered signature + type mismatch.
- [x] No registered signature.
- [x] Multiple SUBMIT calls in one execution.

## Evidence Sources

- `M2-T04-VG-EFFICACY-001-python.txt`
- `M2-T04-submit-roundtrip-scenarios.txt`
