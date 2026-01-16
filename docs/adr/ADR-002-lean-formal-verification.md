# ADR-002: Lean Formal Verification Integration

## Status

Proposed

## Date

2026-01-16

## Context

We need to add formal specification and verification capabilities to rlm-core. The goals are:

1. **Interactive theorem proving** - Allow users to develop and verify formal proofs
2. **Specification validation** - Verify that formal specs are consistent and type-check
3. **Implementation verification** - Prove properties about algorithms and systems
4. **AI-assisted specification** - Help users create and refine formal specifications

Current state:
- rlm-core has a Python REPL with `ReplEnvironment` trait for extension
- We have Topos as a semantic contract language for human-AI collaboration
- AI proof assistants (DeepSeek-Prover, LeanDojo) show LLMs can generate proofs effectively

## Decision

We will implement:

### 1. Lean REPL Integration

**Decision**: Add a Lean 4 REPL as a new `ReplEnvironment` implementation using the same subprocess + JSON-RPC pattern as the Python REPL.

**Rationale**:
- [leanprover-community/repl](https://github.com/leanprover-community/repl) already provides JSON protocol
- Same architecture as Python REPL ensures consistency
- Environment pickling allows proof state persistence
- Subprocess isolation prevents crashes from affecting host

**Alternatives considered**:
- *Embedded Lean via FFI*: Too complex, Lean's runtime is substantial
- *HTTP API*: More overhead, no environment persistence
- *LSP only*: Less flexible for programmatic interaction

### 2. Spec Agent

**Decision**: Create a specialized agent for specification creation and refinement with first-class Topos integration.

**Rationale**:
- Specifications are the bottleneck—proofs can be automated
- Human-AI collaboration on specs is more valuable than pure AI generation
- Topos provides the semantic contract layer; Lean provides formal verification
- Dual-track approach gives both human readability and machine verifiability

**Alternatives considered**:
- *Lean-only specs*: Too hard for non-experts to read/review
- *Topos-only specs*: No machine verification
- *Separate tools*: Loses traceability and sync benefits

### 3. Dual-Track Sync (Topos ↔ Lean)

**Decision**: Maintain bidirectional links between Topos specs and Lean formalizations with drift detection.

**Rationale**:
- Humans review Topos (readable), machines verify Lean (precise)
- Links ensure they stay in sync
- Drift detection catches divergence early
- Progressive formalization allows starting simple

**Implementation**:
- `@lean` annotations in Topos link to Lean artifacts
- `@topos` comments in Lean link back
- Sync engine generates Lean from Topos changes
- Drift detection compares semantically

### 4. Progressive Proof Automation

**Decision**: Four-tier automation strategy: decidable → automation → AI-assisted → human-in-loop.

**Rationale**:
- Many proofs are trivial (decidable tactics handle instantly)
- Automation tactics (aesop, linarith) handle common patterns
- AI proof search works because invalid proofs are rejected
- Human fallback ensures no blocking on hard proofs

**Strategy**:
1. **Decidable**: decide, native_decide, omega, simp (instant)
2. **Automation**: aesop, linarith, ring (seconds)
3. **AI-assisted**: LLM generates tactics, type checker validates (seconds-minutes)
4. **Human-in-loop**: sorry with TODO, create task (async)

### 5. Formalization Levels

**Decision**: Support configurable formalization levels: Types → Types+Invariants → Contracts → Full Proofs.

**Rationale**:
- Not all specs need full proofs initially
- Progressive refinement matches real development
- Default to Full Proofs per user requirement
- Allow relaxation when proofs are too expensive

### 6. Domain-Specific Strategies

**Decision**: Support multiple domains with specialized proof strategies.

**Domains**:
- Algorithms & data structures (induction, recursion)
- Distributed systems (bisimulation, invariants)
- APIs & protocols (state transitions, session types)
- Security properties (information flow, access control)
- Application flow (termination, safety)

## Consequences

### Positive

- Formal verification becomes accessible through AI assistance
- Specifications are both human-readable (Topos) and machine-verifiable (Lean)
- Traceability from requirements to proofs
- Progressive automation handles most proofs automatically
- Learning from successful proof strategies improves over time

### Negative

- Lean dependency adds complexity (build system, versions, mathlib)
- Dual-track sync requires maintenance
- AI proof generation has latency and cost
- Some proofs will always need human expertise

### Risks

| Risk | Mitigation |
|------|------------|
| Lean version fragmentation | Per-project version via elan |
| Mathlib size (2GB+) | Shared cache, lazy download |
| Proof maintenance burden | Drift detection, automated updates |
| AI proof quality | Type checker validation, human review |

## Implementation

See [Lean Formal Verification Design](../lean-formal-verification-design.md) for detailed design.

### Phases

1. **Lean REPL Foundation** (2-3 weeks) - Core REPL, project management
2. **Topos Integration** (1-2 weeks) - MCP client, link annotations
3. **Dual-Track Sync** (2 weeks) - Drift detection, sync engine
4. **Spec Agent** (2-3 weeks) - Intake, refine, formalize phases
5. **Proof Automation** (2-3 weeks) - Progressive automation tiers
6. **DP Integration** (1-2 weeks) - Spec coverage, proof tracking

### Success Metrics

- Lean REPL executes commands correctly
- >70% auto-proof rate on simple theorems
- Drift detection catches spec divergence
- DP integration tracks formal spec coverage

## References

- [leanprover-community/repl](https://github.com/leanprover-community/repl)
- [Topos](https://github.com/rand/topos)
- [Martin Kleppmann on AI + FV](https://martin.kleppmann.com/2025/12/08/ai-formal-verification.html)
- [LeanDojo](https://leandojo.org/)
- [ADR-001: Unified RLM Library](./ADR-001-unified-rlm-library.md)
