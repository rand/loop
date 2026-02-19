# M3-T01 VG-DOC-SPEC-001
Date: 2026-02-19
Task: M3-T01 Resolve batched-query API naming and compatibility
Gate: VG-DOC-SPEC-001
Result: pass

## Checklist

- [x] Canonical runtime helper name is `llm_batch` in implementation (`rlm-core/python/rlm_repl/helpers.py`).
- [x] Compatibility alias `llm_query_batched` exists and routes to canonical helper (`rlm-core/python/rlm_repl/helpers.py`).
- [x] Alias behavior is explicit (deprecation warning + same deferred operation type/params).
- [x] Sandbox exports both helper names for migration compatibility (`rlm-core/python/rlm_repl/sandbox.py`).
- [x] User-facing helper docs identify canonical + alias (`rlm-core/python/README.md`).
- [x] SPEC-26 updated to canonical function naming while documenting alias policy (`docs/spec/SPEC-26-batched-queries.md`).
- [x] Focused REPL Python tests pass with alias coverage (`M3-T01-VG-LOOP-REPL-001-test-repl.txt`).

## Notes

- Canonical name remains stable as `llm_batch` per D-004.
- Compatibility alias remains available to avoid breaking existing/spec-driven call sites during migration.
