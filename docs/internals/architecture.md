# Internal Architecture

## System Overview

Loop is a layered orchestration system:

1. **Entry and Context Layer**
- Session context capture
- Prompt/task intake
- Complexity analysis inputs

2. **Decision Layer**
- Pattern classification
- Activation decision
- Mode and budget strategy selection

3. **Execution Layer**
- Orchestrator recursion/decomposition
- REPL-backed module execution
- LLM routing and cost tracking

4. **Persistence and Evidence Layer**
- Memory graph storage
- Reasoning traces
- Validation outputs and governance evidence

5. **Integration Layer**
- Rust API
- Python bindings
- Go bindings
- Adapter surfaces (Claude Code, TUI, etc.)

## Primary Runtime Components

| Component | Responsibility |
|---|---|
| `context` | Session state, messages, and externalized context |
| `complexity` | Pattern-based activation signals |
| `orchestrator` | Recursive execution and synthesis |
| `repl` | Safe executable surface for generated code/flows |
| `llm` | Provider clients, routing, and cost accounting |
| `memory` | Persistent knowledge and experience graph |
| `reasoning` | Decision traces and graph analysis |
| `spec_agent` | NL -> Topos/Lean workflow |
| `sync` | Topos/Lean drift detection and synchronization |

## Runtime Invariants

1. Every nontrivial execution should be explainable via trajectory/events.
2. Policy gates are enforceable from repository-local runtime commands.
3. Behavior-affecting changes require tests and documentation in the same change set.
4. Integration surfaces should remain compatibility-aware unless explicitly changed.

## Data Flow (High Level)

1. User/task input arrives.
2. Context + complexity signals are computed.
3. Activation decision selects path.
4. Execution path runs modules/LLM/REPL actions.
5. Outputs, traces, and memory updates are persisted.
6. Governance and validation gates assert correctness for ship decisions.

## Design Notes

- Loop favors explicit evidence over implicit confidence.
- Internal flexibility is allowed; release surfaces are contract-sensitive.
- If a behavior cannot be validated, it is not done. It is merely enthusiastic.

Related:
- `runtime-walkthrough.md` for request lifecycle details.
- `module-map.md` for file-level entry points.
