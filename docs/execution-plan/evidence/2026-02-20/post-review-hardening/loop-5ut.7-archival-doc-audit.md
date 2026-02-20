# Archival Documentation Reconciliation (`loop-5ut.7`)
Date: 2026-02-20

## Objective
Eliminate ambiguity between live backlog and historical planning/design artifacts by making archival status explicit and pointing readers to authoritative live trackers.

## Changes Applied

### Archival banner + non-backlog checklist policy added
- `/Users/rand/src/loop/docs/implementation-roadmap.md`
- `/Users/rand/src/loop/docs/implementation-plan-wave3-5.md`
- `/Users/rand/src/loop/docs/migration-spec-rlm-claude-code.md`
- `/Users/rand/src/loop/docs/migration-spec-recurse.md`
- `/Users/rand/src/loop/docs/unified-rlm-library-design.md`
- `/Users/rand/src/loop/docs/lean-formal-verification-design.md`

### Live authority links added to primary docs index
- `/Users/rand/src/loop/docs/README.md`

## Policy Clarification Introduced
Unchecked `[ ]` items inside the files above are now explicitly documented as archival snapshots, not live backlog.

Authoritative live status sources are now consistently referenced:
- `bd status` / `bd ready`
- `/Users/rand/src/loop/docs/execution-plan/STATUS.md`
- `/Users/rand/src/loop/docs/execution-plan/TASK-REGISTRY.md`
- `/Users/rand/src/loop/docs/execution-plan/WORKBOARD.md`

## Result
Historical docs remain useful for context, but no longer present unresolved checklists as active implementation commitments.
