# ADR-003: DSPy-Inspired Improvements for rlm-core

**Status**: Proposed
**Date**: 2026-01-20
**Deciders**: rand, Claude
**Epic**: loop-zcx

## Context

Analysis of three external RLM implementations revealed significant improvement opportunities for rlm-core:

1. **DSPy RLM** (`stanfordnlp/dspy`) - Typed signatures, SUBMIT mechanism, module composition, automatic optimization
2. **Codecrack3 RLM-DSPy** - Context-as-variable pattern, dual-model optimization, graph visualization
3. **Numina-lean-agent** - Single-target proof protocol, NL prohibition, lean diagnostic feedback

Current rlm-core has strong foundations (epistemic verification, tiered memory, multi-language FFI) but lacks DSPy's composability and optimization patterns.

## Decision

We will implement **5 major improvements** organized into 8 specs:

### 1. Typed Signatures System (SPEC-20, P0)

**Decision**: Implement DSPy-style typed signatures with Rust derive macros.

**Rationale**:
- Enables automatic prompt generation from type annotations
- Enables output validation before returning to user
- Foundation for module composition and optimization
- Rust's type system provides compile-time guarantees

**Alternative Considered**: Runtime DSL like DSPy's string signatures
- Rejected: Rust's compile-time types are stronger; derive macros provide better DX

### 2. SUBMIT Mechanism (SPEC-20.07-10, P0)

**Decision**: Implement SUBMIT() function in REPL for structured output termination.

**Rationale**:
- Clean termination with validated outputs
- Separates execution from output collection
- Enables fallback extraction when SUBMIT not called

### 3. Dual-Model Optimization (SPEC-21, P1)

**Decision**: Explicit dual-model configuration with premium root + budget recursive models.

**Rationale**:
- 30-50% cost reduction demonstrated by Codecrack3
- SmartRouter already exists; this adds explicit configuration
- User control over cost/quality tradeoff

**Model Tiers**:
| Tier | Model | Use Case |
|------|-------|----------|
| Root | Opus | Orchestration decisions |
| Recursive | Sonnet/Haiku | Sub-queries, extraction |
| Extraction | Haiku | Fallback extraction |

### 4. Single-Target Proof Protocol (SPEC-22, P1)

**Decision**: Implement Numina-style focused proof sessions for Lean integration.

**Rationale**:
- Prevents combinatorial proof explosion
- Numina proved all 12 Putnam 2025 problems with this approach
- Aligns with existing Lean REPL infrastructure

**Key Constraints**:
- One sorry per session
- Helper lemmas tracked with attribution
- Natural language prohibition (>42 line comments rejected)

### 5. Graph Visualization (SPEC-23, P2)

**Decision**: Add NetworkX-compatible export and interactive HTML visualization for ReasoningTrace.

**Rationale**:
- Essential for debugging complex traces
- Existing Mermaid export is static; need interactive
- D3.js provides mature graph visualization

### 6. BootstrapFewShot Optimizer (SPEC-24, P2)

**Decision**: Implement DSPy-style automatic prompt optimization.

**Rationale**:
- Metric-driven prompt improvement
- Foundation for production-ready modules
- Requires signatures (P0) first

### 7. Context Externalization (SPEC-25, P2)

**Decision**: Enforce context-as-variable pattern to prevent context rot.

**Rationale**:
- 80% accuracy vs 0% on 132k-token tasks (Codecrack3)
- 97% token reduction in prompts
- Enables handling of 150k+ token contexts

### 8. Supporting Features (SPEC-26-27, P2)

- **Batched LLM Queries**: Parallel execution for map-reduce patterns
- **Fallback Extraction**: Graceful output when execution limits reached

## Architecture

