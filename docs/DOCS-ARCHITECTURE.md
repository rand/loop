# Documentation Architecture

This document defines how documentation in Loop is organized, written, and maintained.

If this file is doing its job, nobody needs to ask "where should this live?" during review.

## Goals

1. Make the project operable for newcomers and maintainers.
2. Keep behavior docs close to runtime truth.
3. Provide reliable pathways by role, skill level, and workflow.
4. Keep writing precise, human, and useful under pressure.

## Audience Map

| Audience | Primary docs | What success looks like |
|---|---|---|
| New user | `docs/user-guide/quickstart.md`, `docs/user-guide/learning-paths.md` | Can run first successful checks and avoid common footguns |
| Returning user | `docs/user-guide/workflow-recipes.md`, `docs/reference/command-reference.md` | Can select and run the right workflow quickly |
| Power user | `docs/user-guide/power-user-playbook.md` | Can tune depth, cost, and assurance for high-stakes work |
| Contributor | `docs/developer-guide/setup.md`, `docs/developer-guide/workflow-cookbook.md` | Can ship changes with tests, docs, and evidence |
| Maintainer | `docs/developer-guide/quality-gates.md`, `docs/troubleshooting/incident-playbook.md` | Can diagnose failures and land safe changes predictably |
| Runtime debugger | `docs/internals/architecture.md`, `docs/internals/runtime-walkthrough.md`, `docs/internals/module-map.md` | Can trace behavior from symptom to module |
| Architecture reader | `docs/concepts/mental-model.md`, `docs/concepts/principles.md`, `docs/adr/` | Can explain why system tradeoffs exist |

## Workflow Coverage

| Workflow | Start | Deep dive | Closeout |
|---|---|---|---|
| First run | `docs/user-guide/quickstart.md` | `docs/user-guide/learning-paths.md` | `docs/troubleshooting/common-issues.md` |
| Feature development | `docs/developer-guide/workflow-cookbook.md` | `docs/developer-guide/quality-gates.md` | `docs/developer-guide/contribution-workflow.md` |
| Adapter behavior changes | `docs/user-guide/claude-code-adapter.md` | `docs/internals/ooda-and-execution.md` | `docs/developer-guide/quality-gates.md` |
| Spec and formalization work | `docs/user-guide/workflow-recipes.md` | `docs/spec/`, `docs/internals/runtime-walkthrough.md` | `docs/execution-plan/VALIDATION-MATRIX.md` |
| Incident triage | `docs/troubleshooting/incident-playbook.md` | `docs/troubleshooting/diagnostics-checklist.md` | `docs/troubleshooting/common-issues.md` |
| Command lookup | `docs/reference/command-reference.md` | `Makefile`, `scripts/` | `docs/execution-plan/evidence/` |

## Document Types

1. Orientation docs:
   - Explain where to start and how to choose a path.
   - Examples: `docs/README.md`, section `README.md` files.
2. Procedure docs:
   - Provide commands, success criteria, and failure protocol.
   - Examples: user recipes, developer workflow cookbook, quality gates.
3. Concept docs:
   - Explain mental model and tradeoff logic.
   - Examples: `docs/concepts/*`.
4. Internals docs:
   - Explain runtime behavior, module boundaries, and data flow.
   - Examples: `docs/internals/*`.
5. Troubleshooting docs:
   - Turn symptoms into deterministic diagnosis and fixes.
   - Examples: `docs/troubleshooting/*`.
6. Reference docs:
   - Fast lookup material, minimal narrative.
   - Examples: `docs/reference/*`, `docs/glossary.md`.

## Quality Contract

Every operational document should satisfy:

1. Explicit target reader.
2. Concrete commands for procedures.
3. Clear expected outcomes.
4. Link to adjacent docs instead of copy-paste duplication.
5. No typographic em dash punctuation.
6. No aspirational claims without verification path.

## Change Protocol

When behavior changes:

1. Update code and docs in the same change set.
2. Update the nearest section index if navigation changes.
3. Record validation evidence for nontrivial claims.
4. Re-run docs style checks (`make docs-check`).

When docs structure changes:

1. Update `README.md` at repo root.
2. Update `docs/README.md`.
3. Update affected section `README.md`.
4. Add at least one cross-link from an existing high-traffic page.

## Anti-Patterns

1. "TBD" as permanent architecture.
2. Procedures without success criteria.
3. Claims that cannot be validated.
4. New docs that are not linked from any index page.
5. Deleting old docs without redirecting readers to replacement paths.

## Ownership

- System-wide docs ownership is shared by active maintainers.
- Every merged behavior change is responsible for its own documentation updates.
- During review, "works in code but not in docs" counts as incomplete.

Documentation is part of the runtime interface. It just compiles in human brains instead of cargo.
