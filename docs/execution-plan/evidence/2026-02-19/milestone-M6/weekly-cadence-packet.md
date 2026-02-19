# Weekly Cadence Packet
Date: 2026-02-19
Result: fail
Runner: `scripts/run_weekly_cadence_packet.sh`
LOOP_MIN_AVAILABLE_MIB: `4096`
RUN_LA_FULL_SNAPSHOT: `1`

## Governance Sources

- `docs/execution-plan/COMPATIBILITY-MATRIX.md`
- `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md`
- `docs/execution-plan/MAINTENANCE-CADENCE.md`

## Tuple Snapshot

```
loop_branch=main
loop_sha=50cd8cfe95f3179a4f15a445199fa9b1d1fe91f9
rlm_claude_code_branch=main
rlm_claude_code_sha=54d88c085851fdc08028f3c1835527979645ffe5
rlm_claude_code_vendor_loop= 6779cdbc970c70f3ce82a998d6dcda59cd171560 vendor/loop (heads/main)
loop_agent_branch=dp/loop-agent
loop_agent_sha=da1318fc04357e5a3476efd967a65f61febea16f
io_rflx_branch=main
io_rflx_sha=abf11ca4069bac7a740508d02242114483a6cf51
io_rflx_interop_schema=io_rflx_interop.v0
```

## Compatibility Gate Artifacts

- `VG-RCC-001`: `fail` (`/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-RCC-001.txt`)
- `VG-LA-001`: `pass` (`/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-LA-001.txt`)
- `VG-RFLX-001`: `fail` (`/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-19/milestone-M6/weekly-cadence-m4/M4-T04-VG-RFLX-001.txt`)
- `VG-LA-002` advisory snapshot: `915 passed in 21.31s`

## Gate Notes

- `VG-RCC-001`: `ImportError while loading conftest '/Users/rand/src/rlm-claude-code/tests/conftest.py'.`
- `VG-RFLX-001`: `error: failed to open: /Users/rand/src/io-rflx/target/debug/.cargo-lock`

## Policy Notes

- Full-suite `VG-LA-002` promotion criteria are governed by D-014.
- This packet is intended for weekly cadence review and release-readiness context updates.
