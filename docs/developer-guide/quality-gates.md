# Quality Gates

Loop uses layered quality gates: local correctness, policy enforcement, and release-grade verification.

## Gate Layers

### Layer 1: Local correctness

```bash
make check
```

Runs:
- Type checks
- Tests

### Layer 2: Governance review

```bash
./scripts/dp review --json
./scripts/dp verify --json
```

Purpose:
- Consistent, machine-readable status
- Standardized workflow enforcement

### Layer 3: Enforcement gates

```bash
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Purpose:
- Block policy violations before commit/push boundaries

## Required Interpretation Rules

1. Exit code is authoritative.
2. JSON `ok` is authoritative.
3. Warnings are not failures, but they are not decorative either.

## Failure Protocol

When a gate fails:
1. Re-run same command to confirm.
2. Isolate first failing step.
3. Fix root cause.
4. Re-run full gate chain.

No partial-pass narratives.

## Suggested Daily Rhythm

1. During iteration: targeted tests.
2. Before commit: `make check`.
3. Before push: full `dp` enforcement chain.

## Evidence Logging

For major changes, capture outputs into:
- `docs/execution-plan/evidence/<date>/<scope>/...`

This makes reviews faster and postmortems less fictional.
