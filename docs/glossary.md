# Loop Glossary

## A

**Activation Decision**
The classifier outcome that decides whether Loop should stay simple or invoke multi-step recursive orchestration.

## B

**Behavior (Topos)**
A semantic contract describing an operation, including preconditions, postconditions, and error cases.

**Beads (`bd`)**
The issue tracking system used by this repository for planning, status, and closure.

## C

**Completeness Mode**
Spec-agent generation policy:
- `Baseline`: placeholder-free stubs
- `Placeholder`: explicit `draft:` annotations with executable stubs (no `TODO`/`sorry` tokens)

**Context Externalization**
The process of transforming context into structured variables used by runtime orchestration and prompts.

## D

**Dual-Track Sync**
The synchronization layer between Topos semantic specs and Lean formal artifacts.

## E

**Epistemic Verification**
Claim-level confidence and consistency checks to reduce hallucination risk.

## F

**Formalization Level**
Depth of generated formal artifacts:
- Types
- Invariants
- Contracts
- FullProofs

## G

**Governance Gates (`dp`)**
Repository policy checks (`review`, `verify`, `enforce pre-commit`, `enforce pre-push`) executed via `./scripts/dp`.

## L

**Lean REPL**
Interactive Lean execution surface used for checking generated or authored proof artifacts.

**Loop Mode**
Execution path where recursive orchestration is activated for complex tasks.

## M

**Memory Store**
SQLite-backed hypergraph used for fact, experience, and reasoning trace persistence.

## O

**OODA**
Observe/Orient/Decide/Act loop used to reason about runtime behavior and validation flows.

## P

**Pattern Classifier**
The complexity detector used to determine whether orchestration should activate.

## R

**RLM (Recursive Language Model)**
The orchestration approach that decomposes large tasks into tractable sub-queries with shared context and synthesis.

**Reasoning Trace**
Decision graph capturing options, chosen paths, actions, and outcomes.

## S

**Spec Agent**
Workflow engine that transforms natural language requirements into Topos + Lean artifacts.

**SUBMIT**
Typed-signature output completion mechanism used to finalize structured module outputs.

## T

**Topos**
Human-readable semantic specification format (`.tps`) used for concepts, behaviors, and requirements.

**Trajectory Events**
Structured event stream describing runtime execution progress and decisions.
