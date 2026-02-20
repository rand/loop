# OODA and Execution Flow

This document maps Loop runtime behavior to Observe/Orient/Decide/Act.

## Observe

Inputs gathered:
- User intent
- Session context
- File/tool outputs
- Prior memory and trace data (when relevant)

Key modules:
- `context`
- `memory` (read paths)

## Orient

Analysis performed:
- Complexity signal extraction
- Domain/workflow inference
- Constraint and budget awareness

Key modules:
- `complexity`
- `spec_agent::parser` (for NL formalization flows)

## Decide

Decision points:
- Activate or bypass recursive orchestration
- Select model/routing path
- Select formalization depth and verification posture
- Select fallback behavior under failure/limits

Key modules:
- `orchestrator`
- `llm::router`
- `signature` and fallback extraction paths

## Act

Actions executed:
- Run REPL/module operations
- Perform LLM calls
- Emit trajectory events
- Persist memory/traces
- Run verification and governance checks

Key modules:
- `repl`
- `module`
- `reasoning`
- `dp` governance wrapper (`./scripts/dp`)

## Observability Expectations

A healthy execution path produces:
1. Traceable decisions.
2. Deterministic command evidence for gates.
3. Recoverable diagnostics when failures occur.

If one of these is missing, the system may still run, but operating it safely becomes mostly archaeology.
