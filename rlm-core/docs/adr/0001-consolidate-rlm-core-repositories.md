# ADR-0001: Consolidate rlm-core repositories - Loop version as canonical

**Status**: Accepted
**Date**: 2026-01-17
**Deciders**: rand, Claude
**Relates to**: ADR-001 (unified-rlm-library), ADR-002 (lean-formal-verification)

## Context

Two separate rlm-core repositories were inadvertently created during development:

1. **Standalone** (`/Users/rand/src/rlm-core/`)
   - 31 Rust files, 460KB source
   - Pushed to GitHub (git@github.com:rand/rlm-core.git)
   - Contains core modules only: complexity, context, error, orchestrator, repl, trajectory, epistemic, llm, memory, pybind, ffi
   - Has unique content: `proptest.rs` (property-based tests), `go/tui/` (Bubble Tea TUI)

2. **Loop** (`/Users/rand/src/loop/rlm-core/`)
   - 80 Rust files, 1.3MB source
   - NOT pushed to GitHub
   - Contains all standalone modules PLUS 8 advanced modules: adapters, dp_integration, lean, proof, reasoning, spec_agent, sync, topos
   - Implements waves 1-4 from implementation-roadmap.md

The divergence creates confusion about which is authoritative and risks losing work from either branch.

## Decision

**We will use the Loop version as the canonical rlm-core repository.**

The consolidation will:
1. Preserve unique standalone content (proptest.rs, go/tui/) by copying to loop
2. Adapt proptest.rs imports to use loop's `kl.rs` API
3. Force push loop version to GitHub, replacing standalone history
4. Archive standalone directory

## Alternatives Considered

### Alternative 1: Standalone as canonical, merge loop modules

Merge the 8 advanced modules from loop into standalone.

**Pros**:
- Preserves GitHub commit history
- Standalone already pushed to remote

**Cons**:
- Standalone is missing 49 Rust files (80-31)
- Would require extensive merging of epistemic/ (different file structure)
- Loop's wave-based development is more aligned with specs

### Alternative 2: Merge histories with --allow-unrelated-histories

Combine both git histories into one repository.

**Pros**:
- Preserves all commit history from both repos
- No force push required

**Cons**:
- Creates confusing dual-root history
- Doesn't resolve which code is canonical
- May create merge conflicts in overlapping files

### Alternative 3: Start fresh from loop (chosen)

Force push loop to GitHub, archive standalone.

**Pros**:
- Clean, single history aligned with spec waves
- Loop is 2.8x larger and more complete
- Follows ADR-001 and ADR-002 architectural decisions

**Cons**:
- Loses standalone's GitHub commit history
- Requires force push (mitigated by archiving)

## Consequences

### Positive
- Single source of truth for rlm-core
- All 8 advanced modules available (lean, proof, spec_agent, etc.)
- Property-based tests preserved via copy
- Go TUI preserved via copy

### Negative
- Standalone GitHub history replaced (archived locally)
- proptest.rs requires import adaptation for loop's kl.rs API

### Neutral
- GitHub remote URL unchanged (git@github.com:rand/rlm-core.git)
- Both Python and Go bindings remain functional

## References

- `/Users/rand/src/loop/docs/unified-rlm-library-design.md` - Main design spec
- `/Users/rand/src/loop/docs/adr/ADR-001-unified-rlm-library.md` - Unified library ADR
- `/Users/rand/src/loop/docs/adr/ADR-002-lean-formal-verification.md` - Lean integration ADR
- `/Users/rand/src/loop/docs/implementation-roadmap.md` - Wave-based implementation plan
- `/Users/rand/src/loop/rlm-core/CONSOLIDATION_PLAN.md` - Detailed execution steps
