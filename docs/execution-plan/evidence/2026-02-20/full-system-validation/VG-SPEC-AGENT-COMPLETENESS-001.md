# VG-SPEC-AGENT-COMPLETENESS-001
Date: 2026-02-20
Issue: `loop-xmy`

## Scope
Reduce unresolved placeholder output in spec-agent generators while preserving an explicit draft mode.

## Implemented
1. Added `CompletenessMode` with default `Baseline` in `/Users/rand/src/loop/rlm-core/src/spec_agent/types.rs`.
2. Threaded completeness mode through:
- `/Users/rand/src/loop/rlm-core/src/spec_agent/agent.rs`
- `/Users/rand/src/loop/rlm-core/src/spec_agent/generators.rs`
- `/Users/rand/src/loop/rlm-core/src/spec_agent/mod.rs` (documentation/re-export)
3. `Baseline` mode now emits non-placeholder defaults (no `TODO`/`sorry`) for:
- Topos concepts/behaviors
- Lean structures/invariants/contracts/proof stubs
4. `Placeholder` mode is explicit opt-in and preserves draft markers.

## Validation
- `cargo test --offline spec_agent:: -- --nocapture` passed.
- New tests validate both modes and cover at least two requirement classes (data structure + behavior):
- `test_baseline_mode_contracts_are_placeholder_free_for_data_and_behavior`
- `test_placeholder_mode_emits_explicit_markers`

