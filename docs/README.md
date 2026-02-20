# Loop Documentation

Welcome to the part of the project where we admit future-you exists.

This documentation is organized by audience and workflow, not by who happened to write the last markdown file at 2:13 AM.

## Choose Your Path

| If you are... | Start here | Then go to |
|---|---|---|
| New to Loop | [User Guide](./user-guide/README.md) | [Learning Paths](./user-guide/learning-paths.md) |
| Building features | [Developer Guide](./developer-guide/README.md) | [Workflow Cookbook](./developer-guide/workflow-cookbook.md) |
| Debugging internals | [Internals](./internals/README.md) | [Runtime Walkthrough](./internals/runtime-walkthrough.md) |
| Looking for architecture rationale | [Concepts](./concepts/README.md) | [System Principles](./concepts/principles.md) |
| Looking up exact commands | [Reference](./reference/README.md) | [Command Reference](./reference/command-reference.md) |

## Documentation Map

### User-Facing
- [User Guide](./user-guide/README.md): Start-to-finish guidance by skill level.
- [Learning Paths](./user-guide/learning-paths.md): Guided progression for beginners through power users.
- [Claude Code Adapter Guide](./user-guide/claude-code-adapter.md): Capability envelope, limits, and OODA behavior.
- [Workflow Recipes](./user-guide/workflow-recipes.md): End-to-end task playbooks.
- [Power User Playbook](./user-guide/power-user-playbook.md): Performance, scale, and control.

### Developer-Facing
- [Developer Guide](./developer-guide/README.md): Build and change Loop safely.
- [Setup](./developer-guide/setup.md): Environment bootstrap.
- [Workflow Cookbook](./developer-guide/workflow-cookbook.md): Change-type specific contribution runbooks.
- [Quality Gates](./developer-guide/quality-gates.md): Tests, checks, governance.
- [Contribution Workflow](./developer-guide/contribution-workflow.md): Branch-to-merge routine.

### Architecture and Internals
- [Concepts](./concepts/README.md): Mental models and design vocabulary.
- [System Principles](./concepts/principles.md): Decision criteria and tradeoff posture.
- [Internals](./internals/README.md): Runtime architecture and module boundaries.
- [Architecture](./internals/architecture.md): System structure and data flow.
- [OODA + Execution Flow](./internals/ooda-and-execution.md): Observe/Orient/Decide/Act mapping.
- [Runtime Walkthrough](./internals/runtime-walkthrough.md): Step-by-step request lifecycle trace.
- [Python Orchestrator Swap Analysis](./internals/python-orchestrator-swap-analysis.md): Tradeoffs and decision framework for full Python orchestrator migration.
- [Module Map](./internals/module-map.md): Where behavior actually lives.

### Operational and Recovery
- [Troubleshooting](./troubleshooting/README.md): Fast diagnosis and fix paths.
- [Incident Playbook](./troubleshooting/incident-playbook.md): Severity-based response and closure criteria.
- [Common Issues](./troubleshooting/common-issues.md): Known failure patterns.
- [Diagnostics Checklist](./troubleshooting/diagnostics-checklist.md): Structured incident capture.

### Reference
- [Reference Index](./reference/README.md): Quick lookup section.
- [Command Reference](./reference/command-reference.md): Canonical command map by workflow.

### Reference and Specifications
- [Documentation Architecture](./DOCS-ARCHITECTURE.md): How docs are structured and maintained.
- [Specification Set](./spec/): Canonical feature contracts (SPEC-20 through SPEC-27).
- [Spec Lineage Status](./spec/SPEC-LINEAGE-STATUS.md): How origin-era design/migration specs map to current live status.
- [Execution Plan](./execution-plan/README.md): Program-level planning, evidence, and governance.
- [Architecture Decisions](./adr/): Long-lived technical decisions and context.
- [Glossary](./glossary.md): Shared terms and definitions.
- [Writing Style](./writing-style.md): Documentation voice and formatting contract.

## Documentation Principles

1. Behavior over aspiration: document what exists and runs.
2. Workflow over component: prefer "how to get outcome X" over "here is a list of modules".
3. Reproducibility over vibes: include concrete commands and expected outputs.
4. Honesty over smoothness: if something is sharp, say so plainly.
5. Friendly precision: direct writing, with enough personality that reading it does not feel like filing taxes.

## Suggested Reading Order

1. [Concepts: Mental Model](./concepts/mental-model.md)
2. [Concepts: System Principles](./concepts/principles.md)
3. [User Guide](./user-guide/README.md)
4. [Developer Setup](./developer-guide/setup.md)
5. [Internals Architecture](./internals/architecture.md)
6. [Troubleshooting](./troubleshooting/README.md)

## Scope Note

The docs in this folder are the operational surface. Deep design history and migration rationale still live in long-form docs such as:
- `docs/unified-rlm-library-design.md`
- `docs/lean-formal-verification-design.md`
- `docs/migration-spec-recurse.md`
- `docs/migration-spec-rlm-claude-code.md`

Those are excellent references; they are not where a newcomer should start unless they really enjoy scrolling.

For live implementation status and active backlog, use:
- `bd status` / `bd ready`
- `docs/execution-plan/STATUS.md`
- `docs/execution-plan/TASK-REGISTRY.md`
