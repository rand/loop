# Evidence Storage Conventions

Store validation and investigation artifacts here so sessions stay reproducible without chat log bloat.

## Directory Layout

- `docs/execution-plan/evidence/<YYYY-MM-DD>/` for day-level artifacts.
- `docs/execution-plan/evidence/<YYYY-MM-DD>/milestone-<Mx>/` for milestone-specific artifacts.
- `docs/execution-plan/evidence/templates/` for reusable templates.

## Required Artifact Types

- Command outputs for each passed VG gate.
- Manual checklists for doc/contract gates.
- Benchmark result files for performance/efficacy gates.
- Short session summary note when multiple tasks are completed in one session.

## Naming Rules

- Use VG ID in filenames, for example `VG-LOOP-BUILD-001.txt`.
- For manual checks use `.md`, for structured metrics use `.json`.
- If rerunning a gate on same day, suffix with `-rN`.

## Minimal Metadata Header (for `.md` artifacts)

Use this at the top of manual artifacts:

```md
# <Artifact Title>
Date: YYYY-MM-DD
Task IDs: Mx-Tyy
VG IDs: VG-...
Command(s): ...
Result: pass | fail | blocked
Notes: ...
```

## Review Rule

A task is not complete until referenced evidence artifacts exist and are linked in session handoff.

