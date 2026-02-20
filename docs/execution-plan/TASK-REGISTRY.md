# Task Registry

Single source of truth for execution tasks and dependencies.

## Status Legend

- `todo`: not started
- `in_progress`: active
- `blocked`: blocked by dependency or decision
- `done`: complete with validation evidence

## Lane Assignment

- Lane A: `M7-T01`..`M7-T08` core runtime/spec closure (complete)
- Lane B: M7 spec/governance reconciliation (`M7-T10`) and documentation-only maintenance
- Lane C: consumer cadence + `M7-T09` interop fixture/calibration track

## Priority Queue (Execution Order)

| Rank | Task ID | Status | Why Next |
|---|---|---|---|
| 1 | M7-T01 (`loop-bih.1`) | done | Closed G-001 with Rust host `llm_batch` resolver wiring + refreshed gate evidence |
| 2 | M7-T02 (`loop-bih.2`) | done | SPEC-27 fallback loop wiring landed with deterministic trigger tests and gate artifacts |
| 3 | M7-T03 (`loop-bih.3`) | done | Added enum field semantics + deterministic pre-exec input validation parity with refreshed signature/predict/efficacy evidence |
| 4 | M7-T04 (`loop-bih.4`) | done | Connected dual-model routing to orchestrator mode boundaries and added root/recursive/extraction tier accounting with passing dual/core/perf gates |
| 5 | M7-T05 (`loop-bih.5`) | done | Replaced proof-engine Tier-3 placeholders with executable tactic synthesis and memory-backed persistence/context coverage |
| 6 | M7-T06 (`loop-bih.6`) | done | Added enhanced Mermaid export + TUI/MCP visualization endpoints with passing visualization/doc gates |
| 7 | M7-T07 (`loop-bih.7`) | done | Added reasoning capture + save/load persistence helpers with passing optimizer/efficacy/perf gates |
| 8 | M7-T08 (`loop-bih.8`) | done | Closed prompt/helper contract drift with submit semantics and helper-surface parity evidence (`M7-T08-validation-summary.md`) |
| 9 | M7-T09 (`loop-bih.9`) | done | Delivered loop-owned io-rflx fixture corpus + calibration gate and captured RFLX/contract/perf evidence (`M7-T09-validation-summary.md`) |
| 10 | M7-T10 (`loop-bih.10`) | todo | Reconciles spec status/governance after implementation closure |
| 11 | Ops-Weekly | in_progress | Continue steady-state compatibility cadence while M7 executes |

## M0 Tasks (Foundation and Contracts)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M0-T01 Baseline evidence snapshot | done | none | VG-CONTRACT-001 | Baseline findings captured in status + evidence index |
| M0-T02 Approve compatibility and breaking-change policy | done | M0-T01 | VG-CONTRACT-001 | Accepted decisions D-001, D-002 |
| M0-T03 Approve REPL entrypoint strategy | done | M0-T02 | VG-CONTRACT-001 | Accepted decision D-003 |
| M0-T04 Approve batched API naming/deprecation policy | done | M0-T02 | VG-CONTRACT-001 | Accepted decision D-004 |
| M0-T05 Approve cross-repo integration scope | done | M0-T02 | VG-CONTRACT-001 | Accepted decision D-006 |

## M1 Tasks (Build and Toolchain Integrity)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M1-T01 Fix `rlm_available_features()` compile failure | done | M0-T02 | VG-LOOP-BUILD-001, VG-LOOP-BUILD-002, VG-LOOP-BUILD-003 | Stable feature-list construction across cfgs |
| M1-T02 Add feature-matrix CI checks | done | M1-T01 | VG-LOOP-BUILD-001, VG-LOOP-BUILD-002, VG-LOOP-BUILD-003 | CI job matrix for baseline profiles |
| M1-T03 Make REPL spawn entrypoint robust | done | M0-T03 | VG-LOOP-REPL-002 | `ReplHandle::spawn` works in dev and packaged modes |
| M1-T04 Strengthen REPL spawn diagnostics and docs | done | M1-T03 | VG-LOOP-REPL-002 | Actionable errors for missing Python package path |
| M1-T05 Triage and stabilize failing `rlm-core` regression tests | done | M1-T01, M1-T03 | VG-LOOP-CORE-001 | Full gemini-profile test suite passes consistently |

## M2 Tasks (REPL Protocol + Typed Signatures)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M2-T01 Add JSON-RPC handlers for `register_signature` and `clear_signature` | done | M1-T03 | VG-LOOP-REPL-001, VG-LOOP-REPL-002 | Python server method coverage for Rust client contract |
| M2-T02 Implement sandbox `SUBMIT(outputs)` plumbing and submission state | done | M2-T01 | VG-EFFICACY-001, VG-LOOP-REPL-001 | Structured submit capture and termination semantics |
| M2-T03 Return `submit_result` in execute responses | done | M2-T02 | VG-EFFICACY-001, VG-LOOP-SIG-001 | End-to-end `ExecuteResult.submit_result` correctness |
| M2-T04 Add Rust/Python integration tests for typed SUBMIT scenarios | done | M2-T03 | VG-LOOP-REPL-002, VG-EFFICACY-001 | Scenario suite for success, validation error, no signature, multiple submits |
| M2-T05 Document runtime protocol contract in code and docs | done | M2-T04 | VG-DOC-SPEC-001 | Protocol docs aligned with implementation |

