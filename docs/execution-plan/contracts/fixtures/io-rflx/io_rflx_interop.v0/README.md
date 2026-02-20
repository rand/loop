# io-rflx Interop Fixtures (`io_rflx_interop.v0`)

Fixture corpus for M7-T09 (`loop-bih.9`) contract validation.

## Files

- `provenance-envelope.json`: canonical provenance envelope sample.
- `trajectory-envelope.json`: canonical trajectory envelope sample.
- `verification-envelope.json`: canonical verification envelope sample plus calibration expectation.
- `confidence-calibration-cases.json`: deterministic calibration cases for confidence-bucket policy.

## Validation

Use `/Users/rand/src/loop/scripts/validate_rflx_interop_fixtures.py` for schema and calibration checks.

The `VG-RFLX-002` gate additionally runs targeted `io-rflx` roundtrip serialization tests to confirm shape compatibility for provenance/trajectory/verification core models.
