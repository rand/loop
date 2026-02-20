# VG-GRAPH-MEMORY-001
Date: 2026-02-20
Issues: `loop-vu7`, `loop-y27`

## Scope
Optimize graph-analysis hot paths in drift detection by reducing repeated symbol allocation/hashing and batching short-lived analysis allocations.

## Target Module
- `/Users/rand/src/loop/rlm-core/src/sync/drift.rs`

## Implemented
1. Added internal `StringInterner` (`SymbolId`) for repeated symbol names used in:
- `detect_concept_drift`
- `detect_behavior_drift`
2. Added internal typed arena (`TypedArena<T>`) and migrated temporary analysis-node allocation to arena-backed storage.
3. Switched concept/structure/theorem/link lookups from repeated string-key matching to symbol-keyed lookups.
4. Kept all changes internal to sync drift analysis (no public API changes).

## Migration Notes
1. This is an internal implementation detail only; no serialization schema or external contract changes.
2. Existing drift outputs and suggestions are preserved semantically.
3. Allocation model changed for temporary analysis nodes only (single vector-backed arena per pass).

## Correctness Validation
- `cargo test --offline sync::drift:: -- --nocapture` passed.
- Added unit and property-based tests:
- `test_string_interner_reuses_ids`
- `prop_string_interner_idempotent`
- `prop_behavior_drift_is_order_invariant`

## Success Metrics and Benchmark/Profiling Plan
1. Metric: total allocations in drift detection passes.
- Method: compare baseline vs candidate with allocation profiling (`heaptrack`/`Instruments`) on synthetic large specs (10k+ symbols).
2. Metric: peak RSS during `detect_all`.
- Method: capture RSS over repeated runs using `/bin/ps` sampling.
3. Metric: wall-clock latency of `detect_all`.
- Method: run N>=30 iterations on fixed fixture and report p50/p95.

