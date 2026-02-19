# Loop-Agent Runtime Seam Contract (M4-T02)

Date: 2026-02-19
Status: Active
Owner: Orchestrator

## Goal

Define the first concrete runtime integration seam between `loop-agent` and `loop` that is optional, deterministic, and testable.

## Scope

- Consumer: `/Users/rand/src/loop-agent`
- Integration target: classifier routing + trajectory propagation into loop kernel services
- Non-goal in M4-T02: broad refactor of unrelated durability/optimizer workstreams in `loop-agent`

## Seam Definition

### S1. Optional Kernel Classifier Adapter

- Hook point: route selection path (`Router.classifier` contract).
- Adapter is optional; absence must preserve current classifier behavior.
- Adapter outputs must map to existing route names only.
- Unknown route or adapter error must fall back to existing local classifier logic.

### S2. Optional Kernel Trajectory Sink

- Hook point: trajectory event bridge (`TrajectoryEvent` stream before/alongside OTel emission).
- Adapter is optional; absence must preserve current OTel-only behavior.
- Adapter failures are non-fatal by default and must not abort module execution.

### S3. Sensitivity and Telemetry Guardrails

- Existing sensitivity filtering remains authoritative before adapter emission.
- No new unfiltered payload path is introduced by seam hooks.
- Deterministic telemetry behavior is preserved regardless of adapter presence.

## Invariant Mapping

- B1 (works without loop kernel): satisfied by optional adapter + fallback default.
- B2 (kernel-enabled paths deterministic/testable): satisfied by explicit route-name constraints and defined fallback behavior.
- B3 (sensitivity/telemetry guarantees preserved): satisfied by mandatory pre-emission filtering and non-fatal adapter policy.

## Validation Profile (M4)

- Required gate for seam scope: `VG-LA-001` (seam-critical test subset).
- Advisory health signal: `VG-LA-002` (full-suite snapshot + triage evidence).
- Contract evidence gate: `VG-CONTRACT-001`.

## Full-Suite Promotion Policy (Post-M6)

- Policy source: D-014.
- `VG-LA-002` remains advisory until full-suite reaches `0` failures on supported tuples.
- Non-green `VG-LA-002` runs require explicit failure-category triage artifacts.
- Promotion to release-blocking requires three consecutive green snapshots.
- D-015 requires those promotion snapshots to be tied to committed consumer tuple state.
- D-016 records criteria satisfaction for candidate tuple `f2aeb18`; D-018 records canonical committed tuple stabilization evidence on `30c1fa`.

## Minimal Integration Harness Plan

1. Add a small kernel adapter protocol in `loop-agent` that is injectable and optional.
2. Add deterministic fake adapter tests for router decision mapping and fallback behavior.
3. Add trajectory adapter tests proving non-fatal emission and OTel continuity.
4. Add sensitivity regression tests verifying filtered payloads on adapter-bound events.
5. Keep all tests runnable with `MockBackend` and without a real loop kernel dependency.
