# Migration Specification: rlm-claude-code to rlm-core

> Detailed plan for migrating rlm-claude-code from pure Python to rlm-core Python bindings
>
> Historical planning artifact:
> - Unchecked `[ ]` checklist items in this file are archival snapshots, not active backlog.
> - Authoritative live status is tracked in Beads (`bd status`) and execution-plan trackers:
>   - `docs/execution-plan/STATUS.md`
>   - `docs/execution-plan/TASK-REGISTRY.md`
>   - `docs/execution-plan/WORKBOARD.md`

## Executive Summary

**Project**: rlm-claude-code (Claude Code plugin for RLM)
**Target**: Replace Python implementation with rlm-core Python bindings
**Scope**: ~60 Python modules, ~1.2M bytes of source code
**Strategy**: Phased migration with feature flags for gradual rollout

---

## ⚠️ Migration Reality (Updated Feb 2026)

**Actual execution** of this migration revealed important constraints:

### What's Actually Available in Python Bindings

| Type | Available | Notes |
|------|-----------|-------|
| `PatternClassifier` | ✅ | `should_activate()`, `classify()` |
| `MemoryStore` | ✅ | `open()`, `add_node()`, `query()` |
| `TrajectoryEvent` | ✅ | Factory methods, `to_json()`, `with_metadata()` |
| `TrajectoryEventType` | ✅ | All event type variants |
| `ClaimExtractor` | ✅ | `extract()` - pattern-based, not LLM |
| `SmartRouter` | ✅ | `route()` with `RoutingContext` |
| `CostTracker` | ✅ | `record()`, `merge()`, cost tracking |
| `quick_hallucination_check` | ✅ | Returns risk score (0.0-1.0) |
| `ReplPool` / `ReplHandle` | ❌ | **Not exposed** - Python execution must stay in Python |
| `ClaudeCodeAdapter` | ❌ | **Not exposed** - orchestration stays in Python |
| `Orchestrator` | ❌ | **Not exposed** - use component delegation instead |

### Migration Strategy Adjustment

Instead of full replacement, the migration uses **component-level delegation**:
- Python orchestration layer remains
- Individual components delegate to rlm_core when available
- Feature flag `RLM_USE_CORE=true/false` toggles delegation
- Legacy code preserved for backward compatibility

### Completed Migration (rlm-claude-code-rlmcore fork)

| Phase | Component | Status | Commit |
|-------|-----------|--------|--------|
| 1 | Infrastructure + `USE_RLM_CORE` flag | ✅ | `913628b` |
| 2 | `complexity_classifier.py` → `PatternClassifier` | ✅ | `788575c` |
| 3 | `memory_store.py` → `MemoryStore` | ✅ | `c2543ee` |
| 4 | `trajectory.py` → `TrajectoryEvent` | ✅ | `e695b39` |
| 5 | `repl_environment.py` → `ReplPool` | ⏭️ **N/A** | - |
| 6 | `epistemic/claim_extractor.py` → `ClaimExtractor` | ✅ | `1b1ccc7` |
| 7 | `smart_router.py` → `SmartRouter` | ✅ | `58e2bcf` |
| 8 | Cleanup legacy code | ⏭️ **Deferred** | - |

Authoritative live status for this repository is the closed Beads task `loop-cyl`:
- Component delegation is complete and supported.
- Full Python replacement remains intentionally out of scope until the Python bindings expose orchestration/repl surfaces (`Orchestrator`, `ClaudeCodeAdapter`, `ReplPool`/`ReplHandle`).

---

## 1. Module Mapping (Historical Target vs Current Reality)

### 1.1 Direct Replacements (Historical Target-State Plan)

This table is archival target-state planning from early migration design.
For live truth, use the "Migration Reality" section above.

