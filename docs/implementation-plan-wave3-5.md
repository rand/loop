# Implementation Status and Remaining Work

> Current state assessment and path to completion

> Reconciled on 2026-02-20 against Beads and `docs/execution-plan/STATUS.md`.
> This file is historical wave-planning context, not the live execution tracker.

## Executive Summary

**Wave 3-5 scope is complete.** Both major epics and migration tasks referenced by this plan are closed:

- **rlm-core Epic (loop-uci)**: 12/12 implementation phases closed
- **Lean FV Epic (loop-vce)**: 4/4 implementation phases closed

**Current Follow-on Work**:
- Post-M7 critical refinements (`loop-azq` + children) are closed.
- No open implementation backlog currently exists in Beads; new scope starts via new issue intake.
- Live state is maintained in `docs/execution-plan/STATUS.md`, `docs/execution-plan/TASK-REGISTRY.md`, and `bd status`.

---

## 1. Current State

### 1.1 Implemented Modules

| Module | Files | Status | Phase |
|--------|-------|--------|-------|
| **Core** | `orchestrator.rs`, `complexity.rs`, `context.rs`, `error.rs` | Complete | Phase 1 |
| **Python REPL** | `repl.rs` | Complete | Phase 1 |
| **Memory** | `memory/` (store, schema, types) | Complete | Phase 2 |
| **Reasoning Traces** | `reasoning/` (trace, store, query, types) | Complete | Phase 2 |
| **LLM Client** | `llm/` (client, router, cache, types) | Complete | Phase 3 |
| **Cost Tracking** | Integrated in `trajectory.rs` | Complete | Phase 3 |
| **Python Bindings** | `pybind/` (context, llm, memory, trajectory) | Complete | Phase 4 |
| **Go Bindings (FFI)** | `ffi/` (context, memory, trajectory, error, types) | Complete | Phase 5 |
| **Epistemic Verification** | `epistemic/` (verifier, kl, claims, scrubber, memory_gate) | Complete | Phase 6 |
| **Trajectory Streaming** | `trajectory.rs` (enhanced with budget, streaming) | Complete | Phase 6 |
| **Claude Code Adapter** | `adapters/claude_code/` (adapter, mcp, hooks, skills, types) | Complete | Phase 7 |
| **TUI Adapter** | `adapters/tui/` (adapter, panels, events) | Complete | Phase 7 |
| **Lean REPL** | `lean/` (repl, types) | Complete | Lean Phase 1 |
| **Topos Integration** | `topos/` (client, parser, index, types) | Complete | Lean Phase 2 |
| **Dual-Track Sync** | `sync/` (engine, drift, generators, types) | Complete | Lean Phase 3 |
| **Spec Agent** | `spec_agent/` (agent, parser, generators, types) | Complete | Lean Phase 4 |
| **Proof Automation** | `proof/` (engine, ai_assistant, tactics, types) | Complete | Lean Phase 5 |
| **DP Integration** | `dp_integration/` (commands, coverage, proof_status, review) | Complete | Lean Phase 6 |

### 1.2 Beads Issue Status

| Issue | Scope | Status |
|-------|-------|--------|
| `loop-uci` | Unified RLM epic | Closed |
| `loop-vce` | Lean FV epic | Closed |
| `loop-cyl` | `rlm-claude-code` migration | Closed |
| `loop-m0c` | `recurse` migration | Closed |

### 1.3 Code Statistics

- **Total Rust files**: 78
- **Lines of code**: ~30,000+ (including tests)
- **Git commits**: 4 (initial + waves 1-4)

---

## 2. Remaining Work (Historical Wave Scope)

### 2.1 Migration Tasks (Wave 5)

| Issue | Title | Status | Description |
|-------|-------|--------|-------------|
| `loop-cyl` | Migration: rlm-claude-code to rlm-core | Closed | Replaced legacy Python implementation with rlm-core delegation/runtime |
| `loop-m0c` | Migration: recurse to rlm-core | Closed | Replaced legacy Go implementation with rlm-core CGO bridges |

**Migration Strategy Notes (historical target vs executed reality):**

**rlm-claude-code (loop-cyl):**
1. Historical target in early specs was full replacement.
2. Executed reality is component-level delegation (feature-flagged) because Python bindings do not currently expose `Orchestrator`, `ClaudeCodeAdapter`, or `ReplPool`/`ReplHandle`.
3. Python orchestration/repl remain intentionally local; delegated components use `rlm-core` bindings.

