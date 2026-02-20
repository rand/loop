# Python Orchestrator Swap Analysis

Date: 2026-02-20
Audience: Maintainers and integration owners (`rlm-claude-code`, `loop-agent`, `io-rflx`)

## Question

Can we get the full value of `loop` and `rlm-core` in downstream projects without doing a full Python-side orchestrator swap?

Short answer: you can get most of the practical value now, but not the full architectural unification value.

## Why This Matters

The core tension is simple:

1. We already have strong shared primitives and adapter/runtime behavior in `rlm-core`.
2. Python consumers still orchestrate some flows locally because binding surfaces are intentionally limited today.

This means we can move quickly now, but we still carry some long term drift risk.

## Current Implementation Reality

### What Python bindings expose today

Python currently exposes component-level surfaces such as context, memory, router, classifier, trajectory, and epistemic primitives.

It does not expose:

1. `Orchestrator`
2. `ClaudeCodeAdapter`
3. `ReplHandle` and `ReplPool`

This is documented and currently accepted as scope policy.

### What already exists in Rust

`rlm-core` already has:

1. A real `ClaudeCodeAdapter` runtime
2. OODA-oriented end-to-end tests
3. Orchestrator traits and fallback/runtime primitives
4. REPL handle/pool implementations

So the gap is primarily at cross-language exposure and runtime boundary semantics, not at core capability existence.

## Value You Can Get Right Now Without Full Swap

### `rlm-claude-code`

You can get substantial value now via component delegation:

1. Shared classifier behavior
2. Shared memory semantics
3. Shared routing and cost primitives
4. Shared trajectory and epistemic primitives

What you do not get yet:

1. One canonical orchestrator runtime path across Rust and Python
2. Fully unified lifecycle semantics around orchestration and REPL control
3. Single-place orchestration policy changes

### `loop-agent`

Current integration is seam-oriented and optional by contract, so a Python orchestrator swap is not required to get most near-term value.

The current value path is deterministic optional adapters with strict fallback behavior.

### `io-rflx`

Current integration is schema-first and contract-driven. It does not require a Python orchestrator swap to deliver value.

The primary leverage is stable trajectory/provenance/verification envelopes and versioned mapping policy.

## What Full Swap Actually Means

A full Python-side orchestrator swap means Python consumers invoke Rust orchestration runtime surfaces directly for the full OODA loop, not just delegated components.

In practice, this includes:

1. Exposing adapter and/or orchestrator runtime APIs in Python bindings
2. Defining cross-runtime async semantics (timeouts, cancellation, backpressure)
3. Defining stream/event lifecycle behavior at Python call sites
4. Preserving compatibility contracts and migration windows for active consumers

## Benefits of Doing the Swap

1. Less behavioral drift between Python and Rust orchestration paths
2. Lower long-term maintenance in downstream Python orchestration code
3. Cleaner policy enforcement and observability from one runtime core
4. Simpler future feature rollout once boundaries are stable

## Costs and Risks

1. Async boundary complexity (`asyncio` and Rust runtime interactions)
2. Cancellation and timeout semantics can diverge if not specified tightly
3. REPL lifecycle edge cases can become harder to debug during migration
4. Packaging and CI parity work increases (binary distribution and runtime matrix)

No one gets bonus points for two event loops that disagree about who timed out first.

## Cross-Project Consequences

### For `rlm-claude-code`

This is the main impact area:

1. Migration shims and flags are needed during rollout
2. Existing invariants and tests must hold during dual-path windows
3. Release timing must respect tuple-based compatibility policy

### For `loop-agent`

Indirect impact:

1. Optional seam contracts should remain stable
2. Deterministic fallback and sensitivity guardrails must not regress

### For `io-rflx`

Mostly contract impact:

1. Keep schema versioning and migration notes strict
2. Preserve deterministic confidence mapping behavior

## Decision Framework

Use this as a practical go/no-go filter:

1. Do we have measurable drift or maintenance cost from split orchestration paths?
2. Can we specify async/cancellation semantics as a testable contract before implementation?
3. Can we run shadow mode and compare old vs new paths on representative scenarios?
4. Do compatibility gates cover all impacted consumer tuples?

If the answer is "no" on any of these, defer the full swap and keep harvesting value via component delegation.

## Recommended Path

1. Keep component delegation as default for active delivery work.
2. Do adapter-first binding expansion before generic orchestrator trait exposure.
3. Run shadow mode in `rlm-claude-code` for behavior and metadata diffing.
4. Promote only after compatibility gates are green for affected tuples.
5. Keep a kill switch until stability is demonstrated over a sustained window.

The goal is less glue code, not more glue code with a nicer README.

## Bottom Line

You can move `rlm-claude-code`, `loop-agent`, and `io-rflx` forward now and get high practical value from `loop` and `rlm-core` without a full Python orchestrator swap.

A full swap is a strategic unification move, not a prerequisite for current product progress.

## Source References

1. `docs/migration-spec-rlm-claude-code.md`
2. `docs/execution-plan/DECISIONS.md`
3. `docs/execution-plan/contracts/CONSUMER-INTEGRATION.md`
4. `docs/execution-plan/contracts/LOOP-AGENT-RUNTIME-SEAM.md`
5. `docs/execution-plan/contracts/IO-RFLX-INTEROP-CONTRACT.md`
6. `rlm-core/src/pybind/mod.rs`
7. `rlm-core/src/adapters/claude_code/adapter.rs`
8. `rlm-core/src/orchestrator.rs`
