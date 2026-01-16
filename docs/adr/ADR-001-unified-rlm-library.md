# ADR-001: Unified RLM Library Architecture

**Status**: Proposed
**Date**: 2026-01-15
**Author**: Claude (with Rand)
**Context**: Design unified library for Claude Code plugin + Agentic TUI

## Context

We have two systems implementing RLM (Recursive Language Model) capabilities:

1. **recurse** - A Go-based agentic TUI extending Charmbracelet Crush with RLM orchestration, tiered hypergraph memory, and embedded reasoning traces

2. **rlm-claude-code** - A Python-based Claude Code plugin providing RLM orchestration, complexity classification, smart routing, and trajectory streaming

Both systems share significant conceptual overlap but are implemented separately, leading to:
- Duplicated effort for new features
- Feature drift between implementations
- Inconsistent behavior across deployment targets
- Maintenance burden

## Decision

We will create **rlm-core**, a unified Rust library with Python (PyO3) and Go (CGO) bindings that provides the foundational RLM capabilities for both deployment targets.

### Key Architectural Decisions

#### 1. Implementation Language: Rust

**Decision**: Core library in Rust with language bindings.

**Rationale**:
- Performance-critical operations (REPL sandbox, memory queries, trajectory streaming)
- Memory safety for sandboxed execution
- Single implementation serves both targets
- PyO3 provides excellent Python integration
- CGO enables Go integration

**Alternatives Rejected**:
- *Pure Python*: Would need rewriting for TUI
- *Pure Go*: Would need rewriting for Claude Code
- *Dual implementation*: Maintenance burden, feature drift

#### 2. REPL Architecture: External Subprocess

**Decision**: Python REPL as external subprocess with JSON-RPC IPC.

**Rationale**:
- Process isolation prevents crashes from affecting host
- OS-level resource limits (memory, CPU, time)
- Full Python ecosystem access
- RestrictedPython has limitations for sandboxing

**Alternatives Rejected**:
- *Embedded interpreter*: Crash affects host, harder to limit
- *WASM Python*: Limited ecosystem, performance issues
- *Starlark*: Limited expressiveness

#### 3. Memory System: Unified Hypergraph

**Decision**: Full hypergraph memory with 3-tier evolution, unifying both systems.

**Rationale**:
- Recurse's hypergraph is more expressive than rlm-claude-code's simpler store
- Tier system (task/session/longterm) works for both contexts
- Evolution operations prevent unbounded growth
- Reasoning traces integrate naturally as memory nodes
- SQLite provides portability

**Schema**:
```
nodes(id, type, subtype, content, embedding, tier, confidence, provenance)
hyperedges(id, type, label, weight)
membership(hyperedge_id, node_id, role, position)
```

#### 4. Model Routing: Query-Aware Selection

**Decision**: Unified smart router considering query type, recursion depth, and budget.

**Rationale**:
- Both systems benefit from model tiering
- Query classification helps select appropriate model
- Depth-based routing (Opus → Sonnet → Haiku) optimizes cost

**Routing Strategy**:
| Query Type | Depth 0 | Depth 1 | Depth 2+ |
|------------|---------|---------|----------|
| Architecture | Opus | Sonnet | Sonnet |
| Multi-file | Opus | Sonnet | Haiku |
| Debugging | Sonnet | Haiku | Haiku |
| Extraction | Sonnet | Haiku | Haiku |
| Simple | Haiku | Haiku | Haiku |

#### 5. Trajectory System: Unified Event Schema

**Decision**: Single event schema, context-specific rendering.

**Rationale**:
- Same events can render differently per context
- Claude Code: Streaming text output
- TUI: Bubble Tea panel updates
- Both: JSON export for analysis/replay

**Event Types**:
```rust
enum TrajectoryEventType {
    RlmStart, Analyze, ReplExec, ReplResult,
    Reason, RecurseStart, RecurseEnd,
    Final, Error, ToolUse, CostReport
}
```

## Consequences

### Positive

- **Single source of truth** for RLM logic
- **Consistent behavior** across Claude Code and TUI
- **Shared improvements** benefit both targets
- **Reduced maintenance** burden
- **Better testing** - core tested once, adapters tested separately

### Negative

- **FFI complexity** - Rust ↔ Python and Rust ↔ Go boundaries
- **Build complexity** - Multi-language build system
- **Migration effort** - Both existing systems need updates
- **Learning curve** - Team needs Rust familiarity

### Neutral

- Performance characteristics will differ between targets due to binding overhead
- Some platform-specific optimizations may be harder

## Implementation Plan

1. **Phase 1** (2-3 weeks): Core engine - types, REPL, basic orchestration
2. **Phase 2** (2 weeks): Memory system - hypergraph, tiers, evolution
3. **Phase 3** (1-2 weeks): LLM client - multi-provider, smart routing
4. **Phase 4** (1 week): Python bindings via PyO3
5. **Phase 5** (1 week): Go bindings via CGO
6. **Phase 6** (2 weeks): Full adapters for both targets

## Related Documents

- [Unified RLM Library Design](/docs/unified-rlm-library-design.md)
- [recurse SPEC.md](https://github.com/rand/recurse/blob/main/docs/SPEC.md)
- [rlm-claude-code-spec.md](https://github.com/rand/rlm-claude-code/blob/main/rlm-claude-code-spec.md)
