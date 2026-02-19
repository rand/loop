# Session Runbook (Codex / Claude Code)

## Objective

Execute one bounded task per session with deterministic validation and low context usage.

## Parallel Mode

- Use topology and rules from `docs/execution-plan/ORCHESTRATION.md`.
- Use assignments from `docs/execution-plan/LANE-MAP.md`.
- Enforce `docs/execution-plan/safe-mode/SAFETY-POLICY.md`.
- Launch prompts from `docs/execution-plan/THREAD-STARTER-PACK.md`.
- In parallel mode, only the orchestrator thread updates tracker files.

## Session Start (Required)

1. Open `docs/execution-plan/STATUS.md`.
2. Open `docs/execution-plan/TASK-REGISTRY.md` and select the highest-priority unblocked task.
3. Confirm lane assignment from `docs/execution-plan/LANE-MAP.md`.
4. Run safe-start checks from `docs/execution-plan/safe-mode/SAFE-START-CHECKLIST.md`.
5. Open only the milestone file for that task.
6. Confirm prerequisite tasks and required decisions are satisfied.
7. Confirm required validation gates from `docs/execution-plan/VALIDATION-MATRIX.md`.

## Task Execution Pattern

1. Restate task ID and acceptance criteria.
2. Implement only changes scoped to that task.
3. For heavy commands, use `/Users/rand/src/loop/scripts/safe_run.sh`.
4. Run required validation gates.
5. Store logs/artifacts under `docs/execution-plan/evidence/`.
6. If orchestrator: update `STATUS.md` and `TASK-REGISTRY.md`.

## Session End (Required)

1. Ensure all required gates for completed tasks passed.
2. Write or update evidence artifacts.
3. Update task status fields in `TASK-REGISTRY.md`.
4. Update blockers and next queue in `STATUS.md`.
5. If architecture/API choices changed, append to `DECISIONS.md`.
6. Emit the handoff template from `STATUS.md`.

## Context Controls

- Do not load all milestone files in one session.
- Do not paste long command output into chat.
- Keep raw logs in evidence files; keep chat summaries short.
- Use task IDs and VG IDs in all updates.

## Escalation Conditions

Escalate immediately when any occurs:

- A change would break `rlm-claude-code` public usage.
- A task requires changing previously accepted decision(s).
- Validation gates cannot be run in current environment.
- A cross-repo contract is ambiguous.

Escalation output must include:

1. Failing invariant or ambiguous contract.
2. Options with tradeoffs.
3. Recommended option.

## Task Done Definition

A task is done only if all are true:

- Acceptance criteria met.
- Required validation gates passed.
- Evidence artifacts written.
- Task and status trackers updated.
