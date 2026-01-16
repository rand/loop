# Migration Specification: recurse to rlm-core

> Detailed plan for migrating recurse from pure Go to rlm-core CGO bindings

## Executive Summary

**Project**: recurse (Go TUI for RLM)
**Target**: Replace Go RLM implementation with rlm-core CGO bindings
**Scope**: ~520 Go files, ~80,000+ lines of code
**Strategy**: Layered migration preserving Go TUI, replacing core RLM logic with Rust

---

## 1. Architecture Overview

### 1.1 Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Go Application                          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    TUI Layer (Bubble Tea)                │   │
│  │  Chat, Budget Panel, Memory Dialog, Trace Viewer, etc.  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Application Layer                     │   │
│  │  App, RLMService, REPLManager, MemoryProvider           │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                      RLM Core Layer                      │   │
│  │  Orchestrator, Meta, Routing, Classifier, Compress      │   │
│  │  ToT, LATS, Learning, Hallucination, Verify, REPL       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Data Layer                            │   │
│  │  Memory (Hypergraph), Budget, Session, Message          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Integration Layer                     │   │
│  │  Agent, LLM Providers, OAuth, LSP, Shell                │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Go Application                          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    TUI Layer (Bubble Tea)                │   │
│  │  Chat, Budget Panel, Memory Dialog, Trace Viewer, etc.  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Application Layer                     │   │
│  │  App, RLMBridge, Adapters, Event Handlers               │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │ CGO FFI                          │
│  ════════════════════════════╪══════════════════════════════   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    rlm-core (Rust)                       │   │
│  │  Orchestrator, Memory, Routing, Epistemic, Trajectory   │   │
│  │  REPL, Reasoning, Complexity, LLM Client                │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Integration Layer (Go)                │   │
│  │  Agent, LLM Providers, OAuth, LSP, Shell                │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Module Mapping

### 2.1 Direct Replacements

These Go packages map directly to rlm-core equivalents:

| recurse Package | rlm-core Replacement | Notes |
|-----------------|----------------------|-------|
| `rlm/orchestrator/` | `rlm_core::Orchestrator` | Core orchestration loop |
| `rlm/meta/` | `rlm_core::Orchestrator` | Meta-controller merged |
| `rlm/classifier.go` | `rlm_core::PatternClassifier` | Task classification |
| `rlm/routing/` | `rlm_core::SmartRouter` | Model selection |
| `rlm/repl/` | `rlm_core::ReplPool`, `ReplHandle` | Python REPL |
| `memory/hypergraph/` | `rlm_core::SqliteMemoryStore` | Hypergraph memory |
| `memory/embeddings/` | `rlm_core::SqliteMemoryStore` | Integrated embeddings |
| `memory/tiers/` | `rlm_core::Tier` enum | Memory tiers |
| `memory/reasoning/` | `rlm_core::ReasoningTraceStore` | Trace storage |
| `rlm/tot/` | `rlm_core::reasoning` module | Tree of Thoughts |
| `rlm/lats/` | `rlm_core::reasoning` module | LATS integration |
| `rlm/compress/` | `rlm_core::context` module | Context compression |
| `rlm/hallucination/` | `rlm_core::epistemic` module | Claim verification |
| `rlm/verify/` | `rlm_core::epistemic::EpistemicVerifier` | Verification |
| `rlm/learning/` | `rlm_core::SqliteMemoryStore` | Memory evolution |
| `budget/` | `rlm_core::CostTracker` | Token tracking |
| `rlm/observability/` | `rlm_core::TrajectoryEvent` | Event streaming |

### 2.2 FFI Bridge (New)

New Go code to bridge rlm-core:

| Go Bridge | Purpose |
|-----------|---------|
| `rlmcore/context.go` | Session context FFI wrapper |
| `rlmcore/memory.go` | Memory store FFI wrapper |
| `rlmcore/orchestrator.go` | Orchestrator FFI wrapper |
| `rlmcore/trajectory.go` | Trajectory event FFI wrapper |
| `rlmcore/types.go` | Type conversions |
| `rlmcore/error.go` | Error handling |