### Component Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                 Optimizers (SPEC-24)                 │    │
│  │  BootstrapFewShot, MIPROv2, etc.                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Module Composition (SPEC-20)            │    │
│  │  Predict<S>, Compose<A,B>, Pipeline                 │    │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                 │
├─────────────────────────────────────────────────────────────┤
│                      Core Layer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Signatures │  │   REPL      │  │   Orchestrator      │  │
│  │  (SPEC-20)  │  │  + SUBMIT   │  │  + DualModel        │  │
│  │  + Derive   │  │  + Batched  │  │  + Fallback         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Existing rlm-core                       │    │
│  │  Memory, Trajectory, Epistemic, LLM Client          │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    Proof Layer (Lean)                        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Proof Protocol (SPEC-22)                │    │
│  │  ProofSession, SorryLocation, HelperLemma           │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Dependency Graph

```
SPEC-20 (Signatures)
    │
    ├──► SPEC-20 (Derive Macro)
    │        │
    │        └──► SPEC-20 (Module Composition)
    │                  │
    │                  └──► SPEC-24 (BootstrapFewShot)
    │
    ├──► SPEC-20 (SUBMIT)
    │        │
    │        └──► SPEC-27 (Fallback Extraction)
    │
    └──► SPEC-21 (Dual-Model) [independent]

SPEC-22 (Proof Protocol) [independent]

SPEC-23 (Graph Viz) [independent]

SPEC-25 (Context Externalization) [independent]

SPEC-26 (Batched Queries) [independent]
```

## Implementation Plan

### Phase 1: Foundations (Weeks 1-3)
- SPEC-20.01-03: Signature trait and types
- SPEC-20.04-06: Derive macro
- SPEC-20.07-10: SUBMIT mechanism

### Phase 2: Optimization (Weeks 3-5)
- SPEC-21: Dual-model configuration
- SPEC-25: Context externalization
- SPEC-26: Batched queries

### Phase 3: Lean Integration (Weeks 5-7)
- SPEC-22: Single-target proof protocol
- Integration with existing Lean REPL

### Phase 4: Advanced (Weeks 7-10)
- SPEC-20.11-13: Module composition
- SPEC-23: Graph visualization
- SPEC-24: BootstrapFewShot optimizer
- SPEC-27: Fallback extraction

## Consequences

### Positive

1. **Composability**: Signatures enable type-safe module composition
2. **Optimization**: BootstrapFewShot enables automatic prompt improvement
3. **Cost Reduction**: Dual-model saves 30-50% on complex queries
4. **Scale**: Context externalization handles 150k+ tokens
5. **Debugging**: Graph visualization aids trace debugging
6. **Proof Quality**: Single-target protocol prevents proof explosion

### Negative

1. **Complexity**: Additional abstraction layers
2. **Learning Curve**: New concepts (signatures, modules, optimizers)
3. **Migration**: Existing code needs adaptation to use signatures

### Neutral

1. **Proc Macro Crate**: Requires separate `rlm-core-derive` crate
2. **Python REPL Changes**: SUBMIT function addition
3. **Backward Compatibility**: Old code continues to work; new patterns opt-in

## Migration Strategy

1. **Phase 1**: Add signatures as optional; existing code unchanged
2. **Phase 2**: Migrate internal modules to use signatures
3. **Phase 3**: Deprecate non-signature patterns
4. **Phase 4**: Full signature-based API

## Success Metrics

| Metric | Target | Spec |
|--------|--------|------|
| Signature coverage | >80% of modules | SPEC-20 |
| Cost reduction (dual-model) | 30-50% | SPEC-21 |
| Context handling | 150k+ tokens | SPEC-25 |
| Proof success rate | >70% simple theorems | SPEC-22 |
| Optimization improvement | >10% on benchmarks | SPEC-24 |

## References

- [DSPy RLM](https://github.com/stanfordnlp/dspy/blob/main/dspy/predict/rlm.py)
- [Codecrack3 RLM-DSPy](https://github.com/codecrack3/Recursive-Language-Models-RLM-with-DSpy)
- [Numina-lean-agent](https://github.com/project-numina/numina-lean-agent)
- [DSPy Signatures](https://github.com/stanfordnlp/dspy/blob/main/dspy/signatures/signature.py)
- [DSPy BootstrapFewShot](https://dspy.ai/api/optimizers/BootstrapFewShot/)
- SPEC-20 through SPEC-27 in `docs/spec/`
