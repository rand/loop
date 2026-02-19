# M1-T02 Validation Summary
Date: 2026-02-19
Task IDs: M1-T02
VG IDs: VG-LOOP-BUILD-001, VG-LOOP-BUILD-002, VG-LOOP-BUILD-003
Result: pass
Notes: Executed in safe mode with `LOOP_MIN_AVAILABLE_MIB=3072` and `safe_run.sh` wrapper.

## Artifacts

- `M1-T02-VG-LOOP-BUILD-001.txt`
- `M1-T02-VG-LOOP-BUILD-002.txt`
- `M1-T02-VG-LOOP-BUILD-003.txt`
- `M1-T02-workflow-matrix-snapshot.txt`
- `M1-T02-workflow-yaml-parse.txt`

## Outcomes

- Added CI workflow `/Users/rand/src/loop/.github/workflows/rlm-core-feature-matrix.yml`.
- CI matrix includes baseline profiles mapped to VG IDs:
  - `VG-LOOP-BUILD-001` -> default profile
  - `VG-LOOP-BUILD-002` -> `--no-default-features`
  - `VG-LOOP-BUILD-003` -> `--no-default-features --features gemini`
- Workflow logs and job names identify the failing profile by VG ID and label.
