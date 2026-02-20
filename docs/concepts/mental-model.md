# Loop Mental Model

Loop is a runtime for turning "this problem is too big" into "this is a sequence of tractable decisions with receipts".

## The Core Loop

1. **Observe**
- Collect user intent, context state, and relevant file/tool signals.

2. **Orient**
- Classify complexity and decide whether recursive orchestration is warranted.
- Externalize context into a structure that modules can reason over.

3. **Decide**
- Choose model, budget posture, decomposition strategy, and validation depth.

4. **Act**
- Execute module flows, produce outputs, persist traces/evidence, and report outcomes.

The cycle repeats until quality and completion criteria are met.

## Design Tensions

### Speed vs Certainty
- Fast path gives quick iteration.
- Deep path increases cost but catches mistakes earlier.
- Loop makes this tradeoff explicit via mode selection and policy gates.

### Flexibility vs Reproducibility
- Dynamic runtime behaviors are useful.
- Production workflows need deterministic checks.
- Loop uses typed signatures, structured events, and governance gates to keep both.

### Automation vs Human Control
- Automation scales throughput.
- Humans still own risk decisions.
- Loop supports policy-based enforcement and evidence-first handoff.

## What "Good" Looks Like

A healthy Loop workflow has:
- Clear intent capture.
- Explicit strategy selection.
- Traceable execution decisions.
- Evidence for claims.
- Reproducible quality checks.

Or, put differently: fewer mysteries and fewer surprises during push.
