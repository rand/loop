# M7-T04 Validation Summary
Date: 2026-02-20
Task: M7-T04 SPEC-21 dual-model orchestrator integration

## Required Gates
- VG-LOOP-DUAL-001: pass
- VG-LOOP-CORE-001: pass
- VG-PERF-002: pass

## Key Results
- Dual-model routing now supports explicit extraction-tier routing and remains strategy-aware for root/recursive decisions.
- Tiered accounting now tracks root, recursive, and extraction calls in `CostTracker::record_tiered` + `TierBreakdown`.
- Orchestrator mode boundaries now apply dual-model defaults via `ExecutionMode::default_dual_model_config` and `OrchestrationRoutingRuntime`.
- Full `rlm-core` regression suite remains green after integration changes.

## Commands
1. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini route_rlm'`
2. `LOOP_MIN_AVAILABLE_MIB=3000 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini'`
3. `UV_CACHE_DIR=/tmp/uv-cache LOOP_MIN_AVAILABLE_MIB=3000 EVIDENCE_DATE=2026-02-20 EVIDENCE_DIR=/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-20/milestone-M7 ./scripts/run_m5_perf_harness.sh`

## Artifacts
- `M7-T04-VG-LOOP-DUAL-001.txt`
- `M7-T04-VG-LOOP-CORE-001.txt`
- `M7-T04-VG-PERF-002.txt`
- `M7-T04-VG-PERF-002.json`
- `M7-T04-VG-PERF-002-summary.md`
