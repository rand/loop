# Validation Matrix

This matrix defines mandatory validation gates for milestone completion.

## Usage Rules

- Every completed task must cite passed VG IDs.
- Save command output under `docs/execution-plan/evidence/<date>/`.
- If a gate cannot run, record explicit blocker with reason and owner.
- In safe mode, run all heavy commands through `/Users/rand/src/loop/scripts/safe_run.sh`.
- Use `LOOP_MIN_AVAILABLE_MIB=3072` unless explicitly lowered with rationale.
- For `VG-RCC-001`, also record loop candidate SHA and `vendor/loop` submodule SHA in contract evidence.
- For `VG-LA-002`, always attach a failure summary artifact when the full suite is non-green.
- For `VG-RFLX-001`, contract evidence must cite the active interop schema version.
- For M6 governance tasks, `VG-CONTRACT-001` evidence must cite the compatibility tuple(s) and support tier transitions being claimed.
- For M6 rollout tasks, `VG-CONTRACT-001` evidence must cite release class selection, no-go criteria, and rollback tuple procedure references.
- For M6 cadence tasks, `VG-CONTRACT-001` evidence must cite explicit ownership assignments and recurring schedule definitions.
- For `VG-LA-002` promotion claims, evidence must be tied to committed consumer SHA state (D-015), not a dirty working tree.

## Core Loop Gates

| VG ID | Scope | Command | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-LOOP-BUILD-001 | `rlm-core` baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check'` | Exit code 0 | `.../VG-LOOP-BUILD-001.txt` |
| VG-LOOP-BUILD-002 | `rlm-core` feature baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check --no-default-features'` | Exit code 0 | `.../VG-LOOP-BUILD-002.txt` |
| VG-LOOP-BUILD-003 | `rlm-core` gemini profile | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check --no-default-features --features gemini'` | Exit code 0 | `.../VG-LOOP-BUILD-003.txt` |
| VG-LOOP-SIG-001 | Signature subsystem | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini signature::'` | All tests pass | `.../VG-LOOP-SIG-001.txt` |
| VG-LOOP-REPL-001 | Python REPL unit tests | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q'` | All tests pass | `.../VG-LOOP-REPL-001.txt` |
| VG-LOOP-REPL-002 | Rust ignored REPL spawn integration | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_repl_spawn -- --ignored'` | Test passes | `.../VG-LOOP-REPL-002.txt` |
| VG-LOOP-CORE-001 | Full `rlm-core` regression | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini'` | No failing tests | `.../VG-LOOP-CORE-001.txt` |

## Spec and Contract Gates

| VG ID | Scope | Command / Method | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-DOC-SPEC-001 | SPEC-20/26/27 alignment | Manual checklist against runtime interfaces | No unresolved spec/runtime drift items | `.../VG-DOC-SPEC-001.md` |
| VG-CONTRACT-001 | Consumer contract consistency | Review `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md` against current implementations | No unresolved contract ambiguities for active milestone | `.../VG-CONTRACT-001.md` |

## Cross-Repo Consumer Gates

| VG ID | Scope | Command | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-RCC-001 | `rlm-claude-code` critical unit set | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/rlm-claude-code && uv run pytest -q tests/unit/test_memory_store.py tests/unit/test_complexity_classifier.py tests/unit/test_smart_router.py'` | All tests pass; evidence records loop candidate SHA + `vendor/loop` SHA | `.../VG-RCC-001.txt` |
| VG-LA-001 | `loop-agent` seam-critical compatibility subset | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd <loop_agent_tuple_dir> && <pytest_cmd> -q tests/test_router.py tests/test_trajectory.py tests/test_set_backend_propagation.py tests/test_sensitivity_wiring.py'` | All seam-critical tests pass; claim-grade evidence must run from clean-clone committed tuple mode (D-017) | `.../VG-LA-001.txt` |
| VG-LA-002 | `loop-agent` full-suite health snapshot (advisory unless promoted by D-014) | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd <loop_agent_tuple_dir> && <pytest_cmd> -q'` | Snapshot captured and failures triaged in evidence; promotion to blocking requires D-014 criteria; claim-grade evidence must run from clean-clone committed tuple mode (D-017) | `.../VG-LA-002.txt` |
| VG-RFLX-001 | `io-rflx` core compile baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/io-rflx && cargo check -p rflx-core'` | Exit code 0; contract evidence cites interop schema version | `.../VG-RFLX-001.txt` |

## Performance and Efficacy Gates

| VG ID | Scope | Method | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-PERF-001 | REPL startup + execute latency | `LOOP_MIN_AVAILABLE_MIB=4096 /Users/rand/src/loop/scripts/run_m5_perf_harness.sh` | `M5-T01-VG-PERF-001.json` contains `"pass": true` and all latency regressions <= budget | `.../M5-T01-VG-PERF-001.json` |
| VG-PERF-002 | Synthetic batched operation throughput | `LOOP_MIN_AVAILABLE_MIB=4096 /Users/rand/src/loop/scripts/run_m5_perf_harness.sh` | `M5-T01-VG-PERF-002.json` contains `"pass": true` with throughput regression <= budget and error-rate delta <= 0.01 | `.../M5-T01-VG-PERF-002.json` |
| VG-EFFICACY-001 | Typed-SUBMIT correctness | Structured scenario suite from M2 | 100% pass on required validation scenarios | `.../VG-EFFICACY-001.md` |

## Minimum Gate Sets by Milestone

| Milestone | Required VG IDs |
|---|---|
| M0 | VG-CONTRACT-001 |
| M1 | VG-LOOP-BUILD-001, VG-LOOP-BUILD-002, VG-LOOP-BUILD-003, VG-LOOP-REPL-002, VG-LOOP-CORE-001 |
| M2 | VG-LOOP-SIG-001, VG-LOOP-REPL-001, VG-LOOP-REPL-002, VG-EFFICACY-001 |
| M3 | VG-DOC-SPEC-001, VG-CONTRACT-001 |
| M4 | VG-RCC-001, VG-LA-001, VG-RFLX-001, VG-CONTRACT-001 |
| M5 | VG-PERF-001, VG-PERF-002, VG-EFFICACY-001 |
| M6 | VG-CONTRACT-001 plus applicable prior consumer gates for any tuple newly claimed as supported |
