# M4-T01 VG-CONTRACT-001
Date: 2026-02-19
Task: M4-T01 `rlm-claude-code` vendored sync and compatibility verification
Gate: VG-CONTRACT-001
Result: pass

## Contract Review Scope

- `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
- `M4-T01-VG-RCC-001.txt`
- `M4-T01-submodule-state.txt`
- `M4-T01-consumer-coupling-scan.txt`
- `M4-T01-loop-a5-compat-scan.txt`

## Invariant Check

- A1 import path: `pyproject.toml` keeps `module-name = "rlm_core"` and VG-RCC-001 passed.
- A2 trajectory coupling: `src/trajectory.py` still maps expected `rlm_core.TrajectoryEventType` values and factory methods.
- A3 memory store behavior: `tests/unit/test_memory_store.py` passed under VG-RCC-001 (SQLite/WAL assumptions preserved).
- A4 classifier/router delegation: `tests/unit/test_complexity_classifier.py` and `tests/unit/test_smart_router.py` passed.
- A5 batched helper compatibility: loop runtime exports canonical `llm_batch` and compatibility alias `llm_query_batched` with test coverage.

## Vendored Pin Strategy (M4-T01)

- Loop candidate SHA under evaluation: `50cd8cfe95f3179a4f15a445199fa9b1d1fe91f9` (`/Users/rand/src/loop` HEAD at validation time).
- VG-RCC-001 result scope is tied to `rlm-claude-code/vendor/loop` SHA `6779cdbc970c70f3ce82a998d6dcda59cd171560`.
- Compatibility evidence must record both loop candidate SHA and `vendor/loop` SHA.
- If SHAs differ, compatibility scope is limited to pinned vendor SHA until submodule sync + VG-RCC-001 rerun.

## Conclusion

No unresolved contract ambiguity remains for M4-T01 scope; A1-A5 hold for the currently pinned consumer state.
