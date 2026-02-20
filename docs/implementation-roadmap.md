# Implementation Roadmap

> Intelligent sequencing for maximum efficacy and performance

> Historical planning artifact. Last reconciled against Beads/`STATUS.md` on 2026-02-20.
> Live issue state is tracked in `bd` and `docs/execution-plan/STATUS.md`; tables below are archival sequencing context.
> Checklist lines marked `[historical target]` are archival snapshots, not active backlog.

## Executive Summary

This roadmap sequences work across two major initiatives:
1. **rlm-core completion** - Phases 5-7 remaining
2. **Lean Formal Verification** - New capability

The plan maximizes parallelization while respecting dependencies, targeting efficient resource utilization.

## Current Authoritative State (2026-02-20)

- Runtime/spec closure is tracked under `loop-azq` children in Beads.
- M7 milestone cards are complete and archived under `docs/execution-plan/evidence/2026-02-20/milestone-M7/`.
- For current execution order and status, use:
  - `docs/execution-plan/STATUS.md`
  - `docs/execution-plan/TASK-REGISTRY.md`
  - `bd status` / `bd ready`

---

## 1. Historical State Analysis (Archived)

### 1.1 rlm-core Status

| Phase | Issue | Status | Blocker |
|-------|-------|--------|---------|
| Phase 1: Core Types | src-s6x | **CLOSED** | - |
| Phase 1: Python REPL | src-u9c | **CLOSED** | - |
| Phase 2: Memory System | src-cir | **CLOSED** | - |
| Phase 2: Reasoning Traces | src-tzy | Open | - |
| Phase 3: LLM Client | src-bvx | **CLOSED** | - |
| Phase 3: Cost Tracking | src-dt2 | Open | - |
| Phase 4: Python Bindings | src-9t4 | **CLOSED** | - |
| Phase 5: Go Bindings | src-8ox | **CLOSED** | - |
| Phase 6: Epistemic Verification | src-p4s | **CLOSED** | - |
| Phase 6: Trajectory Streaming | src-y7b | **CLOSED** | - |
| Phase 7: Claude Code Adapter | src-nw2 | ⚠️ **PARTIAL** | See note |
| Phase 7: TUI Adapter | src-u9i | Open | src-8ox |

**Completed**: Core types, REPL, memory, LLM client, Python bindings, epistemic, trajectory, Go bindings
**Remaining**: TUI adapter, recurse migration

### 1.2 Migration Status (Updated Jan 2025)

| Migration | Issue | Status | Notes |
|-----------|-------|--------|-------|
| rlm-claude-code → rlm-core | loop-ziu | **CLOSED** | Component delegation complete |
| recurse → rlm-core | loop-p95 | Open | Go bindings ready, can start |

**Key Finding**: Python bindings don't expose `ClaudeCodeAdapter` or `ReplPool`. Migration uses **component-level delegation** instead of full replacement:

| Component | Python Binding | Delegation Status |
|-----------|---------------|-------------------|
| PatternClassifier | ✅ Available | ✅ Implemented |
| MemoryStore | ✅ Available | ✅ Implemented |
| TrajectoryEvent | ✅ Available | ✅ Implemented |
| ClaimExtractor | ✅ Available | ✅ Implemented |
| SmartRouter | ✅ Available | ✅ Implemented |
| CostTracker | ✅ Available | 🔄 Pending |
| ReplPool | ❌ Not exposed | N/A - Python-specific |
| ClaudeCodeAdapter | ❌ Not exposed | N/A - Keep Python orchestrator |

**Go Bindings** (for recurse/TUI migration):

| Component | Go Binding | Status |
|-----------|-----------|--------|
| SessionContext | ✅ Available | Ready |
| PatternClassifier | ✅ Available | Ready |
| MemoryStore | ✅ Available | Ready |
| Node/HyperEdge | ✅ Available | Ready |
| TrajectoryEvent | ✅ Available | Ready |
| TrajectoryCollector | ✅ Available | Ready |

All Go binding tests pass (21/21). Library: `librlm_core.dylib`

### 1.3 Lean Formal Verification Status

| Phase | Issue | Status | Blocker |
|-------|-------|--------|---------|
| Phase 1: Lean REPL | src-726 | Open | Epic approval |
| Phase 2: Topos Integration | src-4sz | Open | Epic approval |
| Phase 3: Dual-Track Sync | src-o99 | Open | src-726, src-4sz |
| Phase 4: Spec Agent | src-9mn | Open | src-726, src-4sz |
| Phase 5: Proof Automation | src-ryp | Open | src-726 |
| Phase 6: DP Integration | src-cby | Open | src-9mn |

