# Steady-State Maintenance Cadence and Ownership

Date: 2026-02-19
Owner: Orchestrator
Status: Active (M6-T03)

This document defines recurring operational governance after M0-M6 completion.

## Ownership Model

- Orchestrator owner: owns tracker coherence (`STATUS.md`, `TASK-REGISTRY.md`, `WORKBOARD.md`, `DECISIONS.md`), approves lane activation/gate scope changes, validates handoff packets, and closes task cards.
- Lane C execution owner: runs active consumer/performance/release governance work, runs heavy gates in safe mode only, and stores evidence artifacts with tracker update proposals.
- Lane A/B maintenance owners: handle reopened regression fixes only when orchestrator activates the lane.
- Consumer owners (`rlm-claude-code`, `loop-agent`, `io-rflx`): review contract-impacting changes and confirm tuple applicability.
- Release approver: executes go/no-go using `RELEASE-ROLLBACK-PLAYBOOK.md`.

## Cadence Schedule

| Cadence | Activity | Owner | Required Output |
|---|---|---|---|
| Every active coding session | Run safe-start checklist and confirm heavy-command serialization | Active lane owner | Session notes + safe-mode compliance |
| Per consumer-visible loop change | Refresh compatibility tuple claim and run scoped gates | Lane C owner | Updated tuple evidence and matrix rows |
| Weekly | Run cadence packet runner (`scripts/run_weekly_cadence_packet.sh`) | Lane C owner | Weekly packet + compatibility artifacts under current date |
| Weekly | Review `loop-agent` advisory full-suite status (`VG-LA-002`) and triage drift | Lane C owner + loop-agent owner | Failure summary or green snapshot; evaluate D-014 progress, D-015 committed-state requirement, and D-017 clean-clone tuple policy |
| Weekly | Spec/runtime drift review for active spec files and contracts | Lane A owner (or Lane C if A paused) | Updated drift checklist evidence |
| Monthly | Review decision ledger and deprecation timers | Orchestrator owner | Decision audit note and required follow-up tasks |
| Monthly | Validate rollback tuple availability per consumer | Orchestrator + lane owner | Matrix update confirming current + rollback tuples |
| Before release/go-no-go | Execute release class checklist and rollback readiness check | Release owner + approver | Release packet per playbook template |

## Required Validation Footprint

- Contract/governance changes must pass `VG-CONTRACT-001`.
- Consumer tuple claim changes must pass applicable consumer gates (`VG-RCC-001`, `VG-LA-001`, `VG-RFLX-001`).
- Performance-sensitive releases must pass `VG-PERF-001`, `VG-PERF-002`, and `VG-EFFICACY-001`.
- Advisory gates stay visible in evidence even when non-blocking by accepted decision policy.

## Escalation Triggers

- Required gate cannot run or repeatedly fails under safe-mode memory thresholds.
- Contract ambiguity appears between matrix, playbook, and consumer contract docs.
- Hard consumer (`rlm-claude-code`) compatibility breaks without approved migration.
- Rollback tuple is missing for any actively supported consumer.
- Concurrent lane edits create tracker conflicts.

## Documentation Update Rules

1. Update operational truth in this order: `COMPATIBILITY-MATRIX.md` -> `RELEASE-ROLLBACK-PLAYBOOK.md` -> trackers.
2. Add decision entries for policy changes before claiming completed governance tasks.
3. Keep evidence co-located by date/milestone; do not rely on chat history for release decisions.
4. If cadence work identifies unresolved implementation work, create/refresh tasks before ending session.