### 2.3 Partial Integrations

These packages integrate with rlm-core but retain Go logic:

| Package | rlm-core Integration | Retained Logic |
|---------|----------------------|----------------|
| `app/rlm.go` | `rlmcore.Orchestrator` | Service lifecycle |
| `app/memory_provider.go` | `rlmcore.MemoryStore` | TUI adapter |
| `app/repl_provider.go` | `rlmcore.ReplPool` | TUI adapter |
| `session/` | `rlmcore.SessionContext` | Go session management |
| `message/` | `rlmcore.TrajectoryEvent` | Message persistence |

### 2.4 Remove (Legacy)

These packages become obsolete:

| Package | Reason |
|---------|--------|
| `rlm/orchestrator/` | Replaced by rlm-core orchestrator |
| `rlm/meta/` | Merged into rlm-core orchestrator |
| `rlm/wrapper.go` | Handled by rlm-core |
| `rlm/service.go` | Replaced by bridge layer |
| `rlm/classifier.go` | Replaced by PatternClassifier |
| `rlm/routing/` | Replaced by SmartRouter |
| `rlm/compute.go` | Integrated into rlm-core |
| `rlm/proactive.go` | Handled by rlm-core |
| `rlm/guarantees.go` | Handled by rlm-core |
| `rlm/progress.go` | Handled by trajectory events |
| `rlm/tot/` | Replaced by rlm-core reasoning |
| `rlm/lats/` | Replaced by rlm-core reasoning |
| `rlm/compress/` | Replaced by rlm-core context |
| `rlm/cache/` | Integrated into rlm-core LLM |
| `rlm/async/` | Handled by rlm-core |
| `rlm/learning/` | Replaced by memory evolution |
| `rlm/hallucination/` | Replaced by rlm-core epistemic |
| `rlm/verify/` | Replaced by rlm-core epistemic |
| `rlm/synthesize/` | Handled by rlm-core |
| `rlm/decompose/` | Handled by rlm-core |
| `rlm/checkpoint/` | Handled by rlm-core |
| `memory/hypergraph/` | Replaced by SqliteMemoryStore |
| `memory/embeddings/` | Integrated into memory store |
| `memory/tiers/` | Handled by rlm-core Tier |
| `memory/reasoning/` | Replaced by ReasoningTraceStore |
| `budget/` (core logic) | Replaced by CostTracker |

### 2.5 Keep (Go-Specific)

These packages remain in Go:

| Package | Reason |
|---------|--------|
| `cmd/recurse/` | CLI entry point |
| `internal/cmd/` | Command handlers |
| `internal/tui/` | Bubble Tea UI (100+ files) |
| `internal/app/` | Application wiring (adapted) |
| `internal/agent/` | Agent coordination |
| `internal/config/` | Go configuration |
| `internal/db/` | Go database layer |
| `internal/oauth/` | OAuth flows |
| `internal/lsp/` | LSP client |
| `internal/shell/` | Shell execution |
| `internal/env/` | Environment utilities |
| `internal/home/` | Home directory utilities |
| `internal/projects/` | Project management |
| `internal/history/` | File history |

---

## 3. FFI Interface Design

### 3.1 CGO Header (rlm_core.h)

```c
// Context management
typedef struct RlmContext RlmContext;
RlmContext* rlm_context_new(const char* config_json);
void rlm_context_free(RlmContext* ctx);
int rlm_context_add_message(RlmContext* ctx, const char* role, const char* content);
char* rlm_context_get_messages_json(RlmContext* ctx);

// Memory store
typedef struct RlmMemoryStore RlmMemoryStore;
RlmMemoryStore* rlm_memory_store_new(const char* db_path);
void rlm_memory_store_free(RlmMemoryStore* store);
char* rlm_memory_store_query(RlmMemoryStore* store, const char* query_json);
int rlm_memory_store_upsert(RlmMemoryStore* store, const char* node_json);

// Orchestrator
typedef struct RlmOrchestrator RlmOrchestrator;
RlmOrchestrator* rlm_orchestrator_new(const char* config_json);
void rlm_orchestrator_free(RlmOrchestrator* orch);
char* rlm_orchestrator_execute(RlmOrchestrator* orch, const char* request_json);

// Trajectory events (callback-based)
typedef void (*RlmTrajectoryCallback)(const char* event_json, void* user_data);
void rlm_set_trajectory_callback(RlmOrchestrator* orch, RlmTrajectoryCallback cb, void* user_data);

// Error handling
char* rlm_get_last_error(void);
void rlm_free_string(char* s);
```

