# rlm-core Repository Consolidation Plan

## Executive Summary

Two separate rlm-core repositories exist with divergent development:
- **Standalone** (`/Users/rand/src/rlm-core/`): 31 files, 460KB - pushed to GitHub
- **Loop** (`/Users/rand/src/loop/rlm-core/`): 80 files, 1.3MB - NOT pushed to GitHub

**Decision: Loop version is the canonical source.** It contains all standalone functionality plus 8 additional module directories implementing Lean integration, formal verification, and adapters per the design specs.

---

## Repository Comparison

### Module Inventory

| Module | Standalone | Loop | Notes |
|--------|-----------|------|-------|
| complexity.rs | 487 lines | 487 lines | Equivalent |
| context.rs | 358 lines | 359 lines | Equivalent |
| error.rs | 104 lines | 105 lines | Equivalent |
| orchestrator.rs | 308 lines | 309 lines | Equivalent |
| repl.rs | 434 lines | Similar | Python REPL |
| trajectory.rs | 494 lines | Similar | Event streaming |
| epistemic/ | 3,241 lines | 3,874 lines | Loop more complete |
| llm/ | Present | Present | Multi-provider routing |
| memory/ | Present | Present | Hypergraph store |
| pybind/ | Present | Present | Python bindings |
| ffi/ | Present | Present | C FFI layer |
| **adapters/** | MISSING | Present | Claude Code + TUI adapters |
| **dp_integration/** | MISSING | Present | Disciplined Process |
| **lean/** | MISSING | Present | Lean 4 REPL integration |
| **proof/** | MISSING | Present | Proof automation |
| **reasoning/** | MISSING | Present | Decision tree tracing |
| **spec_agent/** | MISSING | Present | NL → formal specs |
| **sync/** | MISSING | Present | Topos ↔ Lean sync |
| **topos/** | MISSING | Present | Semantic contracts |

### Unique Content in Standalone (Must Preserve)

1. **`src/epistemic/proptest.rs`** (464 lines)
   - Property-based tests for KL divergence and entropy
   - Verifies mathematical invariants (Gibbs' inequality, etc.)
   - Uses `proptest` crate for fuzzing
   - **Status**: Does not exist in loop - MUST COPY

2. **`go/tui/`** (3 files, ~20KB)
   - Complete Bubble Tea TUI implementation in Go
   - `model.go` (10KB): Main TUI model with viewport, spinner
   - `stream.go` (5KB): Event streaming integration
   - `model_test.go` (5KB): Unit tests
   - **Status**: Loop has Rust TUI adapter, not Go TUI - MUST COPY

3. **`src/epistemic/budget.rs`** (326 lines)
   - KL divergence computation functions
   - **Status**: Loop has `kl.rs` (377 lines) - COMPARE before merging

### Git History

**Standalone commits:**
```
Initial → Phase 5 (Go) → Phase 6 (Epistemic) → Phase 7 (Adapters) → PropTest → E2E fixes
```

**Loop commits:**
```
Initial → Wave 1 (Lean, Topos, Go) → Wave 2 (Spec Agent, Sync, Proof) → Waves 3-4 (Epistemic, Reasoning, DP, Adapters)
```

---

## Consolidation Steps

### Phase 1: Preserve Unique Standalone Content

```bash
# 1. Copy proptest.rs to loop
cp /Users/rand/src/rlm-core/src/epistemic/proptest.rs \
   /Users/rand/src/loop/rlm-core/src/epistemic/proptest.rs

# 2. Copy Go TUI to loop
cp -r /Users/rand/src/rlm-core/go/tui \
   /Users/rand/src/loop/rlm-core/go/tui

# 3. Copy go.mod and go.sum if different
cp /Users/rand/src/rlm-core/go/go.mod \
   /Users/rand/src/loop/rlm-core/go/go.mod
cp /Users/rand/src/rlm-core/go/go.sum \
   /Users/rand/src/loop/rlm-core/go/go.sum
```

### Phase 2: Update Loop's epistemic/mod.rs

Add proptest module declaration:
```rust
#[cfg(test)]
mod proptest;  // Add this line
```

### Phase 3: Verify proptest.rs Compatibility

The standalone `proptest.rs` imports from `budget.rs`:
```rust
use crate::epistemic::budget::{
    binary_entropy, binary_entropy_bits, compute_budget, kl_bernoulli, kl_bernoulli_bits,
};
```

Loop uses `kl.rs` instead. Need to either:
- **Option A**: Rename loop's `kl.rs` to `budget.rs` and ensure API matches
- **Option B**: Update proptest.rs imports to use loop's `kl.rs` API

**Recommended: Option B** - Update proptest.rs to use loop's API:
```rust
use crate::epistemic::kl::{
    bernoulli_kl_bits, binary_entropy_bits, // etc.
};
```

### Phase 4: Add proptest Dependency

Update loop's `Cargo.toml`:
```toml
[dev-dependencies]
proptest = "1.4"
```

### Phase 5: Git Repository Migration

**Option A: Force Push Loop to GitHub (Recommended)**
```bash
cd /Users/rand/src/loop/rlm-core
git remote add github git@github.com:rand/rlm-core.git
git push --force github main
```

**Option B: Merge Histories**
```bash
cd /Users/rand/src/loop/rlm-core
git remote add standalone /Users/rand/src/rlm-core
git fetch standalone
git merge standalone/main --allow-unrelated-histories -m "Merge standalone history"
git push origin main
```

**Recommended: Option A** - Loop version is more complete and follows the spec's wave-based development. Standalone history can be archived.

### Phase 6: Archive Standalone

```bash
mv /Users/rand/src/rlm-core /Users/rand/src/rlm-core-archived-$(date +%Y%m%d)
```

### Phase 7: Verify Build

```bash
cd /Users/rand/src/loop/rlm-core
cargo build --all-features
cargo test
maturin develop --features python
```

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Losing standalone-unique code | Phase 1 copies all unique content first |
| proptest.rs API mismatch | Phase 3 adapts imports to loop's API |
| GitHub history loss | Archive standalone before force push |
| Build failures | Phase 7 verifies before cleanup |

---

## Verification Checklist

After consolidation:

- [ ] `cargo check` passes
- [ ] `cargo test` passes (including proptest tests)
- [ ] `maturin develop --features python` builds Python bindings
- [ ] Go bindings compile: `cd go && go build ./...`
- [ ] Go TUI tests pass: `cd go/tui && go test`
- [ ] All 8 wave modules present: adapters, dp_integration, lean, proof, reasoning, spec_agent, sync, topos
- [ ] proptest.rs exists in epistemic/
- [ ] go/tui/ exists with model.go, stream.go

---

## Rationale

**Why loop version is canonical:**

1. **Completeness**: 80 files vs 31 files (2.8x more code)
2. **Spec Alignment**: Implements waves 1-4 from implementation-roadmap.md
3. **ADR Compliance**: Follows ADR-001 (unified library) and ADR-002 (Lean integration)
4. **Feature Set**: Has all 8 advanced modules that standalone lacks:
   - Lean REPL integration (ADR-002 requirement)
   - Proof automation with 4-tier strategy
   - Spec Agent for NL → formal specs
   - Topos ↔ Lean bidirectional sync
   - DP Integration for SPEC-XX.YY tracking
   - Claude Code + TUI adapters

**Why preserve standalone content:**

1. **proptest.rs**: Valuable property-based tests for mathematical invariants
2. **go/tui/**: Complete Bubble Tea implementation complements Rust TUI adapter