| rlm-claude-code Module | rlm-core Replacement | Notes |
|------------------------|----------------------|-------|
| `orchestrator.py` | `rlm_core.Orchestrator` | Target-state only; binding is not currently exposed |
| `intelligent_orchestrator.py` | `rlm_core.ClaudeCodeAdapter` | Target-state only; binding is not currently exposed |
| `local_orchestrator.py` | `rlm_core.Orchestrator` | Target-state only; binding is not currently exposed |
| `complexity_classifier.py` | `rlm_core.PatternClassifier` | Uses `ActivationDecision` |
| `auto_activation.py` | `rlm_core.PatternClassifier.should_activate()` | Built into classifier |
| `repl_environment.py` | `rlm_core.ReplHandle`, `ReplPool` | Target-state only; bindings not currently exposed |
| `memory_store.py` | `rlm_core.SqliteMemoryStore` | Hypergraph memory |
| `memory_backend.py` | `rlm_core.SqliteMemoryStore` | Unified backend |
| `memory_evolution.py` | `rlm_core.SqliteMemoryStore` (tier operations) | Consolidate/promote/decay |
| `trajectory.py` | `rlm_core.TrajectoryEvent` | Event streaming |
| `trajectory_analysis.py` | `rlm_core.TrajectoryEvent` | Analysis via events |
| `cost_tracker.py` | `rlm_core.CostTracker` (via LLM module) | Per-component tracking |
| `smart_router.py` | `rlm_core.SmartRouter` | Query-aware model selection |
| `reasoning_traces.py` | `rlm_core.ReasoningTraceStore` | Deciduous-style traces |
| `epistemic/` | `rlm_core.epistemic` module | Full replacement |

### 1.2 Partial Integrations

These modules integrate with rlm-core but retain some logic:

| Module | rlm-core Integration | Retained Logic |
|--------|----------------------|----------------|
| `api_client.py` | `rlm_core.AnthropicClient`, `LLMClient` | API-specific wrappers |
| `context_manager.py` | `rlm_core.SessionContext` | Plugin-specific context handling |
| `config.py` | `rlm_core.AdapterConfig` | Environment variable handling |
| `types.py` | `rlm_core.context`, `trajectory` types | Plugin-specific types |
| `rich_output.py` | `rlm_core.TrajectoryEvent` | Terminal formatting retained |

### 1.3 Remove (Legacy)

These modules become obsolete:

| Module | Reason |
|--------|--------|
| `learning.py` | Replaced by rlm-core memory evolution |
| `continuous_learning.py` | Replaced by rlm-core memory evolution |
| `strategy_cache.py` | Integrated into rlm-core memory |
| `state_persistence.py` | Handled by rlm-core SQLite |
| `embedding_retrieval.py` | Integrated into memory store |
| `context_index.py` | Integrated into memory store |
| `prompt_optimizer.py` | Handled by rlm-core routing |
| `learned_routing.py` | Replaced by SmartRouter |
| `setfit_classifier.py` | Replaced by PatternClassifier |
| `gliner_extractor.py` | Replaced by rlm-core extractors |

### 1.4 Keep (Plugin-Specific)

These modules remain in rlm-claude-code:

| Module | Reason |
|--------|--------|
| `__init__.py` | Plugin entry point |
| `repl_plugin.py` | Claude Code plugin interface |
| `tool_bridge.py` | MCP tool exposure |
| `response_parser.py` | Claude-specific response handling |
| `prompts.py` | Plugin-specific prompt templates |
| `visualization.py` | Claude Code output formatting |
| `progress.py` | UI progress indicators |

---

## 2. Migration Phases

This phased checklist is historical planning context.
Unchecked "Exit Criteria" items below are not the live backlog; they are preserved for traceability.
Live scope/status is tracked in Beads (`loop-cyl`) and `docs/execution-plan/STATUS.md`.

### Phase 1: Add rlm-core Dependency

**Duration**: 1-2 days
**Risk**: Low

1. Add rlm-core Python bindings to `pyproject.toml`:
   ```toml
   [project.optional-dependencies]
   rlm-core = ["rlm-core>=0.1.0"]
   ```

2. Create feature flag in config:
   ```python
   # config.py
   USE_RLM_CORE = os.getenv("RLM_USE_CORE", "false").lower() == "true"
   ```

3. Create adapter layer:
   ```python
   # adapters/core_adapter.py
   if USE_RLM_CORE:
       import rlm_core
       # Use rlm-core implementations
   else:
       # Use legacy implementations
   ```

**Exit Criteria**:
- [ ] rlm-core imports successfully
- [ ] Feature flag toggles between implementations
- [ ] Existing tests pass with flag off

