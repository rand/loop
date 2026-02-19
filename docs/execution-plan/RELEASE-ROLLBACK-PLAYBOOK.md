# Release and Rollback Playbook

Date: 2026-02-19
Owner: Orchestrator
Status: Active (M6-T02)

This playbook defines deterministic release and rollback handling for loop integration changes.

## Scope

- Repo in scope: `/Users/rand/src/loop`
- Consumers in scope: `rlm-claude-code`, `loop-agent`, `io-rflx`
- Support-policy source: `docs/execution-plan/COMPATIBILITY-MATRIX.md`

## Roles

- Release owner: builds release packet, validates required gates, records tuple and evidence paths.
- Consumer owners: confirm migration impacts for their repo and confirm tuple applicability and rollback tuple readiness.
- Approver: performs go/no-go review against this checklist and `DECISIONS.md`.

## Release Classes and Required Gates

| Release Class | Typical change | Required gates | Blocking policy |
|---|---|---|---|
| R0 Docs/governance only | Specs/contracts/plan docs, no runtime behavior change | `VG-CONTRACT-001` | Block if unresolved contract ambiguity exists |
| R1 Loop runtime internals (no consumer-visible API delta) | Internal logic, bug fixes, non-contract code paths | `VG-LOOP-BUILD-001/002/003`, `VG-LOOP-CORE-001` | Block on any failing required core gate |
| R2 Consumer-visible runtime or contract change | API/behavior change, schema/version change, adapter seam changes | `VG-LOOP-*` required by scope, `VG-RCC-001`, `VG-LA-001`, `VG-RFLX-001`, `VG-CONTRACT-001` | `rlm-claude-code` compatibility is release-blocking per D-001/D-008 |
| R3 Performance-sensitive change | Any release class affecting latency/throughput or execution flow | All applicable R1/R2 gates + `VG-PERF-001`, `VG-PERF-002`, `VG-EFFICACY-001` | Block if >10% regression budget exceeded or efficacy gate fails |

## Pre-Release Checklist

1. Capture release tuple(s): `loop_sha`, `consumer_sha`, optional `vendor_loop_sha`, optional `schema_version`.
2. Confirm `docs/execution-plan/COMPATIBILITY-MATRIX.md` has current supported + rollback tuple entries.
3. Select release class (R0/R1/R2/R3) and map required VG IDs.
4. Run required gates with safe-mode controls (`scripts/safe_run.sh` for heavy commands).
5. Verify evidence artifacts exist under `docs/execution-plan/evidence/<date>/`.
6. Confirm decision deltas (if any) are recorded in `docs/execution-plan/DECISIONS.md`.
7. Build release packet summary with tuple refs, VG pass/fail table, known advisory failures, and rollback tuple/trigger table.
8. Perform go/no-go review and record decision outcome.

## Go/No-Go Rules

- Automatic no-go on any required gate failure.
- Automatic no-go on missing tuple evidence for claimed support scope.
- Automatic no-go on unapproved breaking change to active consumer contract.
- Automatic no-go for performance-sensitive releases when `VG-PERF-001` or `VG-PERF-002` fails budget.
- Automatic no-go for performance-sensitive releases when `VG-EFFICACY-001` fails.
- Conditional go requires explicit decision entry and is limited to advisory `VG-LA-002` failures outside seam scope tracked under D-009/D-014.
- Promotion claims for consumer full-suite gates require committed consumer tuple state (D-015).
- While D-017 is active, no-go on `loop-agent` claim evidence produced from canonical dirty working-tree mode instead of clean-clone committed tuple mode.

## Rollback Triggers

- Functional trigger: required gate fails after merge or during release candidate validation.
- Compatibility trigger: consumer regression in `VG-RCC-001`, `VG-LA-001`, or `VG-RFLX-001`.
- Performance trigger: >10% regression relative to approved baseline in any budgeted metric.
- Stability trigger: safe-mode memory protections fail repeatedly or release validation cannot complete without resource exhaustion.
- Contract trigger: newly discovered ambiguity or inconsistency in support tuple scope.

## Rollback Procedure

1. Declare rollback event with trigger type and impacted tuple.
2. Select prior supported rollback tuple from `COMPATIBILITY-MATRIX.md`.
3. Revert loop and/or consumer pin to rollback tuple refs.
4. Rerun minimal release-blocking gates for rollback tuple: `VG-CONTRACT-001`, `VG-RCC-001` (for consumer-visible changes), and additional original-class gates as applicable.
5. Publish rollback evidence under `docs/execution-plan/evidence/<date>/`.
6. Update trackers: `STATUS.md` (blocker/resolution note), `TASK-REGISTRY.md` (new remediation task if needed), and `WORKBOARD.md` (lane reassignment if required).
7. Create decision or issue entries for root-cause remediation before next release attempt.

## Release Packet Template

- `release_date`:
- `release_owner`:
- `release_class`:
- `tuple_current`:
- `tuple_rollback`:
- `required_gates`:
- `gate_results`:
- `advisory_findings`:
- `go_no_go`:
- `approver`:
- `follow_up_actions`:

## Tooling References

- Compatibility gate pipeline: `scripts/run_m4_compat_pipeline.sh`
- Performance gate harness: `scripts/run_m5_perf_harness.sh`
- Safety wrapper: `scripts/safe_run.sh`
- Steady-state cadence: `docs/execution-plan/MAINTENANCE-CADENCE.md`
