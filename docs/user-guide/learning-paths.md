# Learning Paths

This guide helps you choose the shortest route to useful outcomes based on your current level and job.

## Pick a Path

| You are here | Start with | Time budget | Finish line |
|---|---|---|---|
| First-day user | `quickstart.md` | 30-45 minutes | Core checks pass and repo is runnable |
| Working contributor | `workflow-recipes.md` | 45-90 minutes | You can run the right gate sequence for your task |
| Power operator | `power-user-playbook.md` | 60-120 minutes | You can tune depth, cost, and evidence posture |
| Claude adapter user | `claude-code-adapter.md` | 30-60 minutes | You understand activation behavior and guardrails |

## Path 1: First-Day User

Goal: move from clone to confidence without accidental chaos.

1. Complete `quickstart.md`.
2. Run:

```bash
make check
./scripts/dp review --json
```

3. If either fails, use:
   - `../troubleshooting/common-issues.md`
   - `../troubleshooting/diagnostics-checklist.md`

Success criteria:
- `make check` exits `0`.
- `review` JSON reports `"ok": true`.

## Path 2: Working Contributor

Goal: ship feature or fix work safely.

1. Read `workflow-recipes.md`.
2. Use the recipe that matches your current job.
3. Confirm final gate sequence:

```bash
make check
./scripts/dp verify --json
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Success criteria:
- Command outputs and exit codes indicate pass.
- You can explain what each gate protected.

## Path 3: Power Operator

Goal: optimize correctness, cost, and throughput under real constraints.

1. Read `power-user-playbook.md`.
2. Add evidence-first workflow to daily practice.
3. For high-risk work, add targeted gate runs:

```bash
make rustdoc-check
make proptest-gate
make claude-adapter-gate
```

Success criteria:
- You choose assurance depth intentionally, not by habit.
- High-risk claims are backed by artifact paths, not memory.

## Path 4: Claude Adapter User

Goal: understand when adapter behavior is fast, deep, or intentionally conservative.

1. Read `claude-code-adapter.md`.
2. Validate adapter scenarios:

```bash
make claude-adapter-gate
```

3. If behavior surprises you, inspect:
   - `../internals/ooda-and-execution.md`
   - `../internals/runtime-walkthrough.md`

Success criteria:
- You can predict activation and mode decisions for common prompt types.
- You can locate the runtime modules that produced a decision.

## Suggested Progression

1. `quickstart.md`
2. `workflow-recipes.md`
3. `claude-code-adapter.md` (if relevant)
4. `power-user-playbook.md`
5. `../reference/command-reference.md`

If you skip steps, the project will not punish you. It will just do it indirectly through failed gates.
