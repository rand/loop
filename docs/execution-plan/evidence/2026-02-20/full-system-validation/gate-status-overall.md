# Gate Status Overview
Date: 2026-02-20

## Pass
- Core/runtime: `VG-LOOP-BUILD-001`, `VG-LOOP-BUILD-002`, `VG-LOOP-BUILD-003`, `VG-LOOP-SIG-001`, `VG-LOOP-REPL-001-rerun`, `VG-LOOP-REPL-002`, `VG-LOOP-CORE-001-rerun`, `VG-LOOP-BATCH-001`, `VG-LOOP-FALLBACK-001`, `VG-LOOP-SIG-002`, `VG-LOOP-DUAL-001`, `VG-LOOP-PROOF-001`, `VG-LOOP-VIZ-001`, `VG-LOOP-OPT-001`, `VG-LOOP-CONTEXT-001`, `VG-LOOP-SPEC-AGENT-001`
- Cross-repo/contracts: `VG-RCC-001`, `VG-LA-001`, `VG-LA-002` (advisory snapshot), `VG-RFLX-001`, `VG-RFLX-002`
- Perf/efficacy: `VG-PERF-001`, `VG-PERF-002`, `VG-PERF-003`
- Additional: `VG-PROPTEST-001`, `VG-PY-INTEGRATION-001`, `VG-GO-ALL-001-final`
- Manual reconciliation: `VG-DOC-SPEC-002`, `VG-CONTRACT-001`

## Failed / Blocked
- `VG-DP-ENFORCE-PRE-COMMIT`
- `VG-DP-REVIEW`
- `VG-DP-VERIFY`
- `VG-DP-ENFORCE-PRE-PUSH`

Reason: local runtime missing `dp` executable (`Failed to spawn: dp`, os error 2).
Tracked remediation: `loop-rv2`.
