# VG-CONTRACT-001
Date: 2026-02-20
Scope: Consumer contract consistency check against active implementations and tuple evidence
Status: Historical baseline. Refreshed tuple evidence is in `/Users/rand/src/loop/docs/execution-plan/evidence/2026-02-20/post-review-hardening/loop-5ut.6-VG-CONTRACT-001.md`.

## Contract Sources Reviewed
- `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
- `docs/execution-plan/contracts/LOOP-AGENT-RUNTIME-SEAM.md`
- `docs/execution-plan/contracts/IO-RFLX-INTEROP-CONTRACT.md`
- `docs/execution-plan/COMPATIBILITY-MATRIX.md`

## Empirical Gate Results
- `VG-RCC-001`: pass (`weekly-cadence-m4/M4-T04-VG-RCC-001.txt`)
- `VG-LA-001`: pass (`weekly-cadence-m4/M4-T04-VG-LA-001.txt`)
- `VG-LA-002` advisory: pass (`1052 passed in 21.31s`)
- `VG-RFLX-001`: pass (`weekly-cadence-m4/M4-T04-VG-RFLX-001.txt`)
- `VG-RFLX-002`: pass (`VG-RFLX-002.txt`)

## Tuple Snapshot (historical baseline)
- loop: `1a389a519516f55b96eaa436197f83f444517bd5`
- rlm-claude-code: `528f90018e0d464aa7e7459998191d8cfde27787`
- rlm-claude-code vendor/loop: `6779cdbc970c70f3ce82a998d6dcda59cd171560`
- loop-agent canonical: `2f4e762fbdb6fe40a00fe40b5df67b00b85dbb29`
- io-rflx: `abf11ca4069bac7a740508d02242114483a6cf51`
- loop-agent tuple mode: `clean_clone_committed`

## Gaps
- Claude Code adapter runtime and MCP handlers still include placeholder behavior in `rlm-core/src/adapters/claude_code/adapter.rs` and `rlm-core/src/adapters/claude_code/mcp.rs`; tracked as `loop-7fk` and `loop-3sj`.

## Verdict
- `VG-CONTRACT-001`: PASS with tracked follow-up gaps