### 3.2 Go FFI Wrapper

```go
// internal/rlmcore/orchestrator.go
package rlmcore

/*
#cgo LDFLAGS: -lrlm_core
#include "rlm_core.h"
*/
import "C"
import (
    "encoding/json"
    "unsafe"
)

type Orchestrator struct {
    ptr *C.RlmOrchestrator
}

func NewOrchestrator(config Config) (*Orchestrator, error) {
    configJSON, _ := json.Marshal(config)
    cConfig := C.CString(string(configJSON))
    defer C.free(unsafe.Pointer(cConfig))

    ptr := C.rlm_orchestrator_new(cConfig)
    if ptr == nil {
        return nil, getLastError()
    }
    return &Orchestrator{ptr: ptr}, nil
}

func (o *Orchestrator) Execute(req Request) (*Response, error) {
    reqJSON, _ := json.Marshal(req)
    cReq := C.CString(string(reqJSON))
    defer C.free(unsafe.Pointer(cReq))

    cResp := C.rlm_orchestrator_execute(o.ptr, cReq)
    if cResp == nil {
        return nil, getLastError()
    }
    defer C.rlm_free_string(cResp)

    var resp Response
    json.Unmarshal([]byte(C.GoString(cResp)), &resp)
    return &resp, nil
}

func (o *Orchestrator) Close() {
    if o.ptr != nil {
        C.rlm_orchestrator_free(o.ptr)
        o.ptr = nil
    }
}
```

### 3.3 Trajectory Event Bridge

```go
// internal/rlmcore/trajectory.go
package rlmcore

/*
#include "rlm_core.h"

extern void goTrajectoryCallback(char* event_json, void* user_data);
*/
import "C"
import (
    "encoding/json"
    "sync"
)

var (
    callbackMu sync.RWMutex
    callbacks  = make(map[uintptr]func(TrajectoryEvent))
    nextID     uintptr
)

//export goTrajectoryCallback
func goTrajectoryCallback(eventJSON *C.char, userData unsafe.Pointer) {
    id := uintptr(userData)
    callbackMu.RLock()
    cb, ok := callbacks[id]
    callbackMu.RUnlock()

    if ok {
        var event TrajectoryEvent
        json.Unmarshal([]byte(C.GoString(eventJSON)), &event)
        cb(event)
    }
}

func (o *Orchestrator) SetTrajectoryCallback(cb func(TrajectoryEvent)) {
    callbackMu.Lock()
    id := nextID
    nextID++
    callbacks[id] = cb
    callbackMu.Unlock()

    C.rlm_set_trajectory_callback(
        o.ptr,
        C.RlmTrajectoryCallback(C.goTrajectoryCallback),
        unsafe.Pointer(id),
    )
}
```

---

## 4. Migration Phases

### Phase 1: Add rlm-core Dependency

**Duration**: 2-3 days
**Risk**: Low

1. Build rlm-core as shared library:
   ```bash
   cd /path/to/rlm-core
   cargo build --release --features ffi
   ```

2. Add CGO configuration to recurse:
   ```go
   // internal/rlmcore/cgo.go
   package rlmcore

   /*
   #cgo CFLAGS: -I${SRCDIR}/../../rlm-core/ffi/include
   #cgo LDFLAGS: -L${SRCDIR}/../../rlm-core/target/release -lrlm_core
   */
   import "C"
   ```

