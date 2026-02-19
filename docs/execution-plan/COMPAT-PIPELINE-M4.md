# M4 Compatibility Pipeline

Deterministic cross-repo compatibility recipe for M4 consumer gates.

## Script

- Path: `scripts/run_m4_compat_pipeline.sh`
- Execution mode: serialized heavy commands via `scripts/safe_run.sh`

## Default Run

```bash
cd /Users/rand/src/loop
LOOP_MIN_AVAILABLE_MIB=4096 scripts/run_m4_compat_pipeline.sh
```

## Optional Full `loop-agent` Snapshot

```bash
cd /Users/rand/src/loop
LOOP_MIN_AVAILABLE_MIB=4096 RUN_LA_FULL_SNAPSHOT=1 scripts/run_m4_compat_pipeline.sh
```

## Required Gate Outputs

- `M4-T04-VG-RCC-001.txt`
- `M4-T04-VG-LA-001.txt`
- `M4-T04-VG-RFLX-001.txt`
- `M4-T04-pipeline-summary.md`
- `M4-T04-pipeline-run.log`

## Notes

- `VG-LA-002` is advisory on active `loop-agent` branches (see D-009).
- Evidence directory defaults to `docs/execution-plan/evidence/<today>/milestone-M4/`.
- Override with `EVIDENCE_DATE` or full `EVIDENCE_DIR` when replaying.
