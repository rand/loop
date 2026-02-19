# Safe Mode Workboard

Last updated: 2026-02-19
Owner: Orchestrator thread

## Mode

- Safe mode enabled.
- Heavy command concurrency: 1.
- Required wrapper: `/Users/rand/src/loop/scripts/safe_run.sh`
- Recommended threshold: `LOOP_MIN_AVAILABLE_MIB=4096`

## Active Assignments

| Lane | Current Assignment | Status | Notes |
|---|---|---|---|
| Orchestrator | M7 tranche orchestration + safe-mode enforcement | in_progress | M7 plan published; execute task cards sequentially with evidence-first closure |
| Lane A | M7 core runtime closure (`M7-T01`..`M7-T08`) | active | Start with `M7-T01` (`loop-bih.1`); one heavy task at a time |
| Lane B | M7 docs/governance reconciliation prep (`M7-T10`) | paused | Read-only until orchestrator activates after implementation tasks |
| Lane C | Ops-Weekly cadence + M7 interop follow-up (`M7-T09`) | in_progress | Keep D-017 clean-clone policy active; run cadence between M7 heavy tasks as admission allows |

## Next Queue by Lane

- Lane A: `M7-T01` (`loop-bih.1`) -> `M7-T02` (`loop-bih.2`) -> `M7-T03` (`loop-bih.3`)
- Lane B: prepare SPEC status reconciliation checklist for `M7-T10`
- Lane C: continue Ops-Weekly cadence; stage `M7-T09` fixture/calibration prerequisites

## Lane Activation Rules

- Lane A is primary heavy lane for M7 implementation tasks.
- Lane C may run heavy commands only when Lane A is idle.
- Lane B remains read-only until orchestrator explicitly marks it active.
- Never run heavy commands concurrently across lanes.

## Handoff Intake Checklist (Orchestrator)

1. Task ID matches assigned lane.
2. Required VG IDs were run.
3. Evidence artifact files exist.
4. Safe wrapper was used for heavy commands.
5. No unauthorized edits to orchestrator-only files.
6. Task status transitioned in `TASK-REGISTRY.md`.
