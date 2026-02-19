# Safe Mode Policy (Laptop OOM Prevention)

This policy is mandatory for this machine after the prior out-of-memory crash.

## Safety Goals

- Prevent memory exhaustion and system instability.
- Keep execution progress deterministic and recoverable.
- Trade peak throughput for reliability.

## Default Topology

- 1 orchestrator thread.
- 1 active worker thread.
- Additional lanes may run read-only analysis only (no heavy commands).

Heavy command concurrency target: exactly 1.

## Heavy Command Definition

Treat these as heavy commands:

- `cargo check`, `cargo test`, `rustc`
- `pytest`, `uv run pytest`
- `maturin` builds

## Mandatory Wrapper

All heavy commands must run through:

- `/Users/rand/src/loop/scripts/safe_run.sh`

Wrapper controls:

- Global lock to prevent concurrent heavy commands.
- Available-memory threshold check before execution.

Default threshold:

- `LOOP_MIN_AVAILABLE_MIB=2048`

Recommended threshold for safer operation:

- `LOOP_MIN_AVAILABLE_MIB=3072`

## Preflight Before Any Heavy Work

Run:

1. `vm_stat | head -n 25`
2. `memory_pressure | head -n 40`
3. Confirm no unrelated heavy local tasks are running.

If available memory is tight, pause and reduce local load before continuing.

## Hard Stop Conditions

Stop all heavy work immediately when any occurs:

- Wrapper refuses admission due low memory.
- Frequent OS-level memory-pressure alerts appear.
- Large swap activity appears during task execution.
- UI responsiveness degrades materially.

## Recovery Sequence

1. Stop active heavy commands.
2. Preserve current logs/artifacts.
3. Close non-essential local processes.
4. Resume with a single smallest-scope task.
5. Re-run preflight.

## Lane Rules in Safe Mode

- Lane A may run heavy commands.
- Lane B and C remain no-heavy (docs/analysis only) until Lane A is idle.
- Orchestrator serializes all gates requiring heavy commands.

## Evidence Requirement

Record safe-mode operation in evidence artifacts:

- Threshold used.
- Wrapper admission line.
- Any memory-related aborts and mitigations.

