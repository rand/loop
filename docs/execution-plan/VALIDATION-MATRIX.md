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
- For M7 gates, evidence must map each gate result to a specific M7 task ID (`M7-T01`..`M7-T10`).
- For `VG-COVERAGE-001`, CI evidence from `.github/workflows/rlm-core-coverage.yml` is canonical when local environments cannot install `cargo-llvm-cov`.
- For `VG-PY-INTEGRATION-001`, all-skipped or no-tests-ran outcomes are gate failures.
- For `VG-PROPTEST-001`, run with deterministic seed/config (`PROPTEST_RNG_SEED`, `PROPTEST_CASES`, `PROPTEST_RNG_ALGORITHM`) and fail if any scoped suite executes zero tests.
- For `VG-CLAUDE-ADAPTER-E2E-001`, fail if fewer than two scenario tests execute (filter drift guardrail).

## Core Loop Gates

| VG ID | Scope | Command | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-LOOP-BUILD-001 | `rlm-core` baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check'` | Exit code 0 | `.../VG-LOOP-BUILD-001.txt` |
| VG-LOOP-BUILD-002 | `rlm-core` feature baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check --no-default-features'` | Exit code 0 | `.../VG-LOOP-BUILD-002.txt` |
| VG-LOOP-BUILD-003 | `rlm-core` gemini profile | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check --no-default-features --features gemini'` | Exit code 0 | `.../VG-LOOP-BUILD-003.txt` |
| VG-LOOP-SIG-001 | Signature subsystem | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini signature::'` | All tests pass | `.../VG-LOOP-SIG-001.txt` |
| VG-LOOP-REPL-001 | Python REPL unit tests | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q'` | All tests pass | `.../VG-LOOP-REPL-001.txt` |
| VG-PY-INTEGRATION-001 | Python package compatibility helpers | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/run_vg_py_integration_gate.sh` | Integration suite passes with >=1 passing test; all-skipped and no-tests-ran are failures | `.../VG-PY-INTEGRATION-001.txt` |
| VG-PROPTEST-001 | Property-based invariants (epistemic + signature validation + fallback + router/accounting) | `LOOP_MIN_AVAILABLE_MIB=3072 PROPTEST_CASES=96 PROPTEST_RNG_SEED=424242 PROPTEST_RNG_ALGORITHM=cc /Users/rand/src/loop/scripts/run_vg_proptest_gate.sh` | All scoped proptest runs pass with deterministic seed; each scope executes >=1 test (zero-test scopes fail) | `.../VG-PROPTEST-001.txt` |
| VG-CLAUDE-ADAPTER-E2E-001 | Claude adapter end-to-end OODA scenarios | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/run_vg_claude_adapter_e2e_gate.sh` | Scenario suite passes; >=2 scenario tests execute; assertions cover observe/orient/decide/act behavior | `.../VG-CLAUDE-ADAPTER-E2E-001.txt` |
| VG-LOOP-REPL-002 | Rust ignored REPL spawn integration | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_repl_spawn -- --ignored'` | Test passes | `.../VG-LOOP-REPL-002.txt` |
| VG-LOOP-IGNORED-REPL-001 | Unattended ignored subprocess-integration health (`rlm_repl` + Lean REPL spawn paths) | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_repl_spawn -- --ignored --test-threads=1 && cargo test --no-default-features --features gemini test_lean_repl_spawn -- --ignored --test-threads=1'` | Commands complete deterministically (expected runtime: usually < 120s total); no orphaned `rlm_repl`/Lean `repl` subprocesses remain; environment failures fail fast with actionable stderr and are triaged via troubleshooting checklist | `.../VG-LOOP-IGNORED-REPL-001.txt` |
| VG-LOOP-CORE-001 | Full `rlm-core` regression | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini'` | No failing tests | `.../VG-LOOP-CORE-001.txt` |
| VG-COVERAGE-001 | Reproducible line-coverage gate (`rlm-core`) | `LOOP_MIN_AVAILABLE_MIB=4096 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop && make coverage'` | Coverage run succeeds and line coverage is >= 70% (`COVERAGE_MIN_LINES`) | `.../VG-COVERAGE-001.txt` plus `coverage/lcov.info` and `coverage/summary.txt` |
| VG-LOOP-BATCH-001 | End-to-end `LLM_BATCH` runtime path (Rust host + Python REPL) | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop && (cd rlm-core && cargo test --no-default-features --features gemini test_llm_batch) && (cd rlm-core/python && uv run pytest -q tests/test_repl.py -k llm_batch)'` | Rust and Python targeted batch-path suites pass | `.../VG-LOOP-BATCH-001.txt` |
| VG-LOOP-FALLBACK-001 | Orchestrator fallback extraction runtime behavior | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini fallback::'` | Fallback trigger and extraction orchestration tests pass | `.../VG-LOOP-FALLBACK-001.txt` |
| VG-LOOP-SIG-002 | Typed-signature parity (enum and pre-exec input validation) | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini signature:: && cargo test --no-default-features --features gemini predict::'` | New typed-signature parity scenarios pass with deterministic errors | `.../VG-LOOP-SIG-002.txt` |
| VG-LOOP-DUAL-001 | Dual-model routing applied in orchestration paths | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini route_rlm'` | Routing decisions and tiered-cost accounting tests pass | `.../VG-LOOP-DUAL-001.txt` |
| VG-LOOP-PROOF-001 | Proof protocol execution and persistence behavior | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini proof::'` | Proof-engine integration tests pass with no placeholder-only paths | `.../VG-LOOP-PROOF-001.txt` |
| VG-LOOP-VIZ-001 | Graph visualization export/integration surfaces | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini visualize::'` | Visualization exports and integration fixtures pass | `.../VG-LOOP-VIZ-001.txt` |
| VG-LOOP-OPT-001 | Bootstrap optimizer parity and persistence | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini optimize::'` | Optimizer reasoning/persistence/metric tests pass | `.../VG-LOOP-OPT-001.txt` |
| VG-LOOP-CONTEXT-001 | Context externalization prompt contract and helpers | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini externalize::'` | Prompt contract and helper guidance tests pass | `.../VG-LOOP-CONTEXT-001.txt` |

