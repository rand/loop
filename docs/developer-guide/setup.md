# Developer Setup

## Toolchain

Install and verify:

```bash
rustc --version
python3 --version
go version
uv --version
```

Recommended minimums:
- Rust 1.75+
- Python 3.11+
- Go 1.22+

## Repository Bootstrap

```bash
git clone https://github.com/rand/loop.git
cd loop
```

Build Rust core:

```bash
cd rlm-core
cargo build --release
```

## Python Binding Setup

```bash
cd /Users/rand/src/loop/rlm-core/python
uv sync
uv run maturin develop --release
```

## Go Binding Setup

```bash
cd /Users/rand/src/loop/rlm-core
test -f target/release/librlm_core.a || cargo build --release --lib

cd /Users/rand/src/loop/rlm-core/go/rlmcore
go test ./...
```

## Fast Sanity Check

From repository root:

```bash
make check
./scripts/dp review --json
```

If both pass, you have a working developer environment.

## Environment Tips

1. Run commands from repository root unless a doc says otherwise.
2. Prefer offline-capable iterations where practical.
3. Keep evidence files in `docs/execution-plan/evidence/...` for nontrivial validations.

## When Setup Fails

Go directly to:
- [Common Issues](../troubleshooting/common-issues.md)
- [Diagnostics Checklist](../troubleshooting/diagnostics-checklist.md)
