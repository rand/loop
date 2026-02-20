# Internals

This section explains how Loop behaves internally, where major responsibilities live, and how data moves through the system.

## Start Here

1. [Architecture](./architecture.md)
2. [OODA and Execution](./ooda-and-execution.md)
3. [Module Map](./module-map.md)

## Audience

- Contributors debugging deep runtime behavior
- Maintainers reviewing performance/correctness tradeoffs
- Integration authors mapping external surfaces to internal modules

## What You Will Not Find Here

- Full feature specs (see `docs/spec/`)
- Long-form migration rationale (see `docs/migration-spec-*.md`)
- Session-level work plans (see `docs/execution-plan/`)

This is the "how it works now" view.