### Phase 2: Migrate Complexity Classification

**Duration**: 2-3 days
**Risk**: Low

1. Replace `complexity_classifier.py`:
   ```python
   # Before
   from .complexity_classifier import ComplexityClassifier
   classifier = ComplexityClassifier()
   decision = classifier.classify(query, context)

   # After
   from rlm_core import PatternClassifier, SessionContext
   classifier = PatternClassifier()
   ctx = SessionContext(messages=context.messages)
   decision = classifier.should_activate(query, ctx)
   ```

2. Update `auto_activation.py` to use `ActivationDecision`

3. Update tests:
   - `tests/unit/test_auto_activation.py`
   - `tests/unit/test_complexity_classifier.py` (can be removed)

**Exit Criteria**:
- [ ] PatternClassifier produces equivalent results
- [ ] Auto-activation tests pass
- [ ] Performance within 10% of original

### Phase 3: Migrate Memory System

**Duration**: 1 week
**Risk**: Medium (data migration)

1. Replace `memory_store.py` with `SqliteMemoryStore`:
   ```python
   # Before
   from .memory_store import MemoryStore
   store = MemoryStore(db_path)

   # After
   from rlm_core import SqliteMemoryStore
   store = SqliteMemoryStore(db_path)
   ```

2. Migrate data schema:
   - Create migration script for existing SQLite databases
   - Map old node types to rlm-core `NodeType` enum
   - Map old tiers to rlm-core `Tier` enum

3. Update dependent modules:
   - `memory_backend.py` → Remove (use SqliteMemoryStore directly)
   - `memory_evolution.py` → Remove (use store.consolidate/promote/decay)
   - `cross_session_promotion.py` → Update to use rlm-core promotion

4. Update tests:
   - `tests/unit/test_memory_backend.py`
   - `tests/unit/test_memory_store.py`
   - `tests/integration/test_memory_evolution.py`

**Migration Script**:
```python
# scripts/migrate_memory.py
def migrate_database(old_path: str, new_path: str):
    """Migrate rlm-claude-code memory to rlm-core format."""
    # 1. Read old schema
    # 2. Transform node types
    # 3. Transform tier values
    # 4. Write to new schema
```

**Exit Criteria**:
- [ ] Existing memories migrate without data loss
- [ ] Tier evolution works correctly
- [ ] Semantic search produces equivalent results
- [ ] Memory tests pass

### Phase 4: Migrate Trajectory System

**Duration**: 2-3 days
**Risk**: Low

1. Replace `trajectory.py`:
   ```python
   # Before
   from .trajectory import TrajectoryEmitter, TrajectoryEvent
   emitter = TrajectoryEmitter()
   emitter.emit(TrajectoryEvent.RLM_START, {"query": query})

   # After
   from rlm_core import TrajectoryEvent, TrajectoryEventType
   event = TrajectoryEvent(
       event_type=TrajectoryEventType.RlmStart,
       content=query,
       depth=0
   )
   ```

2. Update `trajectory_analysis.py` to consume rlm-core events

3. Update `rich_output.py` to render rlm-core events

**Exit Criteria**:
- [ ] All event types map correctly
- [ ] Trajectory streaming works
- [ ] Export/replay functions work

### Phase 5: Migrate REPL Environment

**Status**: ⏭️ **NOT APPLICABLE**

**Reason**: `ReplPool` and `ReplHandle` are **not exposed** in rlm-core Python bindings.

**Technical Justification**:
The REPL environment executes arbitrary Python code in a sandboxed environment using `RestrictedPython`:

```python
# repl_environment.py uses Python-specific sandboxing
from RestrictedPython import compile_restricted, safe_builtins
from RestrictedPython.Guards import guarded_iter_unpack_sequence, safer_getattr
```

This is fundamentally Python-specific:
1. **Code compilation**: `compile_restricted()` compiles Python AST with security checks
2. **Execution namespace**: Python globals/locals with RLM helper functions injected
3. **Output capture**: Python stdout/stderr interception

Rust cannot efficiently execute arbitrary Python code. Even if `ReplPool` were exposed, it would just be a thin wrapper calling back into Python, adding overhead without benefit.