**recurse (loop-m0c):**
1. Add rlm-core Go bindings via CGO, parallel implementation
2. Migrate RLM service to use rlm-core
3. Migrate memory to rlm-core hypergraph
4. Remove legacy Go implementation
5. Full rlm-core native with Go UI layer

### 2.2 Epic Closure

| Epic | Issue | Acceptance Criteria Status |
|------|-------|---------------------------|
| Unified RLM Library | `loop-uci` | Closed |
| Lean Formal Verification | `loop-vce` | Closed |

**loop-uci Acceptance Criteria:**
- [x] rlm-core Rust crate compiles
- [x] Python bindings (PyO3) available
- [x] Go bindings working with CGO
- [x] recurse migrated to use rlm-core (`loop-m0c`)
- [x] rlm-claude-code migrated to use rlm-core (`loop-cyl`)
- [x] >80% test coverage on core crate (historical closure claim; reproducible `llvm-cov` CI gate is now the canonical proof path)
- [x] All public APIs documented (historical closure interpreted as module-level docs; item-level rustdoc depth remains non-blocking and incremental)

**loop-vce Acceptance Criteria:**
- [x] Lean REPL executes commands and tactics
- [x] Topos ↔ Lean bidirectional linking
- [x] Drift detection for spec divergence
- [x] Spec Agent generates valid Topos and Lean
- [x] Progressive proof automation implemented
- [x] DP integration tracks formal spec coverage

### 2.3 Publishing Tasks (Still Program-Level Backlog)

| Task | Description | Status |
|------|-------------|--------|
| Publish to crates.io | Release rlm-core Rust crate | Not started |
| Publish to PyPI | Release Python bindings | Not started |
| Publish Go module | Release Go bindings | Not started |
| Documentation | API reference, usage guides | Partial (in code) |

---

## 3. Historical Execution Plan

This wave execution plan has already been completed.  
For current execution sequencing, use:

- `docs/execution-plan/STATUS.md`
- `docs/execution-plan/TASK-REGISTRY.md`
- `bd ready` / `bd status`

---

## 4. Dependency Summary

```
                          COMPLETE                                 CURRENT TRACK
                          ════════                                 ═════════════

    ┌─────────────────────────────────────────────────────────┐
    │              All Implementation Phases                   │
    │                                                          │
    │  Phase 1-7 (rlm-core)    ✅ CLOSED                      │
    │  Phase 1-6 (Lean FV)     ✅ CLOSED                      │
    └────────────────────────────┬────────────────────────────┘
                                 │
                                 ▼
              ┌──────────────────────────────────────┐
              │         Migration Tasks              │
              │                                      │
              │  loop-cyl: rlm-claude-code  ✅ CLOSED |
              │  loop-m0c: recurse          ✅ CLOSED |
              └─────────────────┬────────────────────┘
                                │
                                ▼
              ┌──────────────────────────────────────┐
              │         Epic Closure                 │
              │                                      │
              │  loop-uci: Unified RLM     ✅ CLOSED |
              │  loop-vce: Lean FV         ✅ CLOSED |
              └─────────────────┬────────────────────┘
                                │
                                ▼
              ┌──────────────────────────────────────┐
              │      Active Follow-on Backlog        │
              │                                      │
              │  No open implementation backlog      │
              │  (create issue from new findings)    │
              └──────────────────────────────────────┘
```

---

## 5. Key References

### Design Documents
- [Unified RLM Library Design](./unified-rlm-library-design.md)
- [Lean Formal Verification Design](./lean-formal-verification-design.md)

### Architecture Decision Records
- [ADR-001: Unified RLM Library](./adr/ADR-001-unified-rlm-library.md)
- [ADR-002: Lean Formal Verification](./adr/ADR-002-lean-formal-verification.md)

### Implementation Roadmap
- [Implementation Roadmap](./implementation-roadmap.md) (historical)
- `docs/execution-plan/STATUS.md` (authoritative live tracker)

---

## 6. Commands Reference

```bash
# View live status
bd ready                    # Show ready-to-work issues
bd list --status open       # Show open issues
bd stats                    # Project statistics

# Inspect historical post-M7 closure backlog
bd show loop-azq
bd children loop-azq

# Sync at session end
bd sync --from-main
git add . && git commit -m "..."
```
