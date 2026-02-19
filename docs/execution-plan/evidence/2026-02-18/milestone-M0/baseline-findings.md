# Baseline Findings
Date: 2026-02-18
Task IDs: M0-T01
VG IDs: VG-CONTRACT-001
Command(s): mixed discovery + validation commands across loop, loop-agent, rlm-claude-code, io-rflx
Result: pass
Notes: Baseline completed; multiple implementation blockers identified.

## Confirmed Findings

- `rlm-core` baseline compile failure in `rlm-core/src/ffi/mod.rs:152` (`let mut features = Vec::new();` under cfg-dependent push set).
- Rust REPL expects `register_signature` and `clear_signature` methods and submit result propagation.
- Python REPL server (`rlm-core/python/rlm_repl/main.py`) currently lacks those JSON-RPC methods.
- Python REPL sandbox lacks `SUBMIT` support and signature state wiring.
- Rust REPL spawn uses `python -m rlm_repl` while Python package exposes script entrypoint `rlm-repl` and no `rlm_repl.__main__`.
- SPEC/runtime drift for batched naming and SPEC-20 file-location mapping.
- `rlm-claude-code` is hard-coupled to loop via vendored `rlm-core`; `loop-agent` and `io-rflx` are active integration targets.

## Consumer Snapshot

- `rlm-claude-code`: targeted critical unit tests pass in current state.
- `loop-agent`: baseline suite pass observed and no hard runtime `rlm_core` dependency yet.
- `io-rflx`: baseline `cargo check -p rflx-core` passes.