3. Create feature flag:
   ```go
   // internal/config/features.go
   var UseRlmCore = os.Getenv("RLM_USE_CORE") == "true"
   ```

4. Create minimal bridge layer with type definitions

**Exit Criteria**:
- [ ] rlm-core library links successfully
- [ ] Basic FFI calls work (create/destroy context)
- [ ] Feature flag toggles implementations
- [ ] Existing tests pass with flag off

### Phase 2: Migrate Memory System

**Duration**: 1 week
**Risk**: Medium (data migration)

1. Replace `memory/hypergraph/` with rlm-core bridge:
   ```go
   // Before
   store := hypergraph.NewStore(dbPath)
   results := store.Query(query)

   // After
   store := rlmcore.NewMemoryStore(dbPath)
   results := store.Query(query)
   ```

2. Migrate data schema:
   - Create migration script for existing SQLite databases
   - Map Go node types to rlm-core `NodeType` enum
   - Map Go tiers to rlm-core `Tier` enum
   - Preserve embeddings during migration

3. Update memory providers:
   - `app/memory_provider.go` → Use rlmcore.MemoryStore
   - TUI memory dialog → Consume rlm-core types

4. Update tests:
   - `memory/hypergraph/*_test.go`
   - Integration tests for memory operations

**Migration Script**:
```go
// cmd/migrate-memory/main.go
func migrateDatabase(oldPath, newPath string) error {
    // 1. Open old Go SQLite database
    // 2. Read all nodes and edges
    // 3. Transform to rlm-core types
    // 4. Write to new schema via FFI
    // 5. Verify data integrity
}
```

**Exit Criteria**:
- [ ] Existing memories migrate without data loss
- [ ] Semantic search produces equivalent results
- [ ] Tier operations work correctly
- [ ] Memory tests pass
- [ ] TUI memory dialog renders correctly

### Phase 3: Migrate Classification & Routing

**Duration**: 3-4 days
**Risk**: Low

1. Replace `rlm/classifier.go`:
   ```go
   // Before
   classifier := rlm.NewClassifier()
   classification := classifier.Classify(query, context)

   // After
   classifier := rlmcore.NewPatternClassifier()
   decision := classifier.ShouldActivate(query, context)
   ```

2. Replace `rlm/routing/`:
   ```go
   // Before
   router := routing.NewRouter(profiles)
   model := router.SelectModel(query, constraints)

   // After
   router := rlmcore.NewSmartRouter(config)
   model := router.Route(query, context)
   ```

3. Update callers in orchestration layer

**Exit Criteria**:
- [ ] Classification produces equivalent results
- [ ] Model routing works correctly
- [ ] Performance within 10% of original

### Phase 4: Migrate REPL System

**Duration**: 4-5 days
**Risk**: Medium (subprocess management)

1. Replace `rlm/repl/`:
   ```go
   // Before
   manager := repl.NewManager(config)
   result := manager.Execute(code)

   // After
   pool := rlmcore.NewReplPool(config)
   handle := pool.GetHandle()
   result := handle.Execute(code)
   ```

2. Bridge callbacks:
   - LLM callback → Bridge to Go LLM providers
   - Memory callback → Bridge to rlm-core memory

3. Handle resource management:
   - Subprocess lifecycle
   - Timeout enforcement
   - Resource limits

**Exit Criteria**:
- [ ] Code execution produces same results
- [ ] Callbacks work correctly
- [ ] Timeouts enforced
- [ ] Resource cleanup on exit

### Phase 5: Migrate Trajectory System

**Duration**: 2-3 days
**Risk**: Low

1. Replace `rlm/observability/` with rlm-core events:
   ```go
   // Before
   emitter := observability.NewEmitter()
   emitter.Emit(EventRLMStart, data)

   // After (callback-based)
   orchestrator.SetTrajectoryCallback(func(event TrajectoryEvent) {
       // Handle event
   })
   ```

2. Update TUI trace viewer to consume rlm-core events

3. Update budget panel to use cost tracking events

**Exit Criteria**:
- [ ] All event types map correctly
- [ ] TUI trace viewer renders events
- [ ] Budget tracking accurate

