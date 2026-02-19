# Execution Orchestration (Safe Mode Default)

This file defines how to execute the plan safely on this laptop after a prior OOM crash.

## Operating Mode

Default mode: safe mode.

- 1 orchestrator thread (control plane)
- 1 active worker thread (execution)
- Additional workers may do read-only analysis only

## Hard Safety Controls

- Heavy commands are serialized (concurrency = 1).
- Heavy commands must run via `/Users/rand/src/loop/scripts/safe_run.sh`.
- Recommended threshold: `LOOP_MIN_AVAILABLE_MIB=3072`.

See `docs/execution-plan/safe-mode/SAFETY-POLICY.md` for full details.

## Thread Roles

### Orchestrator Thread

Responsibilities:

- Owns sequencing decisions and dependency enforcement.
- Owns updates to:
- `docs/execution-plan/STATUS.md`
- `docs/execution-plan/TASK-REGISTRY.md`
- `docs/execution-plan/DECISIONS.md`
- `docs/execution-plan/WORKBOARD.md`
- Reviews worker evidence and marks tasks done/blocked.
- Enforces safe mode and stop conditions.

### Worker Threads

Responsibilities:

- Execute only assigned task IDs.
- Modify only files in assigned scope.
- Run required validation gates.
- Store artifacts under `docs/execution-plan/evidence/`.
- Return compact handoff packet.

Workers must not edit orchestrator-owned tracker files.

## Claim and Handoff Protocol

1. Orchestrator assigns explicit task IDs to one lane.
2. Worker executes and validates (using safe wrapper for heavy commands).
3. Worker returns handoff packet:
- Completed task IDs
- VG IDs run
- Evidence artifact paths
- Open blockers
4. Orchestrator reviews and updates trackers.

## Conflict Avoidance Rules

- One task ID can only be active in one lane.
- If two lanes need same file, orchestrator serializes that work.
- Prefer small PR-sized deltas per worker handoff.

## Sub-Agent Policy

Use sub-agents only for bounded work inside a lane:

- test-failure categorization
- focused spec/runtime diff checks
- log summarization

Do not use sub-agents for broad multi-repo edits.

## Escalation Triggers

Escalate immediately when any occurs:

- A decision in `DECISIONS.md` must change.
- A change risks breaking `rlm-claude-code` compatibility.
- A required VG gate cannot run.
- Safe wrapper rejects heavy command due memory threshold.
- Another lane is touching required files.

## Success Criteria

- No machine instability or memory exhaustion.
- Completed tasks include deterministic evidence.
- Tracker files remain coherent and minimal.

