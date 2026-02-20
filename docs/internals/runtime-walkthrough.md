# Runtime Walkthrough

This is an end-to-end trace of a typical Loop request.

Use it when you need to answer: "what happened, where, and why?"

## Scope

This walkthrough covers:

1. Request intake and context loading.
2. Complexity scoring and mode decision.
3. Execution through orchestrator, REPL, and LLM layers.
4. Trace, memory, and evidence persistence.
5. Final response shaping for adapter surfaces.

## Step 0: Entry Surface

Common entry points:

- Rust API callers.
- Adapter surfaces in `rlm-core/src/adapters/`.
- Binding callers via `pybind/`, `ffi/`, and Go wrappers.

Start debugging in:
- `rlm-core/src/adapters/claude_code/adapter.rs`
- `rlm-core/src/orchestrator.rs`

## Step 1: Observe (Input Ingestion)

Inputs typically include:

1. Prompt text.
2. Session message history.
3. Files and tool outputs.
4. Memory query results, if enabled.

Relevant modules:
- `rlm-core/src/context/`
- `rlm-core/src/adapters/claude_code/adapter.rs`
- `rlm-core/src/memory/`

Expected outputs of this step:
- Structured request context.
- Candidate signals for complexity analysis.

## Step 2: Orient (Complexity and Strategy Inputs)

The runtime classifies prompt characteristics and computes activation signals.

Relevant modules:
- `rlm-core/src/complexity.rs`
- `rlm-core/src/adapters/claude_code/hooks.rs`

Typical signals:
- Multi-file scope.
- Architecture analysis intent.
- User thoroughness markers.
- Request for speed-only answers.

Expected outputs of this step:
- Activation recommendation.
- Mode preference (`fast`, `balanced`, `thorough` style posture).

## Step 3: Decide (Execution Path Selection)

Decision points:

1. Use recursive orchestration or fast-path response.
2. Select model routing strategy.
3. Select fallback behavior if strict output parsing fails.

Relevant modules:
- `rlm-core/src/orchestrator.rs`
- `rlm-core/src/llm/router.rs`
- `rlm-core/src/signature/validation.rs`
- `rlm-core/src/signature/fallback.rs`

Expected outputs of this step:
- Concrete execution plan.
- Route metadata and budget posture.

## Step 4: Act (Execution)

Execution may involve:

1. REPL-backed context operations.
2. LLM calls or batched calls.
3. Module-level transformations.
4. Structured output generation.

Relevant modules:
- `rlm-core/src/repl.rs`
- `rlm-core/src/llm/client.rs`
- `rlm-core/src/llm/batch.rs`
- `rlm-core/src/module/`

Expected outputs of this step:
- Candidate response payload.
- Execution metadata (costs, signals, timing).

## Step 5: Persist (Traces, Memory, Evidence)

The runtime records what happened for later debugging and learning.

Relevant modules:
- `rlm-core/src/reasoning/trace.rs`
- `rlm-core/src/reasoning/store.rs`
- `rlm-core/src/memory/store.rs`
- `rlm-core/src/trajectory.rs`

Expected outputs of this step:
- Traceable execution events.
- Optional memory updates.
- Recoverable diagnostics context.

## Step 6: Return (Adapter Response Shaping)

Response adapters shape final outputs with metadata suitable for callers.

Relevant modules:
- `rlm-core/src/adapters/claude_code/adapter.rs`
- `rlm-core/src/adapters/tui/adapter.rs`

Expected outputs of this step:
- User-facing result.
- Metadata about mode, signals, and usage.

## Fast Debug Procedure

When behavior looks wrong, run:

```bash
cd /Users/rand/src/loop
rg -n "should_activate|mode|fallback|memory" rlm-core/src/adapters rlm-core/src/orchestrator.rs rlm-core/src/complexity.rs
```

Then:

1. Confirm signal extraction path.
2. Confirm decision branch selected.
3. Confirm execution path produced expected metadata.
4. Confirm persistence path did not drop critical context.

## Common Failure Shapes

1. Wrong activation choice:
   - Inspect `complexity.rs` and adapter hook signal propagation.
2. Correct decision, wrong output shape:
   - Inspect signature validation and fallback extraction.
3. Correct output, missing traceability:
   - Inspect trajectory and reasoning persistence paths.
4. Tests pass, integration still odd:
   - Run scenario gate `make claude-adapter-gate`.

## Related Docs

- `architecture.md`
- `module-map.md`
- `ooda-and-execution.md`
- `../troubleshooting/incident-playbook.md`

The system is complex, not mysterious. This walkthrough exists to keep it that way.
