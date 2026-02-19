# M4-T03 VG-CONTRACT-001
Date: 2026-02-19
Task: M4-T03 Define interoperability contract for `io-rflx`
Gate: VG-CONTRACT-001
Result: pass

## Contract Review Scope

- `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
- `docs/execution-plan/contracts/IO-RFLX-INTEROP-CONTRACT.md`
- `M4-T03-io-rflx-state.txt`
- `M4-T03-io-rflx-interoperability-scan.txt`
- `M4-T03-VG-RFLX-001.txt`

## Invariant Check

- C1 (validate without forced compile-time dependency): satisfied by schema-first adapter contract and compile-gate baseline.
- C2 (versioned/migration-aware schema): satisfied by explicit `schema_version` policy (`io_rflx_interop.v0`) and D-010 decision.

## Interop Surface Confirmation

- `rflx-core` exports provenance, trajectory, and verification structures needed for interchange.
- Provenance includes source, confidence ladder, timestamps, and evidence references.
- Trajectory includes routing/generation/outcome snapshots with session correlation.
- Verification includes per-proposition outcomes plus aggregate confidence scoring.

## Conclusion

M4-T03 contract-level interoperability is concretely defined, versioned, and backed by compile and source-surface evidence.
