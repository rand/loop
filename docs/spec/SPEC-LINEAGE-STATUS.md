# Specification Lineage Status (Origin to Present)

Last reconciled: 2026-02-20

This document is the "don't panic" map for readers who open older design/migration specs and see ambitious target lists.

## Canonical Live Truth

Use these as source of truth for current status and active backlog:

- `bd status` / `bd ready`
- [`docs/execution-plan/STATUS.md`](../execution-plan/STATUS.md)
- [`docs/execution-plan/TASK-REGISTRY.md`](../execution-plan/TASK-REGISTRY.md)
- [`docs/execution-plan/WORKBOARD.md`](../execution-plan/WORKBOARD.md)

## Historical Spec Map

| Document | Role in lineage | How to read checklist lines |
|---|---|---|
| [`docs/implementation-roadmap.md`](../implementation-roadmap.md) | Original sequencing model for rlm-core + formal verification rollout | `[historical target]` = archived target from planning era, not active backlog |
| [`docs/unified-rlm-library-design.md`](../unified-rlm-library-design.md) | Foundational architecture intent and phase plan | `[historical target]` = archival design target |
| [`docs/lean-formal-verification-design.md`](../lean-formal-verification-design.md) | Lean/spec-agent system design intent | `[historical target]` = archival acceptance target |
| [`docs/migration-spec-rlm-claude-code.md`](../migration-spec-rlm-claude-code.md) | Migration target-state design plus later reality adjustments | `[historical target]` = original migration target; see migration reality and Beads for actual achieved scope |
| [`docs/migration-spec-recurse.md`](../migration-spec-recurse.md) | recurse migration planning artifact | `[historical target]` = archived phase exit target; authoritative outcome is tracked in Beads and execution-plan status |

## Operational Checklists vs Backlog

Not every unchecked box means "unfinished engineering work."

- [`docs/troubleshooting/diagnostics-checklist.md`](../troubleshooting/diagnostics-checklist.md) is intentionally a blank incident-response form.
- It uses unchecked boxes by design and should not be interpreted as implementation debt.

## Residual Scope Boundary

Where historical target state exceeds currently exposed APIs (for example full Python-orchestrator replacement surfaces not exposed in bindings), status is intentionally documented as out-of-scope/deferred in the migration specs and execution-plan records, not silently treated as complete.