**Key Insight**: Lean REPL and Topos Integration can start in parallel once the epic is approved.

### 1.4 Dependency Graph

```
                    ┌─────────────────────────────────────────────────────────────────┐
                    │                    CRITICAL PATH                                 │
                    └─────────────────────────────────────────────────────────────────┘

  STREAM A (rlm-core)                              STREAM B (Lean FV)
  ═══════════════════                              ═══════════════════

  ┌─────────────────┐                              ┌─────────────────┐
  │ Phase 5: Go     │                              │ Phase 1: Lean   │
  │ Bindings        │                              │ REPL            │
  │ (src-8ox)       │                              │ (src-726)       │
  └────────┬────────┘                              └────────┬────────┘
           │                                                │
           │         ┌─────────────────┐                    │
           │         │ Phase 6:        │                    │
           ├────────►│ Epistemic       │◄───────────────────┤
           │         │ (src-p4s)       │                    │
           │         └────────┬────────┘                    │
           │                  │                             │
           │                  │                    ┌────────┴────────┐
           │                  │                    │ Phase 2: Topos  │
           │                  │                    │ Integration     │
           │                  │                    │ (src-4sz)       │
           │                  │                    └────────┬────────┘
           │                  │                             │
  ┌────────┴────────┐         │                    ┌────────┴────────┐
  │ Phase 7: TUI    │         │                    │ Phase 3: Sync   │
  │ Adapter         │◄────────┤                    │ Engine          │
  │ (src-u9i)       │         │                    │ (src-o99)       │
  └─────────────────┘         │                    └────────┬────────┘
                              │                             │
                     ┌────────┴────────┐           ┌────────┴────────┐
                     │ Phase 7: Claude │           │ Phase 4: Spec   │
                     │ Code Adapter    │           │ Agent           │
                     │ (src-nw2)       │           │ (src-9mn)       │
                     └─────────────────┘           └────────┬────────┘
                                                            │
                                                   ┌────────┴────────┐
                                                   │ Phase 5: Proof  │
                                                   │ Automation      │
                                                   │ (src-ryp)       │
                                                   └────────┬────────┘
                                                            │
                                                   ┌────────┴────────┐
                                                   │ Phase 6: DP     │
                                                   │ Integration     │
                                                   │ (src-cby)       │
                                                   └─────────────────┘
```

---

## 2. Optimized Implementation Strategy

### 2.1 Parallelization Opportunities

**Wave 1 (Parallel Start)**:
- Stream A: Go Bindings (src-8ox)
- Stream B: Lean REPL (src-726) + Topos Integration (src-4sz)
- Stream C: Reasoning Traces (src-tzy) + Cost Tracking (src-dt2) + Trajectory Streaming (src-y7b)

**Wave 2 (After Wave 1 completes)**:
- Stream A: Epistemic Verification (src-p4s) - needs Go bindings for full testing
- Stream B: Dual-Track Sync (src-o99) - needs Lean REPL + Topos

**Wave 3 (After Wave 2)**:
- Stream A: TUI Adapter (src-u9i) + Claude Code Adapter (src-nw2)
- Stream B: Spec Agent (src-9mn) + Proof Automation (src-ryp)

**Wave 4 (Final)**:
- DP Integration (src-cby)
- Migrations (src-cyl, src-m0c)

### 2.2 Critical Path Analysis

The **critical path** runs through:
```
Lean REPL → Topos Integration → Spec Agent → DP Integration
```

This path takes approximately **8-10 weeks** and should be prioritized.

The **rlm-core completion path** is shorter:
```
Go Bindings → Epistemic Verification → Adapters → Migrations
```

This path takes approximately **5-6 weeks**.

**Recommendation**: Start both paths simultaneously, but if resource-constrained, prioritize Lean FV path as it enables new capabilities.

---

## 3. Detailed Execution Plan

### Phase 0: Preparation (Day 1)

| Task | Duration | Owner |
|------|----------|-------|
| Approve Lean FV epic (src-vce) | 1 hour | Human |
| Update blocking dependencies | 1 hour | Agent |
| Set up Lean 4 development environment | 2 hours | Agent |
| Verify rlm-core builds and tests pass | 1 hour | Agent |

### Wave 1: Foundation (Weeks 1-3)

**Parallel Tracks**:

| Track | Issues | Duration | Dependencies |
|-------|--------|----------|--------------|
| **A: Go Bindings** | src-8ox | 1 week | None |
| **B: Lean REPL** | src-726 | 2-3 weeks | None |
| **B: Topos Integration** | src-4sz | 1-2 weeks | None |
| **C: Observability** | src-tzy, src-dt2, src-y7b | 2 weeks | None |

