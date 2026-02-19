# Compatibility Matrix and Support Policy

Date: 2026-02-19
Owner: Orchestrator
Status: Active (M6-T01)

This document is the canonical support-policy source for `loop` and active consumers.

## Scope

- Producer repo: `/Users/rand/src/loop`
- Consumer repo: `/Users/rand/src/rlm-claude-code`
- Consumer repo: `/Users/rand/src/loop-agent`
- Consumer repo: `/Users/rand/src/io-rflx`

## Policy Terms

- Compatibility tuple fields: `loop_sha`, `consumer_sha`, optional `vendor_loop_sha`, optional `schema_version`.
- Tier `supported`: required gates pass for the tuple and are documented in evidence.
- Tier `conditional`: required seam/contract gates pass, but a declared advisory gate is non-green.
- Tier `unsupported`: no validated tuple or tuple does not satisfy active policy.

## Current Validated Tuples (2026-02-19)

| Consumer | Consumer ref | Loop ref scope | Contract shape | Required gates | Latest evidence | Tier | Notes |
|---|---|---|---|---|---|---|---|
| `rlm-claude-code` | `54d88c085851fdc08028f3c1835527979645ffe5` | pinned `vendor/loop` = `6779cdbc970c70f3ce82a998d6dcda59cd171560` | Hard runtime/build vendoring (`rlm_core`) | `VG-RCC-001`, `VG-CONTRACT-001` | `evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-RCC-001.txt` | `supported` | Pin-aware scope only (D-008). Not a claim for `/Users/rand/src/loop` HEAD unless pin is updated and rerun. |
| `loop-agent` | active committed canonical: `30c1fa786d79e0984cf464ffb8e67cc7a1bfcaeb`; historical promotion candidate: `f2aeb1859592ef82f63f6ae416973854c381666b` (`/tmp/loop-agent-clean`) | `/Users/rand/src/loop` runtime seam contract | Optional runtime seam (classifier + trajectory + sensitivity guardrails) | `VG-LA-001`, `VG-CONTRACT-001` | `evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-LA-001.txt` | `supported` | Clean-clone tuple run on canonical committed SHA reports `VG-LA-001` green (`30 passed`) and advisory `VG-LA-002` green (`936 passed`), superseding pending-landing assumptions from D-016/loop-5va while retaining D-017 claim-source policy. |
| `io-rflx` | `abf11ca4069bac7a740508d02242114483a6cf51` | schema-first interop with loop | `io_rflx_interop.v0` | `VG-RFLX-001`, `VG-CONTRACT-001` | `evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-RFLX-001.txt` | `supported` | Compile + contract validation scope, no hard compile-time loop dependency required. Weekly cadence pass is currently established with isolated `RFLX_CARGO_TARGET_DIR` strategy. |

## Support Window

- While repos are pre-`1.0` and partially untagged, support is tuple-based (SHA-based), not generic branch-wide.
- At least two tuples per consumer must be retained in evidence for operational safety: current supported tuple and previous rollback tuple.
- `rlm-claude-code` remains release-blocking for loop changes that affect public `rlm-core` behavior (D-001, D-008).
- `loop-agent` and `io-rflx` remain governed by their explicit contracts and gate scopes (D-009, D-010, D-014).

## Deprecation Lead-Time Policy

- Any loop change that removes or renames a consumer-observed API/behavior must be captured in `DECISIONS.md` with migration notes.
- Any such change must include a compatibility/deprecation notice in docs/specs.
- Any such change must provide at least 14 calendar days lead time before hard removal on `main`.
- Any such change must include at least one successful rerun of relevant compatibility gate(s) after notice publication.
- For schema-first interop (`io-rflx`), breaking changes require version bump + migration notes before compatibility can be claimed.

## Compatibility Claim Rules

- A compatibility claim is valid only for the exact tuple documented in evidence.
- If any tuple component changes (loop SHA, consumer SHA, vendor pin, schema version), rerun relevant gates before claiming support.
- `scripts/run_m4_compat_pipeline.sh` is the default reproducible command set for cross-repo gate refreshes.
- While D-017 is active, `loop-agent` gate claims must come from `clean_clone_committed` tuple mode and not from canonical dirty working-tree state.
- Release go/no-go and rollback execution follow `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md`.
- Recurring refresh cadence and owners follow `docs/execution-plan/MAINTENANCE-CADENCE.md`.

## Update Workflow

1. Capture new tuple refs (`git rev-parse`, submodule SHA if applicable).
2. Run required gates for impacted consumers.
3. Store evidence in `docs/execution-plan/evidence/<date>/`.
4. Update this matrix, `STATUS.md`, `TASK-REGISTRY.md`, and `WORKBOARD.md`.
5. If policy changes, add a new decision entry in `DECISIONS.md`.
6. Preferred weekly execution path: `scripts/run_weekly_cadence_packet.sh`.
