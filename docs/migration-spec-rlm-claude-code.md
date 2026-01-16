# Migration Specification: rlm-claude-code to rlm-core

> Detailed plan for migrating rlm-claude-code from pure Python to rlm-core Python bindings

## Executive Summary

**Project**: rlm-claude-code (Claude Code plugin for RLM)
**Target**: Replace Python implementation with rlm-core Python bindings
**Scope**: ~60 Python modules, ~1.2M bytes of source code
**Strategy**: Phased migration with feature flags for gradual rollout

---

## 1. Module Mapping

### 1.1 Direct Replacements

These modules map directly to rlm-core equivalents:

| rlm-claude-code Module | rlm-core Replacement | Notes |
|------------------------|----------------------|-------|
| `orchestrator.py` | `rlm_core.Orchestrator` | Main orchestration loop |
| `intelligent_orchestrator.py` | `rlm_core.ClaudeCodeAdapter` | Adapter handles orchestration |
| `local_orchestrator.py` | `rlm_core.Orchestrator` | Merge into main orchestrator |
| `complexity_classifier.py` | `rlm_core.PatternClassifier` | Uses `ActivationDecision` |
| `auto_activation.py` | `rlm_core.PatternClassifier.should_activate()` | Built into classifier |
| `repl_environment.py` | `rlm_core.ReplHandle`, `ReplPool` | Python REPL via subprocess |
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

**Duration**: 3-4 days
**Risk**: Medium (subprocess management)

1. Replace `repl_environment.py`:
   ```python
   # Before
   from .repl_environment import ReplEnvironment
   repl = ReplEnvironment()
   result = repl.execute(code)

   # After
   from rlm_core import ReplPool, ReplConfig
   config = ReplConfig(timeout_ms=30000)
   pool = ReplPool(config)
   handle = pool.get_handle()
   result = handle.execute(code)
   ```

2. Migrate helper functions:
   - `peek()`, `search()`, `summarize()`, `llm()` available via rlm-core

3. Handle deferred operations:
   - rlm-core uses `DeferredOperation` pattern
   - Update async handling

**Exit Criteria**:
- [ ] Code execution produces same results
- [ ] Helper functions work correctly
- [ ] Timeouts and resource limits enforced
- [ ] REPL tests pass

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

**Duration**: 1 week
**Risk**: High (core functionality)

1. Replace `orchestrator.py` and `intelligent_orchestrator.py`:
   ```python
   # Before
   from .intelligent_orchestrator import IntelligentOrchestrator
   orchestrator = IntelligentOrchestrator(config)
   result = await orchestrator.run(query, context)

   # After
   from rlm_core import ClaudeCodeAdapter, AdapterConfig
   config = AdapterConfig(...)
   adapter = ClaudeCodeAdapter(config)
   result = await adapter.execute(RlmRequest(query=query, context=context))
   ```

2. Wire up hooks:
   ```python
   adapter.handle_session_start(session_context)
   adapter.handle_prompt_submit(prompt, context)
   adapter.handle_pre_compact(context)
   ```

3. Remove legacy orchestrators:
   - `local_orchestrator.py`
   - `lats_orchestration.py`
   - `tree_of_thoughts.py`

**Exit Criteria**:
- [ ] End-to-end orchestration works
- [ ] Recursive calls function correctly
- [ ] Cost tracking accurate
- [ ] All orchestration tests pass

### Phase 8: Cleanup

**Duration**: 2-3 days
**Risk**: Low

1. Remove legacy modules (see Section 1.3)
2. Remove feature flags
3. Update documentation
4. Update pyproject.toml to require rlm-core
5. Tag release

**Exit Criteria**:
- [ ] No legacy code remains
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Package size reduced

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

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Add dependency | 1-2 days | 1-2 days |
| Phase 2: Complexity | 2-3 days | 4-5 days |
| Phase 3: Memory | 5-7 days | 9-12 days |
| Phase 4: Trajectory | 2-3 days | 11-15 days |
| Phase 5: REPL | 3-4 days | 14-19 days |
| Phase 6: Epistemic | 3-4 days | 17-23 days |
| Phase 7: Orchestrator | 5-7 days | 22-30 days |
| Phase 8: Cleanup | 2-3 days | 24-33 days |

**Total**: ~4-5 weeks

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

Migration is complete when:

- [ ] All rlm-claude-code tests pass
- [ ] No regression in functionality
- [ ] Performance within 10% of original
- [ ] Memory migration works without data loss
- [ ] Feature flag removed
- [ ] Legacy code deleted
- [ ] Documentation updated
- [ ] Release tagged
