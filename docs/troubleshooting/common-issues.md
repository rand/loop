# Common Issues

For response sequencing and closure criteria, start with `incident-playbook.md` and use this file for pattern-specific fixes.

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
- Generated artifacts include `draft:` annotations when strict baseline output was expected

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

## 8. Ignored Lean/REPL integration tests hang or leave subprocesses behind

Symptoms:
- `cargo test ... -- --ignored` appears stuck
- Later runs show odd REPL startup errors
- Background `rlm_repl` or Lean `repl` processes linger

Actions:
1. Run ignored subprocess tests serially:
   `cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_repl_spawn -- --ignored --test-threads=1 && cargo test --no-default-features --features gemini test_lean_repl_spawn -- --ignored --test-threads=1`
2. Treat runtime > 120s as a failure signal and capture logs to evidence.
3. Scan for leftovers:
   `ps -axo pid=,command=,rss= -ww | rg -n "rlm_repl|lake env repl|\\brepl\\b" -S`
4. If environment prerequisites are missing (Python package path, Lean toolchain), record the actionable stderr and classify as environment blocker, not runtime success.
5. Re-run after fixing prerequisites; unattended runs should complete without manual process cleanup.

## 9. Proof automation reports "Missing proof state"

Symptoms:
- Tactic execution fails immediately with a deterministic missing proof-state error
- AI tactic validation returns failure before trying any tactic

Actions:
1. Ensure you are targeting a `sorry` location with a real `proof_state_id` (protocol session target selection prefers these).
2. Run the protocol path (`execute_tactic_with_feedback` / `execute_tactic_with_repl`) instead of ad hoc tactic calls without state.
3. Confirm operation IDs are stateful (`sorry:<proof_state>:<idx>`) rather than `sorry:missing:<idx>`.
4. If Lean response omitted proof state, treat as runtime blocker and capture the raw response in evidence.

Because gravity is also optional until it is not.