**Resolution**: Keep `repl_environment.py` as pure Python. No migration needed or possible.

### Phase 6: Migrate Epistemic Verification

**Duration**: 3-4 days
**Risk**: Low (new feature, parallel implementation)

1. Replace `epistemic/` module:
   ```python
   # Before
   from .epistemic import ClaimExtractor, HallucinationDetector

   # After
   from rlm_core import (
       ClaimExtractor, EpistemicVerifier, MemoryGate,
       verify_claim, quick_hallucination_check
   )
   ```

2. Wire up memory gate:
   ```python
   gate = MemoryGate(MemoryGateConfig(threshold=2.0))
   decision = gate.check(claim, evidence)
   if decision == GateDecision.Reject:
       # Don't store in memory
   ```

**Exit Criteria**:
- [ ] Hallucination detection rate maintained
- [ ] Memory gate rejects ungrounded facts
- [ ] Epistemic tests pass

### Phase 7: Migrate Orchestrator

**Status**: ⚠️ **PARTIAL - Component Delegation Only**

**Reason**: `ClaudeCodeAdapter` and `Orchestrator` are **not exposed** in rlm-core Python bindings.

**What Was Done Instead**:
Added `SmartRouter` delegation for routing decisions:

```python
# smart_router.py now has optional rlm_core delegation
class SmartRouter:
    @property
    def uses_rlm_core(self) -> bool:
        return self._core_router is not None

    def route_core(self, query: str, depth: int = 0, ...) -> dict | None:
        """Fast routing via rlm_core.SmartRouter."""
        if self._core_router is None:
            return None
        ctx = _rlm_core.RoutingContext().with_depth(depth)
        decision = self._core_router.route(query, ctx)
        return {"model": decision.model.id, "tier": str(decision.tier), ...}
```

**Technical Justification**:
The orchestration layer coordinates async operations across:
- Python asyncio event loop
- LLM API calls (aiohttp/httpx)
- REPL execution (subprocess)
- Memory operations (SQLite)

Cross-language async orchestration (Python asyncio ↔ Rust tokio) is complex and error-prone. The rlm-core bindings expose **component-level** APIs instead:

| Component | Delegation |
|-----------|------------|
| Model routing | ✅ `SmartRouter.route_core()` |
| Cost tracking | ✅ `CostTracker.record()` |
| Trajectory events | ✅ `TrajectoryEvent` factory methods |
| Memory operations | ✅ `MemoryStore` |
| Classification | ✅ `PatternClassifier.should_activate()` |

**Resolution**: Keep Python orchestrator. Components delegate to rlm_core individually.

**Future**: If rlm-core exposes `ClaudeCodeAdapter` Python bindings with proper async support, full orchestrator migration becomes possible.

### Phase 8: Cleanup

**Status**: ⏭️ **DEFERRED**

**Reason**: Legacy code must remain for backward compatibility.

**Why Cleanup Is Premature**:

1. **Feature flag pattern requires both paths**:
   ```python
   if USE_RLM_CORE and _rlm_core is not None:
       # Use rlm_core
   else:
       # Use Python implementation (legacy)
   ```

2. **Users without rlm_core installed** need the Python fallback

3. **Testing requires both modes**:
   - `RLM_USE_CORE=false` tests pure Python behavior
   - `RLM_USE_CORE=true` tests rlm_core delegation
   - Comparing both validates equivalence

4. **CI/CD pipelines** may not have rlm_core in all environments

**When Cleanup Becomes Appropriate**:

| Condition | Action |
|-----------|--------|
| rlm-core becomes required dependency | Remove feature flags |
| All consumers migrated to rlm-core | Remove legacy code |
| Python bindings expose all needed types | Full replacement possible |
| 6+ months stable with rlm_core | Safe to remove fallbacks |

**Current State**:
- Legacy code preserved in rlm-claude-code-rlmcore fork
- Feature flag `USE_RLM_CORE` defaults to `false`
- Both paths tested and working

---

## 3. Testing Strategy

### 3.1 Test Categories

| Category | Approach |
|----------|----------|
| Unit tests | Run with both implementations during migration |
| Integration tests | Test rlm-core integration points |
| Regression tests | Compare outputs between old and new |
| Performance tests | Benchmark critical paths |

