# Quickstart

This is the minimum path from clone to confident first run.

## 1. Prerequisites

Install:
- Rust 1.75+
- Python 3.11+
- Go 1.22+
- `uv`

## 2. Clone and Build

```bash
git clone https://github.com/rand/loop.git
cd loop

cd rlm-core
cargo build --release
```

Expected result:
- `cargo build` exits `0`

## 3. Run Core Quality Check

From repository root:

```bash
make check
```

Expected result:
- Typecheck + tests pass
- Exit code `0`

## 4. Run Governance Gates

```bash
./scripts/dp review --json
./scripts/dp verify --json
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Expected result:
- Each command returns JSON with `"ok": true`

## 5. Optional: Python Bindings

```bash
cd rlm-core/python
uv sync
uv run maturin develop --release
uv run pytest -q
```

## 6. Optional: Go Bindings

```bash
cd rlm-core/go/rlmcore
go test ./...
```

## First-Day Pitfalls

1. Running commands from the wrong directory.
- Fix: run from repository root unless a doc says otherwise.

2. Skipping gates because "tests passed once".
- Fix: run `make check` and `./scripts/dp ...` every time you care about outcomes.

3. Treating warnings as background noise forever.
- Fix: warnings are cheap to ignore and expensive to rediscover.

## Next Steps

- For a guided progression by experience level: [Learning Paths](./learning-paths.md)
- For practical task flows: [Workflow Recipes](./workflow-recipes.md)
- For advanced operation: [Power User Playbook](./power-user-playbook.md)
- For exact command lookup: [Command Reference](../reference/command-reference.md)
