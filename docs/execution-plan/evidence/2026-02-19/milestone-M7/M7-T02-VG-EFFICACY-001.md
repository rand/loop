# M7-T02 VG-EFFICACY-001
Date: 2026-02-19
Task: M7-T02 SPEC-27 orchestrator fallback wiring

## Scope
Validate efficacy-sensitive submit/fallback behavior after introducing orchestrator-side fallback loop wiring.

## Commands
1. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && UV_CACHE_DIR=/tmp/uv-cache uv run pytest -q tests/test_repl.py -k submit'`
2. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini fallback::'`

## Result
- Pass
- Python submit scenarios: `6 passed, 39 deselected`
- Rust fallback loop + extractor scenarios: `18 passed`

## Artifacts
- `M7-T02-submit-scenarios.txt`
- `M7-T02-VG-LOOP-FALLBACK-001.txt`
