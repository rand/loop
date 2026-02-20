# Workflow Cookbook

These are practical contribution paths by change type.

Pick the recipe that matches what you changed. Run the full gate chain before pretending you are done.

## Recipe 1: Docs-Only Change

Use when:
- You changed Markdown only and did not alter behavior.

Commands:

```bash
bd ready
bd show <issue-id>
bd update <issue-id> --status in_progress
make docs-check
./scripts/dp review --json
```

Success criteria:
- Docs style check passes.
- Governance review returns `"ok": true`.
- Updated docs are linked from relevant index pages.

## Recipe 2: Core Rust Behavior Change

Use when:
- You changed files under `rlm-core/src`.

Commands:

```bash
bd ready
bd show <issue-id>
bd update <issue-id> --status in_progress
make check
make rustdoc-check
make proptest-gate
./scripts/dp review --json
./scripts/dp verify --json
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Success criteria:
- Tests and gates pass.
- Behavior docs updated in same change set.
- Evidence captured for nontrivial risk claims.

## Recipe 3: Claude Adapter Change

Use when:
- You changed `rlm-core/src/adapters/claude_code/*`.
- You touched complexity signals or mode-routing behavior.

Commands:

```bash
make check
make claude-adapter-gate
./scripts/dp verify --json
```

Also review:
- `../user-guide/claude-code-adapter.md`
- `../internals/ooda-and-execution.md`

Success criteria:
- Scenario-level adapter gate passes.
- Docs reflect behavior and limits accurately.

## Recipe 4: Python Binding or REPL Change

Use when:
- You changed `rlm-core/python/*` or REPL integration paths.

Commands:

```bash
make check
make py-integration-gate
cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q
```

For subprocess-sensitive changes:

```bash
make ignored-repl-gate
```

Success criteria:
- Python compatibility helpers still behave correctly.
- Ignored REPL tests complete without orphan process leftovers.

## Recipe 5: Go Binding Change

Use when:
- You changed `rlm-core/go/*` or FFI surfaces used by Go.

Commands:

```bash
cd /Users/rand/src/loop/rlm-core
cargo build --release --lib --no-default-features --features tokio-runtime
cd /Users/rand/src/loop/rlm-core/go/rlmcore
go test ./...
```

Success criteria:
- Go tests pass against current Rust static library.
- Any API behavior changes are documented.

## Recipe 6: Spec and Contract Surface Change

Use when:
- You changed contract semantics, spec-agent behavior, or compatibility claims.

Commands:

```bash
make check
./scripts/dp verify --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Then validate against:
- `../spec/`
- `../execution-plan/VALIDATION-MATRIX.md`
- `../execution-plan/contracts/`

Success criteria:
- Contract docs and runtime behavior are aligned.
- Evidence paths exist for compatibility claims.

## Landing the Session

Required closeout sequence:

```bash
git status
git pull --rebase
bd sync
git add ...
git commit -m "docs: <summary>"   # or appropriate type
git push
git status
```

Success criteria:
- No stranded local work.
- Branch up to date with remote.
- Issue closed with reason:

```bash
bd close <issue-id> --reason "implemented + verified"
```

If this feels strict, that is because we enjoy sleeping through pager hours.
