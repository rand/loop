# Claude Code Adapter Guide

This guide explains what the Claude adapter is good at, where it is intentionally limited, and how to use it without treating it like a magical mystery box.

## What It Is

`ClaudeCodeAdapter` is the deployment adapter that turns loop runtime components into a Claude-compatible execution surface:
- Complexity-based activation
- Context externalization (messages/files/tool outputs/working memory)
- REPL-backed execution and structured output
- Memory query/store integration
- Mode-aware routing and cost accounting metadata

If `orchestrator` is the engine, the adapter is the transmission.

## Primary Jobs To Be Done

1. **Complex prompt triage**

Input: A prompt that is broad, cross-file, security/architecture heavy, or explicitly thorough.

Expected behavior:
- Adapter activates RLM path.
- Complexity signals are attached to response metadata.
- Mode can auto-escalate (`fast` -> `thorough`) when strong signals are present.

2. **Context-rich execution without prompt bloat**

Input: Messages + files + tool outputs + working memory.

Expected behavior:
- Context is externalized into REPL variables.
- Root prompt remains concise.
- Response shows context/memory usage metadata.

3. **Memory-assisted analysis**

Input: Query plus prior facts/experience in memory store.

Expected behavior:
- Adapter performs bounded memory search.
- Results influence the execution summary.
- Metadata records memory query count and usage flags.

## OODA Mapping (Adapter View)

- **Observe**: ingest request context and memory hits
- **Orient**: classify complexity, extract active signals
- **Decide**: activate/skip RLM and choose execution mode
- **Act**: run REPL flow, emit answer + cost/trace metadata

Reference tests:
- `test_execute_e2e_incident_triage_ooda_flow`
- `test_execute_e2e_fast_path_skip_with_context_noise`

## Capability Boundaries

The adapter can:
- Produce deterministic activate/skip decisions for common query classes
- Preserve structured metadata about why it did what it did
- Integrate memory and context into execution without dumping raw context into one giant prompt

The adapter does not:
- Guarantee perfect complexity classification on every phrasing
- Replace policy gates or evidence capture
- Infer external truth it has not been given (still subject to model/runtime limits)
- Run infinite orchestration loops just because the prompt is dramatic

## Practical Limits and Tradeoffs

1. **Activation is heuristic**
- Pattern-based scoring is transparent and fast.
- It can still miss edge phrasing. Use `force_activation` when precision matters more than cost.

2. **Mode escalation is signal-driven**
- Strong signals (`architecture_analysis`, `user_thorough`, exhaustive search) push toward deeper modes.
- If the user asks for speed (`quick`, `brief`, `only`), fast-path behavior is favored.

3. **Memory search is bounded**
- Adapter uses limited memory fetch size by design.
- If critical evidence is missing from memory, execution quality drops to whatever context still contains.

4. **Cost accounting is operational, not billing-grade finance**
- Metadata is meant for engineering control and regression detection.
- It is not an invoice.

## Validation Gate

Run:

```bash
make claude-adapter-gate
```

This executes `VG-CLAUDE-ADAPTER-E2E-001`, which requires:
- Scenario tests pass
- At least 2 scenario tests execute (filter-drift guardrail)

Because "0 tests ran" is not a confidence strategy.

## When To Escalate Beyond Adapter Defaults

Escalate when:
- Incident triage is high-stakes and ambiguity is costly
- You need deeper proofs/contracts than adapter runtime summaries
- Consumer compatibility claims are changing (run full compatibility gates)

See also:
- `/Users/rand/src/loop/docs/internals/ooda-and-execution.md`
- `/Users/rand/src/loop/docs/developer-guide/quality-gates.md`
- `/Users/rand/src/loop/docs/troubleshooting/common-issues.md`
