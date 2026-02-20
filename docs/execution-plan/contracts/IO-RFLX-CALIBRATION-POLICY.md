# io-rflx Calibration Policy (M7-T09)

Date: 2026-02-20
Status: Active
Owner: Orchestrator

## Objective

Define a repeatable, deterministic calibration method for confidence semantics exchanged between `loop` and `io-rflx` under schema `io_rflx_interop.v0`.

## Method: `confidence_bucket_v1`

Given a confidence score `p1` in `[0.0, 1.0]`, derive the confidence bucket:

- `Speculative` if `p1 < 0.25`
- `Plausible` if `0.25 <= p1 < 0.50`
- `High` if `0.50 <= p1 < 0.85`
- `Verified` if `p1 >= 0.85`

## Policy Checks

1. All fixture files must declare `schema_version = io_rflx_interop.v0`.
2. Calibration cases in `confidence-calibration-cases.json` must map to expected buckets exactly.
3. Verification envelope confidence bucket must be within `max_bucket_distance` (default `1`) of the fixture's expected bucket.
4. Confidence ladder order must remain monotonic:
   `Speculative < Plausible < High < Verified`.

## Fixture Set

- `/Users/rand/src/loop/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/provenance-envelope.json`
- `/Users/rand/src/loop/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/trajectory-envelope.json`
- `/Users/rand/src/loop/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/verification-envelope.json`
- `/Users/rand/src/loop/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/confidence-calibration-cases.json`

## Gate Integration

`VG-RFLX-002` consumes this policy via `/Users/rand/src/loop/scripts/validate_rflx_interop_fixtures.py`.
