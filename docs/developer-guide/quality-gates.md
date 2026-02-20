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
- `cargo llvm-cov` line-coverage gate (`COVERAGE_MIN_LINES`, default `70`)
- Artifact output: `coverage/lcov.info`, `coverage/summary.txt`

Notes:
- If `cargo-llvm-cov` is missing locally, the script exits with actionable install guidance.
- GitHub Actions workflow `rlm-core-coverage.yml` is the canonical enforcement path.
- Coverage CI bootstraps `rlm-core/python/.venv` and installs `rlm-core/python` so REPL-backed Rust tests run with deterministic Python dependencies.

### Layer 2.5: API docs contract

```bash
make rustdoc-check
```

Purpose:
- Keep public docs buildable and warning-free.
- Catch broken intra-doc links and rustdoc lint regressions early.

### Layer 2.6: Python integration compatibility gate

```bash
make py-integration-gate
```

Purpose:
- Validate package-level compatibility helpers (`version`, `version_tuple`, `has_feature`, `available_features`).
- Reject false-green all-skipped/no-tests-ran outcomes.

### Layer 2.7: Ignored subprocess integration stability

```bash
make ignored-repl-gate
```

Purpose:
- Ensure ignored REPL/Lean subprocess integration tests complete deterministically in unattended runs.
- Catch orphan-process cleanup regressions early.

### Layer 2.8: Property-based invariant gate

```bash
make proptest-gate
```

Purpose:
- Enforce invariant-level proptest coverage across epistemic math, signature validation, fallback behavior, and router/accounting logic.
- Run with deterministic proptest configuration (`PROPTEST_CASES=96`, `PROPTEST_RNG_SEED=424242`, `PROPTEST_RNG_ALGORITHM=cc`) so CI/local results are reproducible.
- Fail fast if any scoped property-test suite accidentally runs zero tests (false-green guardrail).

### Layer 2.9: Claude adapter end-to-end efficacy gate

```bash
make claude-adapter-gate
```

Purpose:
- Validate realistic Claude adapter observe/orient/decide/act scenarios, not just activation plumbing.
- Enforce scenario-level quality assertions (context observation, signal-driven mode choice, execution/accounting outputs).
- Guard against false green from test-filter drift by requiring at least two scenario tests to execute.

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
4. Before push: `make rustdoc-check`.
5. Before push: `make py-integration-gate` when Python bindings or compatibility surface changed.
6. Before push: `make proptest-gate`.
7. Before push: `make claude-adapter-gate`.
8. Before push (release-grade subprocess changes): `make ignored-repl-gate`.
9. Before push: full `dp` enforcement chain.

## Evidence Logging

For major changes, capture outputs into:
- `docs/execution-plan/evidence/<date>/<scope>/...`

This makes reviews faster and postmortems less fictional.
