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
- `adapters::claude_code::adapter` (request-context ingestion + memory lookup)

## Orient

Analysis performed:
- Complexity signal extraction
- Domain/workflow inference
- Constraint and budget awareness

Key modules:
- `complexity`
- `adapters::claude_code::hooks` (prompt analysis and signal propagation)
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
- `adapters::claude_code::adapter` (activate/skip + mode selection bridge)
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
- `adapters::claude_code::adapter` (execution + metadata shaping)
- `module`
- `reasoning`
- `dp` governance wrapper (`./scripts/dp`)

## Observability Expectations

A healthy execution path produces:
1. Traceable decisions.
2. Deterministic command evidence for gates.
3. Recoverable diagnostics when failures occur.
4. Scenario-level adapter efficacy evidence (`VG-CLAUDE-ADAPTER-E2E-001`), not just plumbing tests.

If one of these is missing, the system may still run, but operating it safely becomes mostly archaeology.
