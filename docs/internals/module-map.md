# Internal Module Map

This is a practical index for "where should I look first?".

## Core Runtime (`/Users/rand/src/loop/rlm-core/src`)

### Execution and Orchestration
- `orchestrator.rs`: recursion, execution strategy, synthesis control
- `complexity.rs`: activation heuristics and mode selection
- `repl.rs`: sandboxed execution orchestration

### Context and Signatures
- `context/`: session state, externalization, typed prompt plumbing
- `signature/`: typed signatures, validation, fallback extraction, submit semantics

### LLM Layer
- `llm/client.rs`: provider calls
- `llm/router.rs`: model selection/routing
- `llm/batch.rs`: batched query execution
- `llm/cache.rs`: cache behavior

### Memory and Reasoning
- `memory/`: hypergraph persistence
- `reasoning/trace.rs`: decision graph representation
- `reasoning/query.rs`: trace analysis and traversal

### Formalization and Sync
- `spec_agent/`: NL intake/refine/formalize/verify workflow
- `sync/`: Topos/Lean drift detection and dual-track operations

### Assurance and Proof
- `epistemic/`: claim verification/confidence analysis
- `proof/`: Lean-oriented proof workflows and tactics
- `dp_integration/`: quality review/coverage/proof status tooling

### Integrations
- `adapters/`: external integrations (Claude Code, TUI, CLI)
- `ffi/`, `pybind/`, `go/`: language interoperability surfaces

## Common Debug Entry Points

1. Activation behavior wrong?
- Start at `complexity.rs`, then `orchestrator.rs`.

2. Structured output broken?
- Start at `signature/validation.rs`, `signature/submit.rs`, `signature/fallback.rs`.

3. Drift reports odd?
- Start at `sync/drift.rs`, then `sync/types.rs`.

4. Memory inconsistency?
- Start at `memory/store.rs`, then `reasoning/store.rs`.

## Editing Guidelines

1. Keep changes scoped to one concern where possible.
2. Add tests in same area as changed behavior.
3. Update docs for behavior changes in same commit.
4. Preserve integration contracts unless explicitly changing them.

A map is not the territory, but it does reduce wandering with a flashlight.
