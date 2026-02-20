# M7-T08 VG-DOC-SPEC-002
Date: 2026-02-20
Task: M7-T08 SPEC-25 context externalization contract closure

## Scope
Reconcile SPEC-25 documentation with current runtime behavior for context externalization prompt contract and helper surfaces.

## Checklist
- [x] SPEC status updated from draft to implementation snapshot with explicit implemented/deferred boundaries.
- [x] Root prompt contract section explicitly reflects `SUBMIT({...})` completion semantics and no-full-context behavior.
- [x] Helper surface mapping reflects active runtime helpers (`peek`, `search`, `summarize`, `find_relevant`) and deferred-operation semantics.
- [x] Size-tracking section distinguishes implemented warning/threshold behavior from deferred explicit size-policy APIs.
- [x] Test plan maps to real Rust/Python tests currently present in repo.

## Result
- Pass
- SPEC-25 now matches runtime contract semantics used by `ExternalizedContext` and the Python REPL helper layer.

## References
- `/Users/rand/src/loop/docs/spec/SPEC-25-context-externalization.md`
- `/Users/rand/src/loop/rlm-core/src/context/externalize.rs`
- `/Users/rand/src/loop/rlm-core/python/rlm_repl/helpers.py`
- `/Users/rand/src/loop/rlm-core/python/tests/test_repl.py`
