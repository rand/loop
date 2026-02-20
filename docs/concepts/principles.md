# System Principles

These principles explain how Loop is intended to behave when design tradeoffs appear.

If two choices both look reasonable, this file is the tie-breaker.

## 1. Evidence Beats Confidence

Claims about behavior should be backed by:

1. Passing checks and tests.
2. Reproducible command output.
3. Artifact paths for nontrivial validations.

Why:
- Confidence is cheap and usually wrong at scale.
- Evidence survives context switching.

## 2. Determinism Over Heroics

Operational workflows should prefer deterministic outcomes over fragile brilliance.

Examples:
- Policy gates with machine-readable JSON output.
- Stable command sequences in user and developer docs.
- Explicit failure protocols in troubleshooting docs.

## 3. Contracts Before Convenience

External surfaces are contract-sensitive.
Internal implementations can evolve, but behavioral contracts need deliberate change control.

Examples:
- Compatibility helper surfaces for Python integration.
- Feature contracts under `docs/spec/`.
- Adapter efficacy gates that guard real scenarios.

## 4. Bounded Autonomy

Loop automates aggressively, but not infinitely.
Activation, mode escalation, and memory use are bounded and observable.

Why:
- Unbounded autonomy is just expensive drift.
- Bounded systems are easier to debug and trust.

## 5. Practical Formalism

Formal methods are used where they add safety and leverage, not as decoration.

Examples:
- Tiered formalization depth (`Types`, `Invariants`, `Contracts`, `FullProofs`).
- Proof and sync paths integrated with governance gates.

## 6. Documentation Is Runtime Surface

Docs are part of the product interface.
If behavior changes and docs do not, users still experience a bug.

Required habits:

1. Update docs in the same change set as behavior.
2. Keep navigation indexes current.
3. Keep procedures command-driven and falsifiable.

## 7. Friction Where It Matters

Loop adds process friction at high-risk boundaries:

1. Pre-commit and pre-push policy enforcement.
2. Coverage and adapter efficacy gates.
3. Evidence logging for significant changes.

That friction is intentional. It costs minutes now to save days later.

## Decision Questions

When unsure, ask:

1. Does this change improve reproducibility?
2. Does it reduce hidden risk for users or maintainers?
3. Can another engineer validate the same outcome independently?
4. Is the behavior clear in both code and docs?

If the answer is mostly "no," it is not ready yet.
