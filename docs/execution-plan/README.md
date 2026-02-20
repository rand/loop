# Loop Implementation Plan (Execution Package)

This package turns the evaluation findings into an execution-ready program for `loop`, optimized for multi-session delivery by Codex or Claude Code with minimal context overhead.

Safe mode is the default operating profile on this machine due prior memory exhaustion.

Last reconciled: 2026-02-20 (live state source: `docs/execution-plan/STATUS.md` + Beads).

## Execution Principles

- Contract-first: lock cross-repo behavior before changing implementation.
- Compatibility-first: preserve `rlm-claude-code` behavior unless an explicit breaking decision is approved.
- Safety-first: run one heavy execution lane at a time on this machine.
- Evidence-first: no task is complete without validation artifacts.
- Small-batch: execute one task card at a time, not one milestone at a time.
- Reproducibility: every gate has a deterministic command and pass criteria.

## Current Reality (Live Snapshot: 2026-02-20 Reconciliation)

Use `docs/execution-plan/STATUS.md` for live status before acting on any item below.

- M0-M7 remediation/governance milestones are complete with evidence.
- Historical M7 gaps (`G-001`, `G-002`) are closed by `M7-T01` and `M7-T02`.
- Post-M7 critical refinement backlog (`loop-azq` + `loop-azq.1..9`) is implemented and closed.
- No open implementation backlog currently exists in Beads (`open=0`, `in_progress=0`, `blocked=0`).
- `loop-agent` and `io-rflx` remain active integration targets; claim-grade compatibility evidence follows D-017 clean-clone committed-tuple policy.
- Safe mode remains mandatory due prior OOM history; heavy command concurrency stays at 1.

## File Map

Read in this order at session start:

1. `docs/execution-plan/ORCHESTRATION.md`
2. `docs/execution-plan/STATUS.md`
3. `docs/execution-plan/TASK-REGISTRY.md`
4. `docs/execution-plan/LANE-MAP.md` (parallel mode)
5. `docs/execution-plan/WORKBOARD.md`
6. `docs/execution-plan/safe-mode/SAFETY-POLICY.md`
7. `docs/execution-plan/safe-mode/SAFE-START-CHECKLIST.md`
8. `docs/execution-plan/SESSION-RUNBOOK.md`
9. One active milestone file in `docs/execution-plan/milestones/`
10. `docs/execution-plan/VALIDATION-MATRIX.md` (when verifying)
11. `docs/execution-plan/COMPATIBILITY-MATRIX.md` (M6 governance and support scope)
12. `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md` (M6 release operation)
13. `docs/execution-plan/MAINTENANCE-CADENCE.md` (M6 steady-state operations)
14. `docs/execution-plan/WEEKLY-CADENCE-PACKET.md` (cadence runner)

Reference files:

- `docs/execution-plan/DECISIONS.md`
- `docs/execution-plan/COMPATIBILITY-MATRIX.md`
- `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md`
- `docs/execution-plan/MAINTENANCE-CADENCE.md`
- `docs/execution-plan/WEEKLY-CADENCE-PACKET.md`
- `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
- `docs/execution-plan/evidence/README.md`
- `docs/execution-plan/THREAD-STARTER-PACK.md`
- `docs/execution-plan/WORKBOARD.md`
- `scripts/safe_run.sh`

## Milestones

- `M0`: Foundation and contract freeze
- `M1`: Build and toolchain integrity
- `M2`: REPL protocol and typed-signature closure
- `M3`: SPEC/API drift resolution
- `M4`: Consumer integration tracks
- `M5`: Performance and efficacy validation
- `M6`: Rollout and steady-state governance
- `M7`: Spec completion and integration hardening

Milestone details live in `docs/execution-plan/milestones/M0.md` through `docs/execution-plan/milestones/M7.md`.

## Agent Context Strategy

- Load only one milestone file and one task card at a time.
- Keep raw logs out of chat; store them under `docs/execution-plan/evidence/`.
- Use `TASK-REGISTRY.md` as the single task index; avoid duplicating task text in handoffs.
- Record architectural deltas only in `DECISIONS.md`.

## Completion Rules

A task can be marked done only when all are true:

- Implementation for the task is complete.
- Required validation gates passed.
- Evidence artifacts are stored in the expected path.
- `STATUS.md` and `TASK-REGISTRY.md` are updated.
