# loop-the VG-CONTRACT-001
Date: 2026-02-19
Task IDs: loop-the
VG IDs: VG-CONTRACT-001, VG-LA-001, VG-LA-002
Command(s):
- `safe_run.sh` targeted `VG-LA-001` subset
- `safe_run.sh` full `VG-LA-002` snapshot
- Contract policy review and decision update
Result: pass
Notes: Updated loop-agent advisory gate policy to include explicit promotion criteria based on measured failure envelope.

## Checklist

- [x] Captured fresh full-suite snapshot (`865 passed, 2 failed`).
- [x] Captured fresh seam-critical subset snapshot (`30 passed`).
- [x] Produced failure-class triage for remaining full-suite failures.
- [x] Added decision update with promotion criteria for `VG-LA-002` (D-014).
- [x] Updated tracker docs to reflect narrowed B6 scope and next execution priority.

## Outcome

`VG-LA-002` remains advisory under D-009, with explicit promotion criteria now recorded in D-014.
