# M7-T01 VG-EFFICACY-001
Date: 2026-02-19
Task: M7-T01 SPEC-26 `LLM_BATCH` end-to-end runtime closure

## Scope
Validate typed submit and batched helper efficacy scenarios in the Python REPL runtime, plus host-side batch orchestration coverage.

## Commands
1. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && UV_CACHE_DIR=/tmp/uv-cache uv run pytest -q tests/test_repl.py -k submit'`
2. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && UV_CACHE_DIR=/tmp/uv-cache uv run pytest -q tests/test_repl.py -k llm_batch'`
3. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_llm_batch_host_resolution_roundtrip -- --ignored'`

Note: Threshold lowered from 3072 MiB to 3000 MiB after repeated safe-run admission blocks around 3004 MiB available memory.

## Result
- Pass
- Submit scenarios: `6 passed, 40 deselected`
- Batch helper scenarios: `2 passed, 44 deselected`
- Host roundtrip scenario (`ignored` integration): `1 passed`

## Artifacts
- `M7-T01-submit-scenarios.txt`
- `M7-T01-VG-LOOP-BATCH-001.txt`
- `M7-T01-VG-LOOP-REPL-001.txt`
- `M7-T01-test_llm_batch_host_resolution_roundtrip.txt`
