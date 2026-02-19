# Weekly Cadence Packet Runner

Deterministic weekly runner for governance packet generation across:

- `COMPATIBILITY-MATRIX.md`
- `RELEASE-ROLLBACK-PLAYBOOK.md`
- `MAINTENANCE-CADENCE.md`

## Script

- Path: `scripts/run_weekly_cadence_packet.sh`
- Execution mode: serialized heavy commands via `scripts/safe_run.sh`

## Default Run

```bash
cd /Users/rand/src/loop
LOOP_MIN_AVAILABLE_MIB=4096 scripts/run_weekly_cadence_packet.sh
```

## Optional Controls

- `EVIDENCE_DATE` (default: today, `YYYY-MM-DD`)
- `EVIDENCE_DIR` (default: `docs/execution-plan/evidence/<date>/milestone-M6`)
- `PACKET_PREFIX` (default: `weekly-cadence`)
- `RUN_LA_FULL_SNAPSHOT` (default: `1`)
- `LOOP_AGENT_CANONICAL_DIR` (default: `/Users/rand/src/loop-agent`)
- `LOOP_AGENT_CLEAN_CLONE_DIR` (default: `/tmp/loop-agent-clean-cadence`)
- `LOOP_AGENT_TUPLE_POLICY` (default and only supported value: `committed_clean_clone_only`)

## Outputs

- `<prefix>-packet.md` (governance packet)
- `<prefix>-tuples.txt` (tuple snapshot for supported consumers)
- `<prefix>-run.log` (runner log)
- `<prefix>-m4/` compatibility gate artifacts:
- `M4-T04-VG-RCC-001.txt`
- `M4-T04-VG-LA-001.txt`
- `M4-T04-VG-RFLX-001.txt`
- Optional `M4-T04-VG-LA-002.txt` when enabled

## Notes

- Full-suite `VG-LA-002` stays advisory unless D-014 promotion criteria are satisfied.
- Runner sets temp cache paths (`/tmp`) for `uv`/pytest artifacts to remain sandbox-safe.
- Loop-agent compatibility claims are generated from a clean clone at committed canonical SHA (tuple mode `clean_clone_committed`) until canonical stabilization is explicitly declared.
