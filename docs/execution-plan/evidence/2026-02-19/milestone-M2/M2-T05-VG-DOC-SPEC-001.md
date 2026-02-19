# M2-T05 Doc/Spec Alignment Check
Date: 2026-02-19
Task IDs: M2-T05
VG IDs: VG-DOC-SPEC-001
Command(s): Manual protocol/documentation reconciliation against runtime implementation
Result: pass
Notes: SPEC-20 typed-signature protocol sections now match implemented Rust/Python runtime behavior.

## Checks

- [x] SPEC-20 protocol section now reflects `register_signature` + `clear_signature` methods.
- [x] SPEC-20 no longer describes non-existent JSON-RPC `submit` method.
- [x] Execute-response `submit_result` payload documented with concrete example.
- [x] File-location map updated from stale `python/rlm_repl/submit.py` to actual runtime files.
- [x] Test-plan table updated to current Rust and Python test names used in M2-T01..M2-T04.
