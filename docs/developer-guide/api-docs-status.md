# API Documentation Status

This page is the source of truth for what "documented" means in this repository.

## Current State (2026-02-20)

- Module-level docs: complete baseline.
- Workflow/docs/navigation guides: complete baseline.
- Item-level rustdoc coverage for every public symbol: partial and incremental.

That means the project is navigable and operable, but not every exported method has bespoke prose yet.
No hand-waving, no cosplay as "finished."

## Enforcement Contract

Use these commands for docs quality checks:

```bash
cd /Users/rand/src/loop/rlm-core
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Interpretation:
- Build succeeds: no rustdoc warnings.
- Build fails: fix docs or signatures before merge.

## Policy

1. Never claim "all public APIs documented" unless item-level rustdoc coverage is actually complete.
2. Module-level docs are mandatory.
3. Item-level docs are required for newly introduced public API in the same change set.
4. Legacy public API doc depth is improved incrementally, prioritized by high-use surfaces.

## Practical Rule of Thumb

- If users copy/paste from it: document it now.
- If only a debugger should ever see it: document it soon.
- If nobody can explain it without opening three files: document it yesterday.