## Spec and Contract Gates

| VG ID | Scope | Command / Method | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-DOC-SPEC-001 | SPEC-20/26/27 alignment | Manual checklist against runtime interfaces | No unresolved spec/runtime drift items | `.../VG-DOC-SPEC-001.md` |
| VG-DOC-SPEC-002 | SPEC-20..27 completion/status reconciliation | Manual checklist against implementation snapshots + M7 evidence map | Status/acceptance fields reflect runtime truth and unresolved gaps are explicitly tracked | `.../VG-DOC-SPEC-002.md` |
| VG-CONTRACT-001 | Consumer contract consistency | Review `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md` against current implementations | No unresolved contract ambiguities for active milestone | `.../VG-CONTRACT-001.md` |

## Cross-Repo Consumer Gates

| VG ID | Scope | Command | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-RCC-001 | `rlm-claude-code` critical unit set | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/rlm-claude-code && uv run pytest -q tests/unit/test_memory_store.py tests/unit/test_complexity_classifier.py tests/unit/test_smart_router.py'` | All tests pass; evidence records loop candidate SHA + `vendor/loop` SHA | `.../VG-RCC-001.txt` |
| VG-LA-001 | `loop-agent` seam-critical compatibility subset | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd <loop_agent_tuple_dir> && <pytest_cmd> -q tests/test_router.py tests/test_trajectory.py tests/test_set_backend_propagation.py tests/test_sensitivity_wiring.py'` | All seam-critical tests pass; claim-grade evidence must run from clean-clone committed tuple mode (D-017) | `.../VG-LA-001.txt` |
| VG-LA-002 | `loop-agent` full-suite health snapshot (advisory unless promoted by D-014) | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd <loop_agent_tuple_dir> && <pytest_cmd> -q'` | Snapshot captured and failures triaged in evidence; promotion to blocking requires D-014 criteria; claim-grade evidence must run from clean-clone committed tuple mode (D-017) | `.../VG-LA-002.txt` |
| VG-RFLX-001 | `io-rflx` core compile baseline | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/io-rflx && CARGO_TARGET_DIR=/tmp/io-rflx-cargo-target cargo check -p rflx-core'` | Exit code 0; contract evidence cites interop schema version | `.../VG-RFLX-001.txt` |
| VG-RFLX-002 | `io-rflx` interop fixture roundtrip and calibration checks | `LOOP_MIN_AVAILABLE_MIB=3072 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop && IO_RFLX_DIR=/Users/rand/src/io-rflx RFLX_CARGO_TARGET_DIR=/tmp/io-rflx-cargo-target ./scripts/run_rflx_interop_fixture_gate.sh'` | Fixture schema and calibration checks pass and targeted io-rflx roundtrip tests pass | `.../VG-RFLX-002.txt` |

## Performance and Efficacy Gates

| VG ID | Scope | Method | Pass Criteria | Evidence Artifact |
|---|---|---|---|---|
| VG-PERF-001 | REPL startup + execute latency | `LOOP_MIN_AVAILABLE_MIB=4096 BASELINE_JSON_IN=<baseline_run_or_csv> /Users/rand/src/loop/scripts/run_m5_perf_harness.sh` | `M5-T01-VG-PERF-001.json` contains `"pass": true`; baseline/candidate tuple commits are distinct (unless explicit `ALLOW_SAME_COMMIT=1` calibration mode); latency regressions respect percent budget + absolute floor | `.../M5-T01-VG-PERF-001.json` |
| VG-PERF-002 | Synthetic batched operation throughput | `LOOP_MIN_AVAILABLE_MIB=4096 BASELINE_JSON_IN=<baseline_run_or_csv> /Users/rand/src/loop/scripts/run_m5_perf_harness.sh` | `M5-T01-VG-PERF-002.json` contains `"pass": true`; throughput regressions respect percent budget + absolute floor; error-rate delta <= configured bound (`0.01` default) | `.../M5-T01-VG-PERF-002.json` |
| VG-PERF-003 | M7 comparative overhead guardrail (batch/fallback/interop calibration) | `LOOP_MIN_AVAILABLE_MIB=4096 /Users/rand/src/loop/scripts/run_m5_perf_harness.sh` plus `io-rflx` calibration artifact review | No new >10% regression vs M5 baselines on affected paths; calibration deltas documented | `.../VG-PERF-003.json` |
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
| M7 | VG-LOOP-BATCH-001, VG-LOOP-FALLBACK-001, VG-LOOP-SIG-002, VG-LOOP-DUAL-001, VG-LOOP-PROOF-001, VG-LOOP-VIZ-001, VG-LOOP-OPT-001, VG-LOOP-CONTEXT-001, VG-RFLX-002, VG-PERF-003, VG-DOC-SPEC-002, VG-CONTRACT-001 plus applicable consumer tuple gates (`VG-RCC-001`, `VG-LA-001`, `VG-RFLX-001`) |

Post-M7 steady-state release-quality checks add `VG-COVERAGE-001` to the gate chain before compatibility claim updates.