**Wave 1 Deliverables**:
- Go bindings for rlm-core (CGO)
- Working Lean REPL with lake integration
- Topos MCP client with bidirectional linking
- Reasoning traces, cost tracking, trajectory streaming

### Wave 2: Core Capabilities (Weeks 3-5)

| Track | Issues | Duration | Dependencies |
|-------|--------|----------|--------------|
| **A: Epistemic** | src-p4s | 1-2 weeks | src-8ox (soft) |
| **B: Sync Engine** | src-o99 | 2 weeks | src-726, src-4sz |

**Wave 2 Deliverables**:
- Hallucination detection with memory gate
- Topos ↔ Lean drift detection and sync

### Wave 3: Integration (Weeks 5-8)

| Track | Issues | Duration | Dependencies |
|-------|--------|----------|--------------|
| **A: Adapters** | src-nw2, src-u9i | 2 weeks | src-p4s, src-8ox |
| **B: Spec Agent** | src-9mn | 2-3 weeks | src-726, src-4sz |
| **B: Proof Automation** | src-ryp | 2-3 weeks | src-726 |

**Wave 3 Deliverables**:
- Claude Code plugin using rlm-core
- TUI using rlm-core
- Spec Agent with Topos + Lean generation
- Progressive proof automation pipeline

### Wave 4: Completion (Weeks 8-10)

| Track | Issues | Duration | Dependencies |
|-------|--------|----------|--------------|
| **A: Migrations** | src-cyl, src-m0c | 2 weeks | src-nw2, src-u9i |
| **B: DP Integration** | src-cby | 1-2 weeks | src-9mn |

**Wave 4 Deliverables**:
- rlm-claude-code migrated to rlm-core
- recurse migrated to rlm-core
- Formal spec integration with DP workflow

---

## 4. Resource Allocation Strategy

### 4.1 Parallel Agent Sessions

For maximum throughput, use **3 parallel agent sessions**:

| Session | Focus | Issues |
|---------|-------|--------|
| **Agent 1** | rlm-core completion | src-8ox → src-p4s → src-nw2/src-u9i |
| **Agent 2** | Lean REPL + Spec Agent | src-726 → src-9mn → src-ryp |
| **Agent 3** | Topos + Sync + DP | src-4sz → src-o99 → src-cby |

### 4.2 Human Touchpoints

| Checkpoint | When | Human Action Required |
|------------|------|----------------------|
| Epic approval | Day 1 | Approve src-vce to unblock |
| Architecture review | End of Wave 1 | Review Lean REPL design decisions |
| Spec Agent review | Mid Wave 3 | Review generated Topos/Lean specs |
| Migration approval | Start of Wave 4 | Approve migration plan |

### 4.3 Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Lean version fragmentation | Pin to Lean 4.x, use elan for version management |
| Mathlib build times | Pre-build mathlib cache, share across sessions |
| CGO complexity | Start with minimal C FFI, expand incrementally |
| Proof automation failures | Progressive tiers with human fallback |

---

## 5. Success Metrics

### 5.1 Per-Wave Metrics

| Wave | Success Criteria |
|------|------------------|
| Wave 1 | Go bindings compile, Lean REPL executes `#check Nat`, Topos links parse |
| Wave 2 | Hallucination detection >80%, drift detection catches divergence |
| Wave 3 | Adapters preserve existing functionality, Spec Agent generates valid specs |
| Wave 4 | Migrations complete without regression, DP commands work |

### 5.2 End-to-End Metrics (Historical Snapshot, Non-Backlog)

These checkboxes are a preserved planning snapshot. They do not represent live work intake.

- [historical target] rlm-core fully integrated in both Claude Code and TUI
- [historical target] Lean REPL executes commands and tactics correctly
- [historical target] >70% auto-proof rate on simple theorems
- [historical target] Spec Agent generates valid Topos + Lean from NL requirements
- [historical target] All SPEC-XX.YY items traceable through Lean theorems

---

## 6. Recommended Immediate Actions (Historical)

The action list below was valid for the original roadmap horizon and is retained only for archival context.

For current work intake and execution:

1. Start with `bd ready` and `bd show loop-azq`.
2. Follow `docs/execution-plan/STATUS.md` and `docs/execution-plan/TASK-REGISTRY.md`.
3. Treat this roadmap's legacy issue IDs (`src-*`) as historical references, not active queue items.

---

## 7. References

- [ADR-001: Unified RLM Library](./adr/ADR-001-unified-rlm-library.md)
- [ADR-002: Lean Formal Verification](./adr/ADR-002-lean-formal-verification.md)
- [Unified RLM Library Design](./unified-rlm-library-design.md)
- [Lean Formal Verification Design](./lean-formal-verification-design.md)
