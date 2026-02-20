# Consumer Integration Contract

This document defines how `loop` changes are evaluated against active consumers.

Primary support-policy source: `docs/execution-plan/COMPATIBILITY-MATRIX.md`.

## Scope

Consumers in scope:

- `rlm-claude-code` (hard runtime/build consumer)
- `loop-agent` (active integration target)
- `io-rflx` (active integration target)

## Contract Levels

- Level 1: Build/import compatibility.
- Level 2: API and data-model compatibility.
- Level 3: Behavioral compatibility under representative tests.
- Level 4: Performance and efficacy non-regression.

## Consumer A: `rlm-claude-code`

Current coupling facts:

- Maturin build is pinned to `vendor/loop/rlm-core/Cargo.toml`.
- Runtime directly imports and uses `rlm_core` in memory, classifier, router, trajectory, and epistemic extraction paths.
- Unit tests depend on specific behavior (including storage semantics and enum mappings).

Required contract invariants:

- Invariant A1: Python import path and module name remain usable as `rlm_core`.
- Invariant A2: Core enums and constructors used by `src/trajectory.py` remain compatible or get explicit migration shims.
- Invariant A3: Memory store behavior used by `src/memory_store.py` remains compatible, especially around SQLite/WAL assumptions.
- Invariant A4: Pattern classifier and smart router delegation paths continue to function.
- Invariant A5: REPL batched-query helper remains available as `llm_batch`; compatibility alias `llm_query_batched` must continue to resolve during migration windows.

Required validation:

- VG-RCC-001 on every M1-M4 change affecting `rlm-core` public behavior.

Pin-aware validation policy (M4-T01):

- Compatibility evidence scope is tied to the `vendor/loop` submodule SHA used by VG-RCC-001.
- Every VG-RCC-001 artifact must record both loop candidate SHA under evaluation and `rlm-claude-code/vendor/loop` submodule SHA used by the gate.
- If those SHAs differ, result scope is "verified for pinned vendor SHA" only.
- Claiming compatibility for a new loop candidate SHA requires submodule sync and gate rerun.

## Consumer B: `loop-agent`

Current coupling facts:

- Product and docs frame loop-agent as a layer on top of loop kernel.
- Runtime currently has minimal direct `rlm_core` usage; imports are not hard requirements today.
- Active development intends tighter integration.

Recommended initial integration seam (M4 target):

- Seam B1: optional kernel capability adapter for classification and trajectory emission.
- Seam B2: optional typed-signature execution backend using loop REPL contract.
- Seam B3: strict fallback behavior when kernel capabilities are absent.
- Detailed seam contract: `docs/execution-plan/contracts/LOOP-AGENT-RUNTIME-SEAM.md`.

Required contract invariants:

- Invariant B1: loop-agent core behavior remains functional without loop kernel.
- Invariant B2: when kernel is present, enabled paths are deterministic and testable.
- Invariant B3: sensitivity and telemetry guarantees are preserved when routed through kernel-backed paths.

Required validation:

- VG-LA-001 seam-critical compatibility tests.
- VG-LA-002 full-suite snapshot (advisory while `loop-agent` is under active development).
- D-014 defines objective criteria for future promotion of `VG-LA-002` from advisory to release-blocking.
- D-016 records candidate-tuple satisfaction of those criteria; D-018 records clean-clone canonical stabilization evidence on `loop-agent@30c1fa`.

## Consumer C: `io-rflx`

Current coupling facts:

- No direct crate/runtime dependency on loop currently.
- Architecture and PRD documents reference loop concepts: trajectory, memory, epistemic verification, context externalization.
- Active development intends integration.

Recommended integration shape (M4/M5 target):

- Contract C1: shared trajectory/provenance schema and export format.
- Contract C2: epistemic verification interface shape and confidence semantics.
- Contract C3: benchmark alignment for efficacy/performance comparisons.
- Detailed interop contract: `docs/execution-plan/contracts/IO-RFLX-INTEROP-CONTRACT.md`.

Required contract invariants:

- Invariant C1: integration can be validated without forcing direct compile-time dependency unless explicitly approved.
- Invariant C2: schema changes are versioned and migration-aware.

Required validation:

- VG-RFLX-001 baseline compile gate + contract evidence artifacts.
- VG-RFLX-002 fixture/calibration gate for schema roundtrip and confidence-policy checks.

## Change Management Rules

- Any breaking change to A-level invariants requires explicit decision update and migration plan.
- New loop-agent/io-rflx contracts start as additive and optional until promoted by accepted decision.
- Contract changes must update `TASK-REGISTRY.md`, `DECISIONS.md`, and `VALIDATION-MATRIX.md` in the same session.
- Compatibility claims must match an explicit supported tuple in `docs/execution-plan/COMPATIBILITY-MATRIX.md`.
- Release and rollback execution must follow `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md`.
- Recurring ownership and cadence execution must follow `docs/execution-plan/MAINTENANCE-CADENCE.md`.
