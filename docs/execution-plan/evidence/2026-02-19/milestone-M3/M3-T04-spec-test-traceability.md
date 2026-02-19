# M1-M3 Spec-to-Test Traceability
Date: 2026-02-19
Owner: Orchestrator lane
Scope: M1, M2, M3 task acceptance criteria and required gates

## Traceability Table

| Task | Critical Acceptance Criterion | Implementation Reference | Validation Evidence | Coverage | Gap / Follow-up |
|---|---|---|---|---|---|
| M1-T01 | Feature-list compile behavior is stable across cfg combinations | `rlm-core/src/ffi/mod.rs` | `M1-T01-validation-summary.md`, `VG-LOOP-BUILD-001/002/003` artifacts | Covered | None |
| M1-T02 | Baseline feature-matrix checks are enforced in CI | `.github` workflow updates, build-matrix docs | `M1-T02-validation-summary.md`, `M1-T02-workflow-*.txt` | Covered | None |
| M1-T03 | REPL subprocess spawn works in dev/packaged invocation paths | `rlm-core/src/repl.rs`, `rlm-core/python/rlm_repl/__main__.py` | `M1-T03-validation-summary.md`, `M1-T03-VG-LOOP-REPL-002.txt` | Covered | None |
| M1-T04 | Spawn failures return actionable diagnostics and docs match runtime | `rlm-core/src/repl.rs`, `rlm-core/python/README.md` | `M1-T04-validation-summary.md`, `M1-T04-VG-DOC-SPEC-001.md` | Covered | None |
| M1-T05 | Full gemini-profile regression suite is stable | multi-file fixes in `rlm-core/src/*` + scrubber/LLM test alignment | `M1-T05-validation-summary.md`, `M1-T05-VG-LOOP-CORE-001-r3.txt` | Covered | Monitor for new regressions in M4+ |
| M2-T01 | JSON-RPC methods `register_signature` / `clear_signature` are implemented | `rlm-core/python/rlm_repl/main.py`, `protocol.py` | `M2-T01-validation-summary.md`, `M2-T01-VG-LOOP-REPL-001.txt` | Covered | None |
| M2-T02 | `SUBMIT(outputs)` captures and validates structured outputs | `rlm-core/python/rlm_repl/sandbox.py` | `M2-T02-validation-summary.md`, `M2-T02-repl-submit-scenarios.txt` | Covered | None |
| M2-T03 | Execute response includes typed `submit_result` across Rust/Python boundary | `rlm-core/src/repl.rs` + Python protocol payload | `M2-T03-validation-summary.md`, `M2-T03-submit-result-roundtrip-tests.txt` | Covered | None |
| M2-T04 | Typed submit scenario matrix (success + failure modes) has integration coverage | `rlm-core/src/repl.rs` ignored integration tests + Python tests | `M2-T04-validation-summary.md`, `M2-T04-submit-roundtrip-scenarios.txt` | Covered | Ignored Rust tests require Python environment; keep env parity checks in CI |
| M2-T05 | SPEC-20 runtime contract reflects implementation | `docs/spec/SPEC-20-typed-signatures.md` | `M2-T05-validation-summary.md`, `M2-T05-VG-DOC-SPEC-001.md` | Covered | None |
| M3-T01 | Canonical/alias batched helper naming policy is implemented without breakage | `rlm-core/python/rlm_repl/helpers.py`, `sandbox.py`, `docs/spec/SPEC-26-batched-queries.md` | `M3-T01-validation-summary.md`, `M3-T01-VG-DOC-SPEC-001.md`, `M3-T01-VG-CONTRACT-001.md` | Covered | Validate cross-repo runtime behavior in M4 gates |
| M3-T02 | SPEC-20 file locations and test references are accurate | `docs/spec/SPEC-20-typed-signatures.md` | `M3-T02-validation-summary.md`, `M3-T02-file-location-check.txt`, `M3-T02-test-reference-check.txt` | Covered | None |
| M3-T03 | SPEC-26/27 clearly distinguish implemented vs planned architecture | `docs/spec/SPEC-26-batched-queries.md`, `docs/spec/SPEC-27-fallback-extraction.md` | `M3-T03-validation-summary.md`, `M3-T03-VG-DOC-SPEC-001.md` | Covered | Track planned integration items in M4/M5 |
| M3-T04 | M1-M3 acceptance criteria are traceable to tests/gates | this artifact | `M3-T04-validation-summary.md`, `M3-T04-VG-DOC-SPEC-001.md` | Covered | Keep updated as new tasks land |

## Open Coverage Gaps (Outside M1-M3 Completion)

| Gap ID | Gap | Related Spec/Task | Planned Owner / Milestone |
|---|---|---|---|
| G-001 | End-to-end REPL-host execution integration tests for `LLM_BATCH` are not yet explicit | SPEC-26 integration behavior | M4/M5 (consumer + performance lanes) |
| G-002 | Orchestrator-level fallback loop wiring tests are not yet explicit in current runtime | SPEC-27 integration section | M4/M5 |
| G-003 | Cross-repo compatibility gates (`VG-RCC-001`, `VG-LA-001`, `VG-RFLX-001`) not yet executed post M3 docs alignment | M4 tasks | M4 |
