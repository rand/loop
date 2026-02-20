# Quality Gates

Loop uses layered quality gates: local correctness, coverage proof, policy enforcement, and release-grade verification.

## Gate Layers

### Layer 1: Local correctness

```bash
make check
```

Runs:
- Type checks
- Tests

### Layer 2: Coverage proof (CI-backed, locally runnable when tooling is available)

```bash
make coverage
```

Runs:
- `scripts/run_coverage.sh`
- `cargo llvm-cov` line-coverage gate (`COVERAGE_MIN_LINES`, default `80`)
- Artifact output: `coverage/lcov.info`, `coverage/summary.txt`

Notes:
- If `cargo-llvm-cov` is missing locally, the script exits with actionable install guidance.
- GitHub Actions workflow `rlm-core-coverage.yml` is the canonical enforcement path.

### Layer 3: Governance review

```bash
./scripts/dp review --json
./scripts/dp verify --json
```

Purpose:
- Consistent, machine-readable status
- Standardized workflow enforcement

### Layer 4: Enforcement gates

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
3. Before push: `make coverage` (or verify CI coverage gate pass when local tool install is blocked).
4. Before push: full `dp` enforcement chain.

## Evidence Logging

For major changes, capture outputs into:
- `docs/execution-plan/evidence/<date>/<scope>/...`

This makes reviews faster and postmortems less fictional.
