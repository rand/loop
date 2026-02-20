# M7-T03 VG-EFFICACY-001
Date: 2026-02-19
Task: M7-T03 SPEC-20 typed-signature parity completion

## Scope
Validate typed-signature parity behavior changes that affect deterministic runtime efficacy:
- derive-macro enum field semantics via `#[field(enum_values = "...")]`
- pre-execution input validation in `Predict::forward`
- optional-null handling in signature validation

## Commands
1. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini predict::'`
2. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && UV_CACHE_DIR=/tmp/uv-cache uv run pytest -q tests/test_repl.py -k submit'`

## Result
- Pass
- Predict suite includes deterministic pre-exec validation check: `test_predict_forward_input_validation_happens_pre_exec`
- Submit scenarios unchanged and green: `6 passed, 40 deselected`

## Artifacts
- `M7-T03-VG-LOOP-SIG-002.txt`
- `M7-T03-submit-scenarios.txt`