### 3.2 Regression Testing

Create output comparison tests:
```python
def test_regression(query, context):
    """Compare old vs new implementation."""
    old_result = old_orchestrator.run(query, context)
    new_result = new_adapter.execute(query, context)
    assert_equivalent(old_result, new_result)
```

### 3.3 Performance Benchmarks

| Metric | Target |
|--------|--------|
| REPL execution | < 100ms (simple operations) |
| Memory query | < 200ms (semantic search) |
| Trajectory event | < 10ms |
| Cold start | < 2s |

---

## 4. Rollback Plan

Each phase includes rollback capability:

1. **Feature flag**: Set `RLM_USE_CORE=false` to revert
2. **Version pinning**: Keep old code until phase complete
3. **Database backup**: Backup memory before migration
4. **Git tags**: Tag before each phase for easy revert

---

## 5. Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Memory data loss | High | Low | Backup + migration script |
| Performance regression | Medium | Medium | Benchmarks per phase |
| API incompatibility | Medium | Low | Adapter pattern |
| Subprocess issues | Medium | Medium | Thorough REPL testing |
| Integration failures | High | Medium | Gradual rollout with flag |

---

## 6. Timeline Estimate

### Original Estimate vs Actual

| Phase | Original | Actual | Notes |
|-------|----------|--------|-------|
| Phase 1: Add dependency | 1-2 days | ✅ 1 day | As expected |
| Phase 2: Complexity | 2-3 days | ✅ 1 day | Simpler than expected |
| Phase 3: Memory | 5-7 days | ✅ 1 day | Schema handled by rlm_core |
| Phase 4: Trajectory | 2-3 days | ✅ 1 day | Factory methods simplified |
| Phase 5: REPL | 3-4 days | ⏭️ N/A | Not possible - Python-specific |
| Phase 6: Epistemic | 3-4 days | ✅ 1 day | Pattern-based only |
| Phase 7: Orchestrator | 5-7 days | ⚠️ Partial | Component delegation only |
| Phase 8: Cleanup | 2-3 days | ⏭️ Deferred | Backward compat needed |

### Revised Timeline

| Phase | Duration | Status |
|-------|----------|--------|
| Phase 1-4, 6-7 | ~1 week | ✅ Complete |
| Phase 5 | N/A | ⏭️ Skipped |
| Phase 8 | TBD | ⏭️ Deferred |
| **Validation & Testing** | 1-2 weeks | 🔄 Pending |
| **PR & Merge** | 1 week | 🔄 Pending |

**Total for component delegation**: ~1 week (actual)
**Total including validation**: ~3-4 weeks

---

## 7. Commands Reference

```bash
# Install rlm-core in development
pip install -e /path/to/rlm-core[python]

# Run with rlm-core enabled
RLM_USE_CORE=true python -m rlm_claude_code

# Run tests with both implementations
pytest tests/ --rlm-core-enabled
pytest tests/ --rlm-core-disabled

# Run memory migration
python scripts/migrate_memory.py --old-db ~/.rlm/memory.db --new-db ~/.rlm/memory_v2.db

# Benchmark
pytest tests/benchmarks/ --benchmark-compare
```

---

## 8. Success Criteria

### Component Delegation (Current Goal)

Migration is complete when:

- [x] All rlm-claude-code tests pass with `RLM_USE_CORE=false`
- [x] All tests pass with `RLM_USE_CORE=true` (rlm_core available)
- [ ] Performance within 10% of original
- [ ] Memory operations work with both backends
- [x] Feature flag controls delegation
- [x] Graceful fallback when rlm_core unavailable
- [x] Documentation updated with migration reality
- [ ] Fork merged via PR after validation

### Full Replacement (Future Goal - Requires Python Binding Updates)

Full migration requires rlm-core to expose:

- [ ] `ClaudeCodeAdapter` with async Python support
- [ ] `ReplPool` / `ReplHandle` (or accept Python-specific impl)
- [ ] Full `Orchestrator` interface

Until then, component delegation provides:
- Unified trajectory format across consumers
- Shared memory schema
- Consistent routing decisions
- Common epistemic verification primitives