### Phase 6: Migrate Epistemic Verification

**Duration**: 3-4 days
**Risk**: Low

1. Replace `rlm/hallucination/`:
   ```go
   // Before
   detector := hallucination.NewDetector()
   result := detector.Check(claim, evidence)

   // After
   verifier := rlmcore.NewEpistemicVerifier(config)
   result := verifier.VerifyClaim(claim, evidence)
   ```

2. Replace `rlm/verify/`:
   - Proof checking → rlm-core verifier
   - Output gate → rlm-core memory gate

**Exit Criteria**:
- [ ] Hallucination detection rate maintained
- [ ] Memory gate rejects ungrounded facts
- [ ] Verification tests pass

### Phase 7: Migrate Reasoning Strategies

**Duration**: 4-5 days
**Risk**: Medium (complex logic)

1. Replace `rlm/tot/` (Tree of Thoughts):
   ```go
   // Before
   tot := tot.NewExplorer(config)
   result := tot.Explore(query, context)

   // After
   orchestrator := rlmcore.NewOrchestrator(config)
   result := orchestrator.ExecuteWithReasoning(query, context, ReasoningToT)
   ```

2. Replace `rlm/lats/` (Language Agent Tree Search):
   - Similar pattern to ToT

3. Replace `rlm/compress/`:
   - Hierarchical compression → rlm-core context module
   - Incremental compression → rlm-core context module

**Exit Criteria**:
- [ ] ToT produces equivalent exploration
- [ ] LATS search works correctly
- [ ] Compression maintains quality
- [ ] Reasoning tests pass

### Phase 8: Migrate Orchestrator

**Duration**: 1-2 weeks
**Risk**: High (core functionality)

1. Replace `rlm/orchestrator/` and `rlm/service.go`:
   ```go
   // Before
   service := rlm.NewService(config)
   result := service.Execute(request)

   // After
   orchestrator := rlmcore.NewOrchestrator(config)
   result := orchestrator.Execute(request)
   ```

2. Replace `rlm/wrapper.go`:
   - Prompt preparation → rlm-core
   - Response processing → rlm-core

3. Wire up application layer:
   - `app/rlm.go` → Use rlmcore.Orchestrator
   - Event dispatch to TUI

4. Handle async execution:
   - Parallel operations
   - Budget-aware execution

**Exit Criteria**:
- [ ] End-to-end orchestration works
- [ ] Recursive calls function correctly
- [ ] Meta-control decisions correct
- [ ] All orchestration tests pass

### Phase 9: Migrate Learning & Budget

**Duration**: 3-4 days
**Risk**: Low

1. Replace `learning/`:
   - Continuous learning → rlm-core memory evolution
   - Corrections → rlm-core memory metadata

2. Replace `budget/`:
   ```go
   // Before
   manager := budget.NewManager(limits)
   manager.Track(tokens)

   // After (via trajectory events)
   // Cost tracking integrated into orchestrator
   orchestrator.SetTrajectoryCallback(func(e TrajectoryEvent) {
       if e.Type == EventCostUpdate {
           updateBudgetUI(e.Cost)
       }
   })
   ```

**Exit Criteria**:
- [ ] Learning state migrated
- [ ] Budget tracking accurate
- [ ] Budget UI updates correctly

### Phase 10: Cleanup

**Duration**: 3-4 days
**Risk**: Low

1. Remove legacy packages (see Section 2.4)
2. Remove feature flags
3. Update go.mod dependencies
4. Update documentation
5. Update build scripts
6. Tag release

**Exit Criteria**:
- [ ] No legacy RLM code remains
- [ ] All tests pass
- [ ] Build clean without warnings
- [ ] Documentation updated

---

## 5. Testing Strategy

### 5.1 Test Categories

| Category | Approach |
|----------|----------|
| Unit tests | Run with both implementations during migration |
| Integration tests | Test FFI boundary thoroughly |
| Regression tests | Compare outputs between old and new |
| Performance tests | Benchmark critical paths |
| E2E tests | Full TUI interaction tests |

