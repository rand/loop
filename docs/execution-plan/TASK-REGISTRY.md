# Task Registry

Single source of truth for execution tasks and dependencies.

## Status Legend

- `todo`: not started
- `in_progress`: active
- `blocked`: blocked by dependency or decision
- `done`: complete with validation evidence

## Lane Assignment

- Lane A: `M3` maintenance only (standby/read-only by default)
- Lane B: `M1`, `M2` maintenance only (standby/read-only by default)
- Lane C: `M4`, `M5`, `M6` (active heavy lane in safe mode)

## Priority Queue (Execution Order)

| Rank | Task ID | Status | Why Next |
|---|---|---|---|
| 1 | loop-5va | in_progress | Land committed candidate tuple into canonical loop-agent repo and refresh baseline evidence (D-017 requires clean-clone committed tuple mode until landing stabilizes) |

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
