# M7-T09 VG-CONTRACT-001
Date: 2026-02-20
Task: M7-T09 io-rflx adapter fixture + calibration delivery

## Scope
Reconcile io-rflx interoperability contract, fixture assets, and validation gates for schema version `io_rflx_interop.v0`.

## Checklist
- [x] Interop contract includes canonical fixture corpus paths and active schema version.
- [x] Calibration method and threshold policy are explicitly documented and versioned.
- [x] `VG-RFLX-002` command is executable and deterministic in loop-owned safe mode.
- [x] Consumer contract references fixture/calibration gate requirements for io-rflx.
- [x] Validation matrix command map aligns with current gate implementation.

## Result
- Pass
- Contract now defines a concrete fixture+calibration validation path for io-rflx integration claims beyond compile-only checks.

## References
- `/Users/rand/src/loop/docs/execution-plan/contracts/IO-RFLX-INTEROP-CONTRACT.md`
- `/Users/rand/src/loop/docs/execution-plan/contracts/IO-RFLX-CALIBRATION-POLICY.md`
- `/Users/rand/src/loop/docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0/README.md`
- `/Users/rand/src/loop/docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
- `/Users/rand/src/loop/docs/execution-plan/VALIDATION-MATRIX.md`
- `/Users/rand/src/loop/scripts/run_rflx_interop_fixture_gate.sh`
