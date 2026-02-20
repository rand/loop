# io-rflx Interoperability Contract (M4-T03)

Date: 2026-02-19
Status: Active
Owner: Orchestrator

## Goal

Define a concrete interoperability contract between `loop` and `io-rflx` for trajectory, provenance, and verification exchange without forcing a hard compile-time dependency.

## Scope

- Consumer: `/Users/rand/src/io-rflx`
- Primary crate surface: `crates/rflx-core`
- Interop focus: C1 (shared trajectory/provenance schema), C2 (versioned and migration-aware contract)

## Contract Invariants

- C1: Integration can be validated without direct compile-time dependency by exchanging versioned JSON payloads over adapter boundaries.
- C2: Schema changes require explicit version increments and migration notes before cross-repo compatibility claims.

## Canonical Interchange Shapes (v0)

### Provenance Envelope

- Source model: `rflx_core::Provenance` and `ProvenanceSource`.
- Required fields:
- `source` (enum-like object)
- `confidence` (`Speculative|Plausible|High|Verified`)
- `created_at` (unix seconds)
- `evidence` (`string[]`)

### Trajectory Envelope

- Source model: `rflx_core::TrajectoryRecord`.
- Required fields:
- `id`, `session_id`, `timestamp`
- `routing` (`hole_id`, `hole_type`, `constraint_count`, `complexity_score`, `chosen_tier`, ...)
- `generation` (`provider`, `model`, token counts, `latency`, `estimated_cost_usd`, ...)
- `outcome` (`Verified|Rejected|Accepted|Proposed|Error`)

### Verification Envelope

- Source model: `rflx_core::VerificationResult`.
- Required fields:
- `model_loaded`
- `outcomes[]` (per-proposition status and criticality)
- `confidence` (derived score in `VerificationResult::confidence()`)
- `total_elapsed`

## loop Mapping Rules

- loop trajectory exports that feed `io-rflx` must map into the Trajectory Envelope while preserving session correlation IDs.
- loop epistemic/verification statuses map to interop confidence ladder:
- unsupported claim -> `Speculative`
- weakly supported claim -> `Plausible`
- strongly supported claim -> `High`
- formally or exhaustively validated claim -> `Verified`
- Adapter-level mapping must be pure and deterministic for identical inputs.

## Versioning and Migration

- Every payload includes `schema_version` (initially `io_rflx_interop.v0`).
- Backward-compatible additions (new optional fields) keep minor version.
- Breaking changes (field rename/removal/semantic rewrite) require major version bump and explicit migration note in M4/M6 docs.
- Fixture corpus for `io_rflx_interop.v0` is stored under `docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/`.

## Fixture Corpus and Calibration

- Canonical fixture files:
- `provenance-envelope.json`
- `trajectory-envelope.json`
- `verification-envelope.json`
- `confidence-calibration-cases.json`
- Fixture validation script: `scripts/validate_rflx_interop_fixtures.py`.
- Gate runner script: `scripts/run_rflx_interop_fixture_gate.sh`.
- Calibration policy source: `docs/execution-plan/contracts/IO-RFLX-CALIBRATION-POLICY.md`.
- Calibration method: `confidence_bucket_v1` (deterministic confidence bucket thresholds and drift checks).

## Validation Hooks

1. `VG-RFLX-001`: `CARGO_TARGET_DIR=/tmp/io-rflx-cargo-target cargo check -p rflx-core` in `/Users/rand/src/io-rflx`.
2. `VG-RFLX-002`: run `scripts/run_rflx_interop_fixture_gate.sh` (fixture schema+calibration validation plus targeted `io-rflx` roundtrip serialization tests).
3. Contract evidence review against `rflx-core` exported structures and active fixture schema version.

## M5 Benchmark Touchpoints

- Interop serialization overhead (trajectory envelope encode/decode latency).
- Event throughput for trajectory export/import path.
- Confidence calibration drift between loop and `io-rflx` verification outputs.
