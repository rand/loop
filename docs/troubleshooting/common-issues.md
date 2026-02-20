# Common Issues

## 1. `make check` fails with type/test errors

Symptoms:
- Nonzero exit
- Compiler or assertion failures

Actions:
1. Re-run `make check`.
2. Run failing test target directly for focused logs.
3. Fix root cause.
4. Re-run `make check` and governance gates.

## 2. `./scripts/dp ...` returns `ok: false`

Symptoms:
- JSON output contains failing step(s)

Actions:
1. Identify first failed step in JSON.
2. Run that command directly.
3. Resolve failure.
4. Re-run full `dp` chain.

## 3. Python binding build failures

Symptoms:
- `maturin develop` errors

Actions:
1. Verify Python and `uv` versions.
2. Re-run `uv sync`.
3. Ensure Rust build succeeds first.
4. Re-run binding install.

## 4. Go binding test failures

Symptoms:
- `go test ./...` fails in `rlm-core/go/rlmcore`

Actions:
1. Confirm static Rust library exists in expected path.
2. Rebuild Rust library.
3. Re-run Go tests.

## 5. Drift detection output appears unstable

Symptoms:
- Different drift ordering/results on repeated runs

Actions:
1. Re-run targeted sync/drift tests.
2. Verify input ordering assumptions.
3. Check symbol interning / temporary allocation code paths.
4. Capture deterministic repro fixture before patching.

## 6. Spec-agent output includes placeholders unexpectedly

Symptoms:
- Generated artifacts include `TODO` or `sorry`

Actions:
1. Verify `CompletenessMode` configuration.
2. Use `Baseline` for placeholder-free stubs.
3. Re-run formalization and tests.

## 7. "It works locally" but gates fail elsewhere

Symptoms:
- Local ad hoc run passes, policy gates fail

Actions:
1. Trust gate outputs over ad hoc confidence.
2. Capture exact environment details.
3. Align local commands with policy commands.
4. Re-run from clean state.

Because gravity is also optional until it is not.
