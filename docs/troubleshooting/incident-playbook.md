# Incident Playbook

Use this playbook when checks fail, behavior regresses, or integration confidence drops.

This is for live response, not post-fact storytelling.

## Severity Levels

| Level | Definition | Response target |
|---|---|---|
| S1 | Release-blocking or correctness-critical failure | Start triage immediately |
| S2 | Significant workflow disruption with workaround | Start triage within same work session |
| S3 | Localized failure with low blast radius | Triage before merge or handoff |

## First 15 Minutes

1. Re-run the failing command exactly.
2. Capture full output to an artifact file.
3. Record:
   - exit code
   - branch and commit
   - working directory
   - elapsed wall time if hung
4. Classify scope:
   - environment/toolchain
   - build/type
   - test regression
   - governance policy
   - runtime behavior
   - integration compatibility

Suggested command template:

```bash
ts=$(date -u +"%Y-%m-%dT%H-%M-%SZ")
mkdir -p docs/execution-plan/evidence/$ts/incident
<failing command> > docs/execution-plan/evidence/$ts/incident/failure.txt 2>&1
echo $? > docs/execution-plan/evidence/$ts/incident/exit_code.txt
```

## 15 to 45 Minutes

1. Isolate first failing step.
2. Build minimal reproducible sequence.
3. Identify probable module ownership.
4. Write one-paragraph root-cause hypothesis.
5. Apply the smallest safe fix.

Helpful commands:

```bash
git status --short --branch
git rev-parse HEAD
rg -n "<key symbol or error token>" rlm-core/src docs
```

## 45+ Minutes or Repeated Failure

Escalate quality, not panic.

1. Run full relevant gate chain for your change type.
2. Compare against known issue patterns in `common-issues.md`.
3. Complete `diagnostics-checklist.md`.
4. Capture a short incident summary with:
   - symptom
   - root cause
   - fix
   - validation commands
   - evidence paths

## Mandatory Validation Before Closure

1. Re-run original failing command.
2. Re-run full expected gate sequence.
3. Confirm no collateral regressions.
4. Update docs if behavior changed.
5. Link evidence artifacts in issue notes or handoff.

## Fast Decision Tree

If `make check` fails:
- Fix code/test regression first.
- Then run `./scripts/dp verify --json`.

If `dp` enforcement fails:
- Trust the first failing JSON step.
- Fix that step, then rerun full chain.

If adapter scenario gate fails:
- Run `make claude-adapter-gate`.
- Inspect adapter signal and mode decision paths.

If subprocess tests hang:
- Run `make ignored-repl-gate`.
- Scan for leftover REPL/Lean processes.

## Communication Template

Use this exact shape for handoff:

1. Incident: one-sentence symptom.
2. Scope: primary failure domain.
3. Root cause: one paragraph.
4. Fix: what changed and where.
5. Validation: commands and pass/fail.
6. Evidence: artifact paths.
7. Remaining risk: explicit and bounded.

## Exit Criteria

Incident is closed only when:

1. Repro is gone.
2. Gates pass.
3. Docs match behavior.
4. Evidence is recorded.

Everything else is progress, not closure.
