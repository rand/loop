# Command Reference

Canonical command map for Loop workflows.

All commands are intended to run from repository root unless stated otherwise.

## Issue Tracking (`bd`)

```bash
bd ready
bd show <issue-id>
bd update <issue-id> --status in_progress
bd close <issue-id> --reason "implemented + verified"
bd sync
```

Use for intake, claiming work, and closure tracking.

## Core Quality Gates

```bash
make check
make coverage
make rustdoc-check
make py-integration-gate
make ignored-repl-gate
make proptest-gate
make claude-adapter-gate
make docs-check
```

Use for local correctness, integration assurance, and docs style validation.

## Governance (`dp`)

```bash
./scripts/dp review --json
./scripts/dp verify --json
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Use for machine-readable policy and release readiness checks.

## Rust Focused Commands

```bash
cd /Users/rand/src/loop/rlm-core
cargo check --quiet
cargo test --quiet
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Use for direct crate iteration and rustdoc validation.

## Python Binding Commands

```bash
cd /Users/rand/src/loop/rlm-core/python
uv sync
uv run maturin develop --release
uv run pytest -q
```

Use for Python binding development and compatibility checks.

## Go Binding Commands

```bash
cd /Users/rand/src/loop/rlm-core
cargo build --release --lib --no-default-features --features tokio-runtime
cd /Users/rand/src/loop/rlm-core/go/rlmcore
go test ./...
```

Use for Go integration and FFI-backed verification.

## Troubleshooting Snapshot

```bash
rustc --version
python3 --version
go version
uv --version
git rev-parse HEAD
git status --short --branch
ps -axo pid=,command=,rss= -ww | rg -n "rlm_repl|lake env repl|\\brepl\\b" -S
```

Use for deterministic environment capture during incident triage.

## Docs Style Validation

```bash
make docs-check
```

Current guardrails:

1. No em dash punctuation in operational docs.
2. Scope excludes historical evidence artifacts.

## Session Closeout

```bash
git status
git pull --rebase
bd sync
git add ...
git commit -m "type: summary"
git push
git status
```

Use to avoid stranded local work at the end of a session.
