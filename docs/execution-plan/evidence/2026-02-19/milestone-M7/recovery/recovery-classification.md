# Recovery Classification
Date: 2026-02-19
Branch: codex/recovery-crash-integration

## Purpose
Classify crash-session residue into actionable integration workstreams and cleanup buckets before editing or dropping files.

## Tracked Modifications

### Workstream A (likely real feature work, keep)
- docs/spec/SPEC-20-typed-signatures.md
- docs/spec/SPEC-26-batched-queries.md
- docs/spec/SPEC-27-fallback-extraction.md
- rlm-core/python/README.md
- rlm-core/python/rlm_repl/helpers.py
- rlm-core/python/rlm_repl/main.py
- rlm-core/python/rlm_repl/protocol.py
- rlm-core/python/rlm_repl/sandbox.py
- rlm-core/python/tests/test_repl.py
- rlm-core/src/repl.rs

### Workstream B (stabilization and regression fixes, keep pending validation)
- rlm-core/src/ffi/mod.rs
- rlm-core/src/llm/client.rs
- rlm-core/src/llm/batch.rs
- rlm-core/src/llm/router.rs
- rlm-core/src/module/optimize.rs
- rlm-core/src/proof/session.rs
- rlm-core/src/reasoning/visualize.rs

### Workstream C (behavior changes requiring explicit accept/reject)
- rlm-core/src/complexity.rs
- rlm-core/src/epistemic/scrubber.rs

## Untracked Files

### Keep candidates
- .github/workflows/rlm-core-feature-matrix.yml
- rlm-core/python/rlm_repl/__main__.py
- rlm-core/proptest-regressions/epistemic/proptest.txt
- rlm-core/uv.lock (policy decision pending)

### Drop as generated/noise
- .reasoning_logs/**
- scripts/perf/__pycache__/**
- .DS_Store
- docs/.DS_Store
- ${CLAUDE_PLUGIN_ROOT}/**

## Pending Decisions
1. Track or discard `rlm-core/uv.lock`.
2. Accept/revert Workstream C behavior changes after targeted validation.