### 5.2 FFI-Specific Tests

```go
func TestFFIMemoryRoundtrip(t *testing.T) {
    store := rlmcore.NewMemoryStore(":memory:")
    defer store.Close()

    // Store node
    node := Node{ID: "test-1", Content: "test content"}
    err := store.Upsert(node)
    require.NoError(t, err)

    // Query back
    results, err := store.Query(Query{Content: "test"})
    require.NoError(t, err)
    require.Len(t, results, 1)
    assert.Equal(t, node.Content, results[0].Content)
}

func TestFFITrajectoryCallback(t *testing.T) {
    orchestrator := rlmcore.NewOrchestrator(config)
    defer orchestrator.Close()

    var events []TrajectoryEvent
    orchestrator.SetTrajectoryCallback(func(e TrajectoryEvent) {
        events = append(events, e)
    })

    _, err := orchestrator.Execute(request)
    require.NoError(t, err)

    assert.NotEmpty(t, events)
    assert.Equal(t, EventRlmStart, events[0].Type)
}
```

### 5.3 Regression Testing

```go
func TestRegressionOrchestration(t *testing.T) {
    // Run same query through both implementations
    oldResult := oldOrchestrator.Execute(query, context)
    newResult := newOrchestrator.Execute(query, context)

    // Compare outputs (allow for non-determinism)
    assertEquivalentResults(t, oldResult, newResult)
}
```

### 5.4 Performance Benchmarks

| Metric | Target |
|--------|--------|
| FFI call overhead | < 1ms per call |
| Memory query | < 200ms (semantic search) |
| Orchestration latency | < 50ms added overhead |
| Memory usage | < 10% increase |

---

## 6. Rollback Plan

Each phase includes rollback capability:

1. **Feature flag**: Set `RLM_USE_CORE=false` to revert
2. **Version pinning**: Keep old code until phase complete
3. **Database backup**: Backup memory before migration
4. **Git tags**: Tag before each phase for easy revert
5. **Dual binary**: Can build with or without rlm-core

---

## 7. Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| FFI memory leaks | High | Medium | Careful resource management, leak detection |
| CGO build complexity | Medium | Medium | CI/CD pipeline testing |
| Memory data loss | High | Low | Backup + migration script + verification |
| Performance regression | Medium | Medium | Benchmarks per phase |
| Callback deadlocks | High | Low | Careful lock management |
| TUI integration issues | Medium | Medium | Incremental integration |
| Cross-platform builds | Medium | High | Test on Linux, macOS, Windows |

---

## 8. Build Configuration

### 8.1 Cargo Configuration

```toml
# rlm-core/Cargo.toml
[features]
ffi = ["cbindgen"]

[lib]
crate-type = ["cdylib", "staticlib", "rlib"]
```

### 8.2 Go Build Configuration

```go
// build flags for different platforms
// +build cgo

/*
#cgo linux LDFLAGS: -L${SRCDIR}/lib -lrlm_core -ldl -lm -lpthread
#cgo darwin LDFLAGS: -L${SRCDIR}/lib -lrlm_core -ldl -lm -lpthread
#cgo windows LDFLAGS: -L${SRCDIR}/lib -lrlm_core -lws2_32 -luserenv
*/
import "C"
```

### 8.3 CI/CD Pipeline

```yaml
# .github/workflows/build.yml
jobs:
  build-rlm-core:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release --features ffi
      - uses: actions/upload-artifact@v4
        with:
          name: rlm-core-${{ matrix.os }}
          path: target/release/librlm_core.*

  build-recurse:
    needs: build-rlm-core
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/download-artifact@v4
      - run: go build ./cmd/recurse
```

---

