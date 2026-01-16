# Optimized Implementation Plan: Waves 3-5

> Intelligent sequencing to complete rlm-core and Lean FV

## Executive Summary

**Current State Analysis:**
- **Waves 1-2 Complete**: 16,315 lines of new code (Lean REPL, Topos, Sync, Spec Agent, Proof Automation, Go Bindings)
- **rlm-core**: 8/14 phases complete (57%)
- **Lean FV**: 5/6 phases complete (83%)

**Critical Path Identified**: Epistemic Verification (src-p4s) is the sole blocker for both adapters and migrations.

**Recommended Strategy**: 3 parallel waves with 3 tracks each, completing all work in ~4-6 weeks.

---

## 1. Current State Summary

### 1.1 rlm-core Epic (src-uci)

| Phase | Issue | Status | Effort |
|-------|-------|--------|--------|
| Phase 1: Core Types | src-s6x | ✅ CLOSED | - |
| Phase 1: Python REPL | src-u9c | ✅ CLOSED | - |
| Phase 2: Hypergraph Memory | src-cir | ✅ CLOSED | - |
| Phase 2: Reasoning Traces | src-tzy | ❌ OPEN | 1 week |
| Phase 3: LLM Client | src-bvx | ✅ CLOSED | - |
| Phase 3: Cost Tracking | src-dt2 | ❌ OPEN | 1 week |
| Phase 4: Python Bindings | src-9t4 | ✅ CLOSED | - |
| Phase 5: Go Bindings | src-8ox | ✅ CLOSED | - |
| **Phase 6: Epistemic Verification** | **src-p4s** | ❌ OPEN | **1-2 weeks** |
| Phase 6: Trajectory Streaming | src-y7b | ❌ OPEN | 1 week |
| Phase 7: Claude Code Adapter | src-nw2 | ❌ OPEN | 1-2 weeks |
| Phase 7: TUI Adapter | src-u9i | ❌ OPEN | 1-2 weeks |
| Migration: rlm-claude-code | src-cyl | ❌ OPEN | 1 week |
| Migration: recurse | src-m0c | ❌ OPEN | 1 week |

### 1.2 Lean FV Epic (src-vce)

| Phase | Issue | Status | Effort |
|-------|-------|--------|--------|
| Phase 1: Lean REPL | src-726 | ✅ CLOSED | - |
| Phase 2: Topos Integration | src-4sz | ✅ CLOSED | - |
| Phase 3: Dual-Track Sync | src-o99 | ✅ CLOSED | - |
| Phase 4: Spec Agent | src-9mn | ✅ CLOSED | - |
| Phase 5: Proof Automation | src-ryp | ✅ CLOSED | - |
| **Phase 6: DP Integration** | **src-cby** | ❌ OPEN | **1 week** |

---

## 2. Dependency Graph

```
                    WAVE 3 (Now)                    WAVE 4              WAVE 5
                    ════════════                    ════════            ════════

        ┌─────────────────────┐
        │ src-p4s: Epistemic  │ ─────────┐
        │ Verification        │          │
        │ [CRITICAL PATH]     │          │
        └─────────────────────┘          │
                                         │
        ┌─────────────────────┐          │    ┌──────────────────┐
        │ src-cby: DP         │          ├───►│ src-nw2: Claude  │
        │ Integration         │          │    │ Code Adapter     │────┐
        │ [Completes Lean FV] │          │    └──────────────────┘    │
        └─────────────────────┘          │                            │
                                         │    ┌──────────────────┐    │    ┌──────────────────┐
        ┌─────────────────────┐          ├───►│ src-u9i: TUI     │    ├───►│ src-cyl: migrate │
        │ src-tzy: Reasoning  │          │    │ Adapter          │────┤    │ rlm-claude-code  │
        │ Traces              │          │    └──────────────────┘    │    └──────────────────┘
        └─────────────────────┘          │                            │
                                         │                            │    ┌──────────────────┐
        ┌─────────────────────┐          │                            └───►│ src-m0c: migrate │
        │ src-dt2: Cost       │──────────┘                                 │ recurse          │
        │ Tracking            │                                            └──────────────────┘
        └─────────────────────┘

        ┌─────────────────────┐
        │ src-y7b: Trajectory │
        │ Streaming           │
        └─────────────────────┘
```

---

## 3. Optimized Wave Execution Plan

### Wave 3: Unblock & Complete (Start Now)

**Objective**: Clear the critical path and complete Lean FV epic.

| Track | Issue | Priority | Duration | Description |
|-------|-------|----------|----------|-------------|
| **A** | **src-p4s** | **P0** | 1-2 weeks | Epistemic Verification (Strawberry) |
| **B** | src-cby | P1 | 1 week | DP Integration (completes Lean FV) |
| **C1** | src-tzy | P2 | 1 week | Reasoning Traces |
| **C2** | src-dt2 | P2 | 1 week | Cost Tracking |
| **C3** | src-y7b | P2 | 1 week | Trajectory Streaming |

**Track A (Critical Path)**: Epistemic Verification
- KL divergence computation (Bernoulli, bits)
- Claim extractor (NL → atomic claims)
- Evidence scrubber (p0 estimation)
- Verification backends
- Memory gate integration
- REPL functions: `verify_claim()`, `audit_reasoning()`

**Track B (Complete Lean FV)**: DP Integration
- Spec coverage tracking (`/dp:spec coverage --with-lean`)
- Proof status tracking (complete/sorry/failed)
- Review checks for formalization coverage
- SPEC-XX.YY ↔ Lean theorem linking

