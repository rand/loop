# M2-T02 Efficacy Scenario Matrix
Date: 2026-02-19
Task IDs: M2-T02
VG IDs: VG-EFFICACY-001
Command(s): `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q tests/test_repl.py'`
Result: pass
Notes: SUBMIT path behavior validated through deterministic unit scenarios in `TestReplServer`.

## Scenario Coverage

- [x] SUBMIT without signature returns structured `no_signature_registered` validation error.
- [x] Registered signature + valid SUBMIT returns `status=success` with outputs.
- [x] Registered signature + missing field returns structured `missing_field` error.
- [x] Registered signature + type mismatch returns structured `type_mismatch` error.
- [x] Multiple SUBMIT calls in a single execution return structured `multiple_submits` error.

## Evidence Source

- `M2-T02-repl-submit-scenarios.txt`
