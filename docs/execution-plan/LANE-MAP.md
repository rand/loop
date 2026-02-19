# Lane Map

This map defines lane ownership and safe-mode activation rules.

## Safe Mode Activation

Default on this machine:

- Lane C: active execution lane (heavy work allowed via wrapper)
- Lane A: read-only maintenance only
- Lane B: read-only maintenance only

Lane A/B can run heavy commands only when orchestrator explicitly marks them active in `WORKBOARD.md`.

## Lane A - M3 Maintenance (Standby)

Scope:

- post-completion fixes for `M1`, `M2`, `M3` only (regression-only)

Task order:

1. Regression-only reopen tasks as needed

Primary file areas:

- `/Users/rand/src/loop/rlm-core/src/`
- `/Users/rand/src/loop/rlm-core/python/`
- `/Users/rand/src/loop/docs/spec/`
- `/Users/rand/src/loop/docs/execution-plan/`

## Lane B - M1/M2 Maintenance (Standby)

Scope:

- post-completion fixes for `M1`, `M2` only (regression-only)

Task order:

1. M1/M2 regressions only, if reopened by validation

Primary file areas:

- `/Users/rand/src/loop/rlm-core/src/`
- `/Users/rand/src/loop/rlm-core/python/`

Start condition:

- Lane A blocked or waiting on external input.
- Orchestrator marks Lane B active in `WORKBOARD.md`.

## Lane C - Consumer Integration (Active)

Scope:

- `M4` consumer integration tracks
- `M5` benchmark/efficacy prep
- `M6` release governance prep
- Post-M6 steady-state compatibility/spec governance cadence

Task order:

1. `M4-T01`
2. `M4-T02`
3. `M4-T03`
4. `M4-T04`
5. `M5-T01` (prep)
6. `M5-T02` (prep)
7. `M6-T01`
8. `M6-T02`
9. `M6-T03`
10. cadence-driven maintenance runs

Primary file areas:

- `/Users/rand/src/rlm-claude-code/`
- `/Users/rand/src/loop-agent/`
- `/Users/rand/src/io-rflx/`
- `/Users/rand/src/loop/docs/execution-plan/contracts/`

Start condition:

- M3 closure complete.
- Decisions D-001..D-007 accepted.
- Orchestrator marks Lane C active in `WORKBOARD.md`.

## Orchestrator-Only Files

Only orchestrator updates these:

- `/Users/rand/src/loop/docs/execution-plan/STATUS.md`
- `/Users/rand/src/loop/docs/execution-plan/TASK-REGISTRY.md`
- `/Users/rand/src/loop/docs/execution-plan/DECISIONS.md`
- `/Users/rand/src/loop/docs/execution-plan/WORKBOARD.md`