## M3 Tasks (SPEC/API Drift Closure)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M3-T01 Resolve `llm_query_batched` vs `llm_batch` policy and implementation | done | M0-T04, M2-T05 | VG-DOC-SPEC-001 | Canonical API + compatibility alias |
| M3-T02 Fix SPEC-20 file locations and runtime references | done | M2-T05 | VG-DOC-SPEC-001 | Accurate file map and test plan references |
| M3-T03 Reconcile SPEC-26/27 acceptance criteria with current runtime architecture | done | M3-T01 | VG-DOC-SPEC-001 | Explicit implemented vs planned markers |
| M3-T04 Create spec-to-test traceability table for M1-M3 | done | M3-T03 | VG-DOC-SPEC-001 | Traceability artifact under evidence/docs |

## M4 Tasks (Consumer Integration)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M4-T01 `rlm-claude-code` vendored sync and compatibility verification | done | M1-T04, M3-T04 | VG-RCC-001, VG-CONTRACT-001 | Updated submodule strategy and passing critical tests |
| M4-T02 Define first runtime integration contract for `loop-agent` | done | M3-T04 | VG-LA-001, VG-CONTRACT-001 | Signed contract section + minimal integration harness plan |
| M4-T03 Define interoperability contract for `io-rflx` trajectory/provenance/verification | done | M3-T04 | VG-RFLX-001, VG-CONTRACT-001 | Contract document + validation hooks |
| M4-T04 Add cross-repo compatibility CI recipe (manual or automated) | done | M4-T01, M4-T02, M4-T03 | VG-RCC-001, VG-LA-001, VG-RFLX-001 | Reproducible compatibility pipeline |

## M5 Tasks (Performance and Efficacy)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M5-T01 Define performance benchmark harness for REPL startup and batch execution | done | M2-T04 | VG-PERF-001, VG-PERF-002 | Benchmark scripts and baseline metrics |
| M5-T02 Define efficacy suite for typed-SUBMIT/fallback behaviors | done | M2-T04, M3-T04 | VG-EFFICACY-001 | Deterministic scenario matrix with expected outcomes |
| M5-T03 Run comparative baseline vs current candidate metrics | done | M5-T01, M5-T02 | VG-PERF-001, VG-PERF-002, VG-EFFICACY-001 | Performance and efficacy report |

## M6 Tasks (Rollout and Governance)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M6-T01 Publish compatibility matrix and support policy | done | M4-T04 | VG-CONTRACT-001 | Version/support table for loop and consumers |
| M6-T02 Define release/rollback playbook for integration changes | done | M6-T01 | VG-CONTRACT-001 | Rollout checklist with rollback triggers |
| M6-T03 Establish steady-state maintenance cadence and ownership | done | M6-T02 | VG-CONTRACT-001 | Governance section in plan docs |

## M7 Tasks (Spec Completion and Integration Hardening)

| Task ID | Status | Depends On | Required Gates | Deliverable |
|---|---|---|---|---|
| M7-T01 SPEC-26 `LLM_BATCH` end-to-end runtime closure (`loop-bih.1`) | done | M6-T03 | VG-LOOP-BATCH-001, VG-LOOP-REPL-001, VG-EFFICACY-001 | Rust-host + Python REPL batched path integration with deterministic tests |
| M7-T02 SPEC-27 orchestrator fallback wiring (`loop-bih.2`) | done | M7-T01 | VG-LOOP-FALLBACK-001, VG-EFFICACY-001, VG-LOOP-SIG-001 | Fallback extraction triggered and validated in orchestrator runtime loop |
| M7-T03 SPEC-20 typed-signature parity completion (`loop-bih.3`) | done | M7-T02 | VG-LOOP-SIG-002, VG-LOOP-SIG-001, VG-EFFICACY-001 | Enum/input-validation parity and deterministic pre-exec errors |
| M7-T04 SPEC-21 dual-model orchestration integration (`loop-bih.4`) | done | M7-T03 | VG-LOOP-DUAL-001, VG-LOOP-CORE-001, VG-PERF-002 | Orchestrator-applied dual-model routing with tiered accounting |
| M7-T05 SPEC-22 proof protocol execution closure (`loop-bih.5`) | done | M7-T04 | VG-LOOP-PROOF-001, VG-EFFICACY-001 | Proof engine placeholder removal and persistence coverage |
| M7-T06 SPEC-23 graph visualization integration closure (`loop-bih.6`) | done | M7-T05 | VG-LOOP-VIZ-001, VG-DOC-SPEC-002 | Visualization export/integration parity with explicit fixtures |
| M7-T07 SPEC-24 bootstrap optimizer parity closure (`loop-bih.7`) | done | M7-T06 | VG-LOOP-OPT-001, VG-EFFICACY-001, VG-PERF-003 | Reasoning capture + persistence + metric alignment |
| M7-T08 SPEC-25 context externalization contract closure (`loop-bih.8`) | done | M7-T07 | VG-LOOP-CONTEXT-001, VG-LOOP-REPL-001, VG-DOC-SPEC-002 | Runtime prompt/guide behavior aligned with SPEC-25 |
| M7-T09 `io-rflx` adapter fixtures + calibration delivery (`loop-bih.9`) | done | M7-T08 | VG-RFLX-001, VG-RFLX-002, VG-PERF-003, VG-CONTRACT-001 | Roundtrip fixture set and calibration policy/evidence |
| M7-T10 spec/governance reconciliation and promotion (`loop-bih.10`) | todo | M7-T09 | VG-DOC-SPEC-002, VG-CONTRACT-001, VG-RCC-001, VG-LA-001, VG-RFLX-001 | SPEC-20..27 status reconciliation with refreshed traceability and support claims |
