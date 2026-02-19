# Lane Map

This map defines lane ownership and safe-mode activation rules.

## Safe Mode Activation

Default on this machine:

- Lane A: active execution lane for M7 runtime closure (heavy work allowed via wrapper)
- Lane B: read-only by default (docs/governance lane)
- Lane C: cadence + consumer lane (heavy work only when Lane A is idle)

Lane A is the default heavy lane for M7; Lane B and Lane C may run heavy commands only when orchestrator explicitly marks them active in `WORKBOARD.md`.

## Lane A - M7 Core Runtime Closure (Active)

Scope:

- `M7-T01` through `M7-T08`
- Core runtime/spec closure in `rlm-core` and `rlm-core/python`

Task order:

1. `M7-T01` SPEC-26 batch runtime closure
2. `M7-T02` SPEC-27 fallback wiring
3. `M7-T03` SPEC-20 typed-signature parity
4. `M7-T04` SPEC-21 dual-model integration
5. `M7-T05` SPEC-22 proof closure
6. `M7-T06` SPEC-23 visualization closure
7. `M7-T07` SPEC-24 optimizer closure
8. `M7-T08` SPEC-25 context externalization closure

Primary file areas:

- `/Users/rand/src/loop/rlm-core/src/`
- `/Users/rand/src/loop/rlm-core/python/`
- `/Users/rand/src/loop/docs/spec/`
- `/Users/rand/src/loop/docs/execution-plan/`

## Lane B - M7 Docs/Governance Reconciliation (Standby)

Scope:

- `M7-T10` spec/governance reconciliation
- documentation-only maintenance and traceability refreshes

Task order:

1. Prep reconciliation checklist while Lane A executes runtime tasks
2. Execute `M7-T10` after `M7-T09` completion

Primary file areas:

- `/Users/rand/src/loop/docs/spec/`
- `/Users/rand/src/loop/docs/execution-plan/`

Start condition:

- Lane A has completed implementation-heavy tasks or is blocked.
- Orchestrator marks Lane B active in `WORKBOARD.md`.

## Lane C - Consumer Cadence and Interop Follow-up (Conditional)

Scope:

- Ongoing Ops-Weekly compatibility cadence
- `M7-T09` io-rflx adapter fixture + calibration task

Task order:

1. Ops-Weekly cadence runs (steady-state)
2. `M7-T09` once `M7-T08` is done
3. Resume cadence ownership after M7 closure

Primary file areas:

- `/Users/rand/src/rlm-claude-code/`
- `/Users/rand/src/loop-agent/`
- `/Users/rand/src/io-rflx/`
- `/Users/rand/src/loop/docs/execution-plan/contracts/`

Start condition:

- Lane A is idle or explicitly paused.
- Orchestrator marks Lane C heavy execution window in `WORKBOARD.md`.

## Orchestrator-Only Files

Only orchestrator updates these:

- `/Users/rand/src/loop/docs/execution-plan/STATUS.md`
- `/Users/rand/src/loop/docs/execution-plan/TASK-REGISTRY.md`
- `/Users/rand/src/loop/docs/execution-plan/DECISIONS.md`
- `/Users/rand/src/loop/docs/execution-plan/WORKBOARD.md`