**Track C (Observability)**: Parallelizable enhancements
- Reasoning traces with deciduous-style decision trees
- Per-component cost tracking
- Enhanced trajectory streaming with export

**Wave 3 Exit Criteria**:
- [ ] `verify_claim()` achieves >80% hallucination detection
- [ ] Memory gate rejects ungrounded facts (budget_gap > threshold)
- [ ] `/dp:spec coverage` shows Lean formalization status
- [ ] Cost tracking reports per-component usage
- [ ] Trajectory export works for replay

### Wave 4: Adapters (After src-p4s)

**Objective**: Complete platform adapters for both deployment targets.

| Track | Issue | Priority | Duration | Dependencies |
|-------|-------|----------|----------|--------------|
| **A** | src-nw2 | P1 | 1-2 weeks | src-p4s |
| **B** | src-u9i | P1 | 1-2 weeks | src-p4s |

**Track A**: Claude Code Plugin Adapter
- MCP tools: `rlm_execute`, `rlm_status`, `memory_query`
- Hooks: `SessionStart`, `UserPromptSubmit`, `PreCompact`
- Skill integration
- Epistemic verification integration

**Track B**: TUI Adapter (Bubble Tea)
- Panel renderers: RLM trace, REPL view, Memory inspector
- Trajectory event streaming to UI
- Budget status display
- Keyboard handlers

**Wave 4 Exit Criteria**:
- [ ] Claude Code plugin passes existing test suite
- [ ] TUI renders trajectory events correctly
- [ ] Both adapters integrate epistemic verification
- [ ] Performance: <100ms REPL latency, <10ms event latency

### Wave 5: Migrations (After Adapters)

**Objective**: Migrate existing projects to use rlm-core.

| Track | Issue | Priority | Duration | Dependencies |
|-------|-------|----------|----------|--------------|
| **A** | src-cyl | P2 | 1 week | src-nw2 |
| **B** | src-m0c | P2 | 1 week | src-u9i |

**Track A**: rlm-claude-code Migration
- Replace Python orchestrator with rlm-core
- Migrate memory to hypergraph
- Remove legacy implementation
- Regression testing

**Track B**: recurse Migration
- Replace Go RLM service with rlm-core via CGO
- Migrate memory to hypergraph
- Remove legacy implementation
- Regression testing

**Wave 5 Exit Criteria**:
- [ ] rlm-claude-code uses rlm-core exclusively
- [ ] recurse uses rlm-core exclusively
- [ ] No regressions in existing functionality
- [ ] Legacy code removed

---

## 4. Resource Allocation

### 4.1 Parallel Agent Sessions

For maximum throughput, use **3 parallel agents**:

| Agent | Wave 3 | Wave 4 | Wave 5 |
|-------|--------|--------|--------|
| **Agent 1** | src-p4s (Epistemic) | src-nw2 (Claude Code) | src-cyl (migrate) |
| **Agent 2** | src-cby (DP Integration) | src-u9i (TUI) | src-m0c (migrate) |
| **Agent 3** | src-tzy, src-dt2, src-y7b | Support/Review | Support/Review |

### 4.2 Execution Commands

```bash
# Wave 3 - Start immediately
bd update src-p4s --status in_progress --assignee claude
bd update src-cby --status in_progress --assignee claude
bd update src-tzy --status in_progress --assignee claude

# Wave 4 - After src-p4s completes
bd update src-nw2 --status in_progress --assignee claude
bd update src-u9i --status in_progress --assignee claude

# Wave 5 - After adapters complete
bd update src-cyl --status in_progress --assignee claude
bd update src-m0c --status in_progress --assignee claude
```

---

## 5. Risk Analysis

| Risk | Impact | Mitigation |
|------|--------|------------|
| Epistemic verification latency | High | Sample mode for interactive use, batch for storage |
| Adapter API changes | Medium | Version adapters with rlm-core |
| Migration regressions | High | Comprehensive test suites before migration |
| Parallel agent conflicts | Low | Separate module directories, coordinate via beads |

---

## 6. Success Metrics

### Per-Wave Metrics

| Wave | Success Criteria |
|------|------------------|
| Wave 3 | >80% hallucination detection, DP integration working, cost tracking active |
| Wave 4 | Adapters pass all tests, <100ms latency |
| Wave 5 | Zero regressions, legacy code removed |

### End-to-End Metrics

- [ ] rlm-core fully integrated in Claude Code and TUI
- [ ] Lean FV epic closed (src-vce)
- [ ] rlm-core epic closed (src-uci)
- [ ] All migrations complete
- [ ] >80% test coverage maintained

---

## 7. Recommended Immediate Actions

### Today

1. **Start Wave 3 Track A**: Begin Epistemic Verification (src-p4s)
   - This is the critical path blocker
   - Unblocks both adapters

2. **Start Wave 3 Track B**: Begin DP Integration (src-cby)
   - Completes the Lean FV epic
   - No blockers

3. **Start Wave 3 Track C**: Begin Observability trio
   - Parallelizable with no dependencies
   - Lower priority but quick wins

### This Week

4. **Review Strawberry/Pythea**: Understand KL-based verification
5. **Define claim extraction heuristics**: How to parse NL into atomic claims
6. **Design memory gate threshold**: Budget gap threshold for rejection

---

## 8. Timeline Estimate

| Phase | Duration | End Date (from start) |
|-------|----------|----------------------|
| Wave 3 | 2 weeks | Week 2 |
| Wave 4 | 2 weeks | Week 4 |
| Wave 5 | 1-2 weeks | Week 5-6 |

**Total**: ~5-6 weeks to complete both epics and migrations.
