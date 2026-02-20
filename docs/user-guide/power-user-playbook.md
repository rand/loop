# Power User Playbook

For operators who want stronger control over correctness, cost, and throughput.

## Operating Modes and Depth

Use deeper formalization only where it pays off:
- `Types`: schema and shape confidence
- `Invariants`: data constraints
- `Contracts`: behavior guarantees
- `FullProofs`: highest assurance, highest effort

Principle: spend proof effort where failure costs are high.

## Evidence-First Workflow

Treat logs and artifacts as first-class outputs.

Pattern:
1. Run command.
2. Persist output into evidence path.
3. Summarize decision and result.
4. Link evidence in handoff or PR notes.

This removes "trust me" from the process.

## Performance Hygiene

1. Avoid running heavy suites in parallel on constrained systems.
2. Use targeted tests while iterating.
3. Run full gates before final push.
4. Track regression risk in notes when changing memory-heavy paths.

## Memory and Drift Analysis at Scale

When handling large symbol graphs:
- Prefer interned symbol lookups over repeated string matching.
- Use arena-style allocation for short-lived analysis structures.
- Validate that behavior remains deterministic under input ordering variance.

## Practical Guardrails

1. No skipped gates for convenience.
2. No undocumented bypasses.
3. No mystery config changes.
4. No "I’ll fix it in follow-up" without a tracked issue.

## Communication Pattern for High-Risk Changes

Use this format:
1. What changed.
2. Why this was necessary.
3. What could regress.
4. Which gates/evidence prove safety.

Short, explicit, reproducible. Heroics not required.
