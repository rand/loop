# Compatibility Matrix and Support Policy

Date: 2026-02-20
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

## Current Validated Tuples (2026-02-20)

| Consumer | Consumer ref | Loop ref scope | Contract shape | Required gates | Latest evidence | Tier | Notes |
|---|---|---|---|---|---|---|---|
| `rlm-claude-code` | `528f90018e0d464aa7e7459998191d8cfde27787` | loop candidate `75f806f85985302c498e9d8e4915af6f144ed6ad`; pinned `vendor/loop` = `6779cdbc970c70f3ce82a998d6dcda59cd171560` | Hard runtime/build vendoring (`rlm_core`) | `VG-RCC-001`, `VG-CONTRACT-001` | `evidence/2026-02-20/post-review-hardening/loop-5ut.6-weekly-cadence/weekly-cadence-m4/M4-T04-VG-RCC-001.txt` | `supported` | Pin-aware scope only (D-008). Candidate loop SHA differs from vendor pin; result scope is validated for the pinned vendor tuple plus compatibility check of the current loop candidate. |
| `loop-agent` | `2f4e762fbdb6fe40a00fe40b5df67b00b85dbb29` (canonical `dp/loop-agent`) | loop tuple `75f806f85985302c498e9d8e4915af6f144ed6ad` via clean-clone committed mode | Optional runtime seam (classifier + trajectory + sensitivity guardrails) | `VG-LA-001`, `VG-CONTRACT-001` | `evidence/2026-02-20/post-review-hardening/loop-5ut.6-weekly-cadence/weekly-cadence-m4/M4-T04-VG-LA-001.txt` | `supported` | D-017 policy is in force; claim-grade run used `/tmp/loop-agent-clean-cadence` clean clone, with advisory `VG-LA-002` snapshot green (`1052 passed`). |
| `io-rflx` | `abf11ca4069bac7a740508d02242114483a6cf51` | loop tuple `75f806f85985302c498e9d8e4915af6f144ed6ad` (schema-first interop) | `io_rflx_interop.v0` | `VG-RFLX-001`, `VG-RFLX-002`, `VG-CONTRACT-001` | `evidence/2026-02-20/post-review-hardening/loop-5ut.6-weekly-cadence/weekly-cadence-m4/M4-T04-VG-RFLX-001.txt` + `evidence/2026-02-20/post-review-hardening/loop-5ut.6-VG-RFLX-002.txt` | `supported` | Compile + contract validation remains additive and schema-first; fixture roundtrip/calibration checks rerun on refreshed tuple with isolated `CARGO_TARGET_DIR`. |

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