## 9. Timeline Estimate

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Add dependency | 2-3 days | 2-3 days |
| Phase 2: Memory | 5-7 days | 7-10 days |
| Phase 3: Classification/Routing | 3-4 days | 10-14 days |
| Phase 4: REPL | 4-5 days | 14-19 days |
| Phase 5: Trajectory | 2-3 days | 16-22 days |
| Phase 6: Epistemic | 3-4 days | 19-26 days |
| Phase 7: Reasoning | 4-5 days | 23-31 days |
| Phase 8: Orchestrator | 7-10 days | 30-41 days |
| Phase 9: Learning/Budget | 3-4 days | 33-45 days |
| Phase 10: Cleanup | 3-4 days | 36-49 days |

**Total**: ~5-7 weeks

---

## 10. Commands Reference

```bash
# Build rlm-core shared library
cd /path/to/rlm-core
cargo build --release --features ffi

# Copy library to recurse
cp target/release/librlm_core.* /path/to/recurse/lib/

# Run with rlm-core enabled
RLM_USE_CORE=true go run ./cmd/recurse

# Run tests with both implementations
go test ./... -tags=rlmcore
go test ./... -tags=legacy

# Run memory migration
go run ./cmd/migrate-memory --old-db ~/.recurse/memory.db --new-db ~/.recurse/memory_v2.db

# Benchmark FFI overhead
go test ./internal/rlmcore/... -bench=. -benchmem

# Check for memory leaks
go test ./internal/rlmcore/... -race -count=100
```

---

## 11. Success Criteria

Migration is complete when:

- [ ] All recurse tests pass
- [ ] No regression in functionality
- [ ] Performance within 10% of original
- [ ] Memory migration works without data loss
- [ ] TUI renders correctly with rlm-core backend
- [ ] Cross-platform builds work (Linux, macOS, Windows)
- [ ] Feature flag removed
- [ ] Legacy RLM code deleted
- [ ] Documentation updated
- [ ] Release tagged

---

## 12. Module Removal Checklist

### Packages to Remove

```
internal/rlm/
├── orchestrator/           # → rlm-core orchestrator
│   ├── orchestrator.go
│   ├── core.go
│   ├── intelligent.go
│   ├── steering.go
│   └── checkpoint.go
├── meta/                   # → rlm-core orchestrator
├── routing/                # → rlm-core SmartRouter
├── repl/                   # → rlm-core ReplPool
├── tot/                    # → rlm-core reasoning
├── lats/                   # → rlm-core reasoning
├── compress/               # → rlm-core context
├── cache/                  # → rlm-core LLM
├── async/                  # → rlm-core
├── learning/               # → rlm-core memory
├── hallucination/          # → rlm-core epistemic
├── verify/                 # → rlm-core epistemic
├── synthesize/             # → rlm-core
├── decompose/              # → rlm-core
├── checkpoint/             # → rlm-core
├── observability/          # → rlm-core trajectory
├── service.go              # → rlmcore bridge
├── wrapper.go              # → rlm-core
├── classifier.go           # → rlm-core PatternClassifier
├── compute.go              # → rlm-core
├── proactive.go            # → rlm-core
├── guarantees.go           # → rlm-core
└── progress.go             # → rlm-core trajectory

internal/memory/
├── hypergraph/             # → rlm-core SqliteMemoryStore
├── embeddings/             # → rlm-core memory
├── tiers/                  # → rlm-core Tier
└── reasoning/              # → rlm-core ReasoningTraceStore

internal/budget/            # → rlm-core CostTracker (core logic only)
```

### Files to Keep (estimated count after migration)

```
internal/
├── rlmcore/        # NEW: FFI bridge (~10 files)
├── tui/            # KEEP: 100+ files
├── app/            # KEEP: ~10 files (adapted)
├── agent/          # KEEP: ~15 files
├── cmd/            # KEEP: ~10 files
├── config/         # KEEP: ~5 files
├── db/             # KEEP: ~10 files
├── oauth/          # KEEP: ~5 files
├── lsp/            # KEEP: ~5 files
├── shell/          # KEEP: ~3 files
├── session/        # KEEP: ~5 files (adapted)
├── message/        # KEEP: ~5 files (adapted)
└── [utilities]     # KEEP: ~20 files
```

**Estimated reduction**: ~200+ Go files removed, ~180 files remaining
