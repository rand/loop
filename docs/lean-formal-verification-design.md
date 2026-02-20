# Lean Formal Verification System Design

> Formal specification and verification capabilities for rlm-core
>
> Historical design/planning artifact:
> - Unchecked `[ ]` checklist items in this file are archival, not active implementation backlog.
> - For live execution status and priorities, use `bd status` and:
>   - `docs/execution-plan/STATUS.md`
>   - `docs/execution-plan/TASK-REGISTRY.md`
>   - `docs/execution-plan/WORKBOARD.md`

## Executive Summary

This document proposes two interconnected capabilities:

1. **Lean REPL** - A Lean 4 REPL integrated with rlm-core for interactive theorem proving, spec validation, and implementation verification
2. **Spec Agent** - An AI agent specialized in creating, refining, and formalizing specifications, with first-class Topos integration

Together, these enable a **dual-track specification workflow**:
- **Topos** (semantic contracts) - Human-readable intent, traceability, evidence
- **Lean** (formal specs) - Machine-verifiable types, invariants, proofs

---

## 1. Vision: Dual-Track Formal Specifications

### 1.1 The Problem

Current specification approaches have a gap:
- **Natural language specs** are ambiguous, can't be verified
- **Formal specs** are precise but hard to write and maintain
- **AI-generated code** may not match intent without verification

### 1.2 The Solution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DUAL-TRACK SPECIFICATION                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Natural Language    Spec Agent      Topos Spec        Lean Formalization   │
│      Intent      ───────────────►    (.tps)     ◄────►     (.lean)          │
│                      (bidirectional)                                         │
│                                         │                    │               │
│                                         │                    │               │
│                                         ▼                    ▼               │
│                                   Implementation      Proof Checking         │
│                                     (Code)           (Type Checker)          │
│                                         │                    │               │
│                                         ▼                    ▼               │
│                                    Evidence ◄──────────► Verified            │
│                                  (git, tests)          Properties            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key insight**: LLMs are well-suited for proof generation because invalid proofs are rejected by the type checker. The challenge shifts to writing good specifications—which is exactly what the Spec Agent addresses.

### 1.3 Traceability Chain

```
[REQ-1] User requirement (Topos)
    ↓
[BEH-1] Behavior contract (Topos)
    ↓ implements
[SPEC-01.01] Lean type/theorem reference
    ↓ formalizes
theorem create_order_preserves_inventory : ∀ order, valid_order order → ...
    ↓ proves
[TASK-1] Implementation task (Topos)
    ↓ evidence
PR #123, commit abc123, 95% coverage
```

---

## 2. Lean REPL Architecture

### 2.1 Integration Model

The Lean REPL follows the same subprocess + JSON-RPC pattern as the Python REPL:

```
┌─────────────┐  JSON-RPC   ┌─────────────────────────┐
│  rlm-core   │◄───────────►│  Lean 4 REPL            │
│  (Rust)     │  stdin/out  │  (leanprover-community) │
└─────────────┘             └─────────────────────────┘
                                      │
                                      ▼
                            ┌─────────────────────────┐
                            │  Mathlib / Custom Libs  │
                            │  (lake dependencies)    │
                            └─────────────────────────┘
```

**Backend**: [leanprover-community/repl](https://github.com/leanprover-community/repl)
- JSON protocol over stdin/stdout
- Environment pickling (save/restore proof states)
- Both command mode and tactic mode

### 2.2 Rust Implementation

```rust
// Extension of existing ReplEnvironment trait
pub struct LeanRepl {
    process: Child,
    reader: BufReader<ChildStdout>,
    writer: BufWriter<ChildStdin>,
    env_counter: u64,  // Track Lean environments
    project_root: PathBuf,
}

impl ReplEnvironment for LeanRepl {
    fn execute(&mut self, code: &str) -> Result<ExecuteResult> {
        // Send JSON command to Lean REPL
        let request = LeanCommand::Command {
            cmd: code.to_string(),
            env: self.current_env,
        };
        self.send_json(&request)?;

        // Parse response
        let response: LeanResponse = self.read_json()?;

        Ok(ExecuteResult {
            success: response.sorries.is_empty() && response.messages.iter()
                .all(|m| m.severity != "error"),
            stdout: response.format_output(),
            stderr: response.format_errors(),
            metadata: Some(json!({
                "env": response.env,
                "sorries": response.sorries,
                "goals": response.goals,
            })),
        })
    }

    fn get_pending_operations(&self) -> Vec<String> {
        // Sorries and holes become pending operations
        self.sorries.iter()
            .map(|s| format!("sorry:{}", s.goal))
            .collect()
    }

    fn resolve_operation(&mut self, id: &str, result: Value) -> Result<()> {
        // Apply tactic or proof provided by AI
        if id.starts_with("sorry:") {
            let tactic = result.as_str().ok_or(Error::InvalidTactic)?;
            self.apply_tactic(tactic)?;
        }
        Ok(())
    }
}
```

### 2.3 Lean REPL Protocol

**Commands** (from leanprover-community/repl):

| Command Type | Fields | Purpose |
|--------------|--------|---------|
| `command` | `cmd`, `env?` | Execute Lean command |
| `tactic` | `tactic`, `proofState` | Apply tactic in proof mode |
| `pickle` | `path`, `env` | Save environment to file |
| `unpickle` | `path` | Restore environment |

**Response Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `env` | `number` | Environment ID (for backtracking) |
| `messages` | `array` | Compiler messages (info, warning, error) |
| `sorries` | `array` | Unfinished goals with positions |
| `goals` | `array?` | Proof goals in tactic mode |

### 2.4 Project Management

Lean requires project context (mathlib, dependencies). The REPL manager handles this:

```rust
pub struct LeanProjectManager {
    /// Active project configurations
    projects: HashMap<PathBuf, LeanProject>,
}

pub struct LeanProject {
    root: PathBuf,
    lakefile: LakefileConfig,
    lean_version: String,
    /// Built packages available for import
    packages: Vec<String>,
}

impl LeanProjectManager {
    /// Initialize or get existing project
    pub async fn get_or_create(&mut self, path: &Path) -> Result<&LeanProject> {
        if !self.projects.contains_key(path) {
            // Check for existing lakefile.lean/lakefile.toml
            let project = if path.join("lakefile.lean").exists() {
                LeanProject::from_existing(path)?
            } else {
                // Create new project with defaults
                LeanProject::create_new(path, LeanProjectTemplate::Mathlib)?
            };

            // Build dependencies
            project.lake_build().await?;

            self.projects.insert(path.to_path_buf(), project);
        }
        Ok(self.projects.get(path).unwrap())
    }
}
```

### 2.5 Proof State Management

```rust
/// Proof state that can be saved/restored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofState {
    /// Environment ID from Lean REPL
    env: u64,
    /// Current proof goals
    goals: Vec<Goal>,
    /// Proof steps taken
    history: Vec<ProofStep>,
    /// Checkpoint file path (for persistence)
    checkpoint: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Goal type signature
    target: String,
    /// Local hypotheses
    hyps: Vec<Hypothesis>,
    /// Suggested tactics (AI-generated)
    suggestions: Vec<TacticSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    tactic: String,
    /// Goals before
    pre_goals: Vec<String>,
    /// Goals after
    post_goals: Vec<String>,
    /// Time taken
    elapsed_ms: u64,
}
```

### 2.6 Trajectory Events for Lean

```rust
pub enum TrajectoryEventType {
    // ... existing events ...

    // Lean-specific events
    LeanExec,           // Lean command execution
    LeanResult,         // Lean execution result
    ProofStart,         // Begin proof attempt
    TacticApply,        // Tactic application
    GoalChange,         // Proof goals changed
    SorryFound,         // Unfinished proof detected
    ProofComplete,      // All goals discharged
    TypeCheck,          // Type checking result
    LemmaExtract,       // Lemma extracted from proof
}
```

---

## 3. Spec Agent Architecture

### 3.1 Agent Overview

The Spec Agent is a specialized agent for specification creation and refinement:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            SPEC AGENT                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │   Intake     │───►│   Refine     │───►│  Formalize   │                   │
│  │              │    │              │    │              │                   │
│  │ • NL parsing │    │ • Clarify    │    │ • Topos gen  │                   │
│  │ • Intent     │    │ • Decompose  │    │ • Lean gen   │                   │
│  │   extraction │    │ • Validate   │    │ • Sync       │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│          │                  │                   │                            │
│          └──────────────────┼───────────────────┘                            │
│                             ▼                                                │
│                    ┌──────────────┐                                          │
│                    │   Verify     │                                          │
│                    │              │                                          │
│                    │ • Type check │                                          │
│                    │ • Prove      │                                          │
│                    │ • Validate   │                                          │
│                    └──────────────┘                                          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Agent Capabilities

| Capability | Description | Tools |
|------------|-------------|-------|
| **Intake** | Parse natural language requirements into structured intent | NLP, LLM |
| **Refine** | Iterative clarification with human feedback | AskUserQuestion, Memory |
| **Formalize (Topos)** | Generate Topos specs (.tps files) | Topos MCP tools |
| **Formalize (Lean)** | Generate Lean types, theorems, proofs | Lean REPL |
| **Verify** | Type-check Lean specs, attempt proofs | Lean REPL |
| **Sync** | Keep Topos ↔ Lean in sync | Custom sync engine |

### 3.3 Agent Configuration

```rust
pub struct SpecAgentConfig {
    /// Default formalization level
    pub default_level: FormalizationLevel,
    /// Domains to prioritize (affects library imports)
    pub domains: Vec<Domain>,
    /// Proof strategy
    pub proof_strategy: ProofStrategy,
    /// Topos integration
    pub topos_enabled: bool,
    pub topos_project_root: Option<PathBuf>,
    /// Lean project settings
    pub lean_project_root: Option<PathBuf>,
    pub lean_libraries: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum FormalizationLevel {
    /// Types only - domain models, enums, structs
    Types,
    /// Types + invariants - type-level constraints
    TypesWithInvariants,
    /// Pre/post conditions - function contracts
    Contracts,
    /// Full proofs - complete correctness proofs
    FullProofs,
}

#[derive(Debug, Clone, Copy)]
pub enum ProofStrategy {
    /// decide, simp, omega, then AI, then human
    Progressive,
    /// Always try AI proof search first
    AIFirst,
    /// Draft outline, human completes
    HumanInLoop,
}

#[derive(Debug, Clone)]
pub enum Domain {
    AlgorithmsDataStructures,
    DistributedSystems,
    APIsProtocols,
    SecurityProperties,
    ApplicationFlow,
}
```

### 3.4 Workflow: Intake → Refine → Formalize

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE 1: INTAKE                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User: "I need an order management system that ensures inventory             │
│         is never oversold, supports partial fulfillment, and                 │
│         maintains audit trails for compliance."                              │
│                                                                              │
│                              ▼                                               │
│                                                                              │
│  Spec Agent extracts:                                                        │
│  • Domain: E-commerce / Inventory                                            │
│  • Key entities: Order, Inventory, AuditLog                                  │
│  • Invariants: inventory >= 0, no overselling                                │
│  • Behaviors: create_order, fulfill_order, partial_fulfill                   │
│  • Properties: audit_complete (all changes logged)                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE 2: REFINE                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Spec Agent asks clarifying questions:                                       │
│                                                                              │
│  Q1: "How should partial fulfillment work?"                                  │
│      • Split into multiple shipments                                         │
│      • Backorder remaining items                                             │
│      • Cancel unfulfillable items                                            │
│                                                                              │
│  Q2: "What audit events need to be captured?"                                │
│      • All state changes                                                     │
│      • Only fulfillment events                                               │
│      • Configurable per order type                                           │
│                                                                              │
│  Q3: "Are there concurrency requirements?"                                   │
│      • Single-writer (queue-based)                                           │
│      • Optimistic locking                                                    │
│      • Serializable transactions                                             │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE 3: FORMALIZE                                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Generates Topos spec (human-readable):                                      │
│                                                                              │
│  ```topos                                                                    │
│  spec OrderManagement                                                        │
│                                                                              │
│  ## Concepts                                                                 │
│                                                                              │
│  Order:                                                                      │
│    id: `OrderId`                                                             │
│    items: list of `OrderItem`                                                │
│    status: `pending` | `partial` | `fulfilled` | `cancelled`                 │
│    invariant: items.all(i => i.quantity > 0)                                 │
│    @lean: Order.lean#Order                                                   │
│                                                                              │
│  ## Behaviors                                                                │
│                                                                              │
│  create_order:                                                               │
│    implements: REQ-1                                                         │
│    given: valid order request                                                │
│    returns: `Order` with status `pending`                                    │
│    ensures: inventory reserved for all items                                 │
│    @lean: Order.lean#create_order_spec                                       │
│  ```                                                                         │
│                                                                              │
│  AND generates Lean formalization (machine-verifiable):                      │
│                                                                              │
│  ```lean                                                                     │
│  -- Order.lean                                                               │
│  import Mathlib.Data.List.Basic                                              │
│                                                                              │
│  structure OrderItem where                                                   │
│    productId : Nat                                                           │
│    quantity : Nat                                                            │
│    quantity_pos : quantity > 0                                               │
│                                                                              │
│  structure Order where                                                       │
│    id : Nat                                                                  │
│    items : List OrderItem                                                    │
│    status : OrderStatus                                                      │
│                                                                              │
│  -- Theorem: creating an order reserves inventory                            │
│  theorem create_order_reserves_inventory                                     │
│    (inv : Inventory) (req : OrderRequest)                                    │
│    (h_valid : valid_request inv req) :                                       │
│    let (order, inv') := create_order inv req                                 │
│    ∀ item ∈ order.items,                                                     │
│      inv'.reserved item.productId ≥ item.quantity := by                      │
│    sorry  -- [?] AI will attempt proof                                       │
│  ```                                                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Topos ↔ Lean Dual-Track Sync

### 4.1 Linking Mechanism

Topos specs reference Lean artifacts via `@lean` annotations:

```topos
## Concepts

Order:
  id: `OrderId`
  items: list of `OrderItem`
  status: `pending` | `partial` | `fulfilled` | `cancelled`
  invariant: items.all(i => i.quantity > 0)
  @lean: specs/Order.lean#Order           # Links to Lean structure
  @lean.invariant: specs/Order.lean#Order.items_nonempty  # Links to theorem
```

Lean files include reverse links via comments:

```lean
/--
@topos: OrderManagement.tps#Order
Order represents a customer order with line items.
-/
structure Order where
  id : Nat
  items : List OrderItem
  status : OrderStatus
  items_pos : ∀ item ∈ items, item.quantity > 0  -- @topos: Order.invariant
```

### 4.2 Sync Engine

```rust
pub struct DualTrackSync {
    topos_project: PathBuf,
    lean_project: PathBuf,
    link_index: LinkIndex,
}

#[derive(Debug)]
pub struct LinkIndex {
    /// Topos artifact → Lean artifact
    topos_to_lean: HashMap<ToposRef, LeanRef>,
    /// Lean artifact → Topos artifact
    lean_to_topos: HashMap<LeanRef, ToposRef>,
}

impl DualTrackSync {
    /// Check for drift between Topos and Lean
    pub async fn check_drift(&self) -> Result<DriftReport> {
        let mut report = DriftReport::new();

        for (topos_ref, lean_ref) in &self.link_index.topos_to_lean {
            // Parse both artifacts
            let topos_artifact = self.parse_topos(topos_ref)?;
            let lean_artifact = self.parse_lean(lean_ref)?;

            // Compare semantically
            if let Some(drift) = self.compare_artifacts(&topos_artifact, &lean_artifact)? {
                report.add_drift(topos_ref.clone(), lean_ref.clone(), drift);
            }
        }

        // Check for unlinked artifacts
        report.unlinked_topos = self.find_unlinked_topos()?;
        report.unlinked_lean = self.find_unlinked_lean()?;

        Ok(report)
    }

    /// Propagate changes from Topos to Lean
    pub async fn sync_topos_to_lean(&self, changes: &[ToposChange]) -> Result<Vec<LeanChange>> {
        let mut lean_changes = Vec::new();

        for change in changes {
            match change {
                ToposChange::ConceptAdded(concept) => {
                    // Generate Lean structure
                    let lean_struct = self.generate_lean_structure(concept)?;
                    lean_changes.push(LeanChange::StructureAdded(lean_struct));
                }
                ToposChange::InvariantAdded(inv) => {
                    // Generate Lean theorem
                    let lean_theorem = self.generate_lean_theorem(inv)?;
                    lean_changes.push(LeanChange::TheoremAdded(lean_theorem));
                }
                ToposChange::BehaviorAdded(behavior) => {
                    // Generate Lean function spec
                    let lean_spec = self.generate_lean_function_spec(behavior)?;
                    lean_changes.push(LeanChange::FunctionSpecAdded(lean_spec));
                }
                // ... other change types
            }
        }

        Ok(lean_changes)
    }
}
```

### 4.3 Formalization Levels

The sync engine supports progressive formalization:

| Level | Topos | Lean |
|-------|-------|------|
| **Types** | Concepts with fields | `structure` definitions |
| **Types + Invariants** | Concepts with invariants | Structures with proofs in fields |
| **Contracts** | Behaviors with pre/post | Function specs with `requires`/`ensures` |
| **Full Proofs** | All of above | Complete theorems with proofs |

```rust
impl DualTrackSync {
    pub fn set_formalization_level(&mut self, level: FormalizationLevel) {
        self.level = level;
    }

    fn generate_lean_for_concept(&self, concept: &Concept) -> Result<String> {
        match self.level {
            FormalizationLevel::Types => {
                // Just structure definition
                self.generate_basic_structure(concept)
            }
            FormalizationLevel::TypesWithInvariants => {
                // Structure with proof fields
                self.generate_structure_with_proofs(concept)
            }
            FormalizationLevel::Contracts | FormalizationLevel::FullProofs => {
                // Full specification
                self.generate_full_spec(concept)
            }
        }
    }
}
```

---

## 5. Proof Automation Strategy

### 5.1 Progressive Automation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PROOF AUTOMATION PIPELINE                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  TIER 1: Decidable (instant)                                                 │
│  ─────────────────────────                                                   │
│  • decide - boolean decidability                                             │
│  • native_decide - native computation                                        │
│  • omega - linear arithmetic                                                 │
│  • simp - simplification with lemmas                                         │
│                                                                              │
│            │ if fails                                                        │
│            ▼                                                                 │
│                                                                              │
│  TIER 2: Automation (seconds)                                                │
│  ────────────────────────────                                                │
│  • aesop - extensible automation                                             │
│  • linarith - linear arithmetic reasoning                                    │
│  • ring - ring algebra                                                       │
│  • polyrith - polynomial arithmetic (with LLM hints)                         │
│                                                                              │
│            │ if fails                                                        │
│            ▼                                                                 │
│                                                                              │
│  TIER 3: AI-Assisted (seconds-minutes)                                       │
│  ─────────────────────────────────────                                       │
│  • LLM generates tactic sequence                                             │
│  • Validates against type checker                                            │
│  • Retries with different strategies                                         │
│  • Uses LeanDojo / DeepSeek-Prover patterns                                  │
│                                                                              │
│            │ if fails                                                        │
│            ▼                                                                 │
│                                                                              │
│  TIER 4: Human-in-Loop                                                       │
│  ─────────────────────                                                       │
│  • Mark with sorry + TODO                                                    │
│  • Create task in beads                                                      │
│  • Provide proof outline for human                                           │
│  • Store in memory for future reference                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 AI Proof Generation

```rust
pub struct AIProofAssistant {
    llm_client: Arc<dyn LLMClient>,
    lean_repl: Arc<Mutex<LeanRepl>>,
    strategy_memory: Arc<MemoryStore>,
}

impl AIProofAssistant {
    /// Attempt to prove a theorem
    pub async fn prove(&self, theorem: &str, context: &ProofContext) -> Result<ProofResult> {
        // Step 1: Try decidable tactics
        if let Some(proof) = self.try_decidable(theorem).await? {
            return Ok(ProofResult::Proved(proof));
        }

        // Step 2: Try automation tactics
        if let Some(proof) = self.try_automation(theorem, context).await? {
            return Ok(ProofResult::Proved(proof));
        }

        // Step 3: AI-assisted proof search
        for attempt in 0..self.config.max_ai_attempts {
            let tactics = self.generate_tactics(theorem, context, attempt).await?;

            match self.validate_tactics(&tactics).await? {
                ValidationResult::Valid(proof) => {
                    // Store successful strategy
                    self.strategy_memory.store_strategy(theorem, &proof).await?;
                    return Ok(ProofResult::Proved(proof));
                }
                ValidationResult::Partial { progress, remaining_goals } => {
                    // Update context with progress
                    context.add_progress(progress);
                    context.set_goals(remaining_goals);
                }
                ValidationResult::Invalid(errors) => {
                    // Learn from failure
                    context.add_failed_attempt(&tactics, &errors);
                }
            }
        }

        // Step 4: Give up, create TODO
        Ok(ProofResult::Sorry {
            outline: self.generate_proof_outline(theorem, context).await?,
            task_id: self.create_proof_task(theorem, context).await?,
        })
    }

    async fn generate_tactics(&self, theorem: &str, context: &ProofContext, attempt: u32) -> Result<Vec<String>> {
        let prompt = format!(
            r#"You are a Lean 4 theorem prover. Generate tactics to prove:

```lean
{theorem}
```

Context:
- Available lemmas: {lemmas}
- Similar proved theorems: {similar}
- Previous failed attempts: {failed}

Attempt {attempt}. Generate a sequence of tactics, one per line.
If you need a helper lemma, use `have` to introduce it.
"#,
            theorem = theorem,
            lemmas = context.available_lemmas.join(", "),
            similar = context.similar_proofs.join("\n"),
            failed = context.failed_attempts.join("\n"),
            attempt = attempt,
        );

        let response = self.llm_client.complete(&prompt).await?;
        Ok(self.parse_tactics(&response))
    }
}
```

### 5.3 Domain-Specific Strategies

```rust
/// Domain-specific proof strategies
pub fn get_domain_strategies(domain: Domain) -> Vec<ProofStrategy> {
    match domain {
        Domain::AlgorithmsDataStructures => vec![
            ProofStrategy::Induction,          // Structural induction
            ProofStrategy::WellFoundedRecursion,
            ProofStrategy::CaseAnalysis,
        ],
        Domain::DistributedSystems => vec![
            ProofStrategy::Bisimulation,       // State machine equivalence
            ProofStrategy::Invariant,          // Inductive invariants
            ProofStrategy::Refinement,         // Refinement mappings
        ],
        Domain::APIsProtocols => vec![
            ProofStrategy::StateTransition,    // Pre/post conditions
            ProofStrategy::SessionTypes,       // Protocol compliance
            ProofStrategy::ResourceAccounting, // Linear types
        ],
        Domain::SecurityProperties => vec![
            ProofStrategy::InformationFlow,    // Non-interference
            ProofStrategy::AccessControl,      // Authorization
            ProofStrategy::Cryptographic,      // Game-based proofs
        ],
        Domain::ApplicationFlow => vec![
            ProofStrategy::Termination,        // Liveness
            ProofStrategy::Safety,             // Invariant preservation
            ProofStrategy::Fairness,           // Progress properties
        ],
    }
}
```

---

## 6. Integration with Disciplined Process

### 6.1 Spec-Linked Tasks

Every Lean theorem/lemma traces to a `[SPEC-XX.YY]` identifier:

```topos
## Requirements

REQ-1: Order Creation
  when: valid order request received
  the system shall: create order with reserved inventory
  @spec: SPEC-01.01
  @lean: specs/Order.lean#create_order_reserves_inventory
```

```lean
/--
@spec: SPEC-01.01
@topos: OrderManagement.tps#REQ-1
-/
theorem create_order_reserves_inventory
  (inv : Inventory) (req : OrderRequest)
  (h_valid : valid_request inv req) :
  let (order, inv') := create_order inv req
  ∀ item ∈ order.items,
    inv'.reserved item.productId ≥ item.quantity := by
  ...
```

### 6.2 DP Integration Points

| DP Phase | Spec Agent Role |
|----------|-----------------|
| **Orient** | Query spec coverage, find unformalized requirements |
| **Specify** | Generate/refine Topos specs, create Lean formalizations |
| **Decide** | ADRs reference formal properties proved |
| **Test** | Tests annotated with `@trace SPEC-XX.YY`, Lean proofs validate |
| **Implement** | Implementation guided by proved properties |
| **Review** | Review checks: are all specs formalized? proofs complete? |
| **Close** | Evidence includes proof status |

### 6.3 CLI Integration

```bash
# Check spec coverage
/dp:spec coverage --with-lean

# List unproved theorems
/dp:spec list --unproved

# Generate Lean formalization for a requirement
/dp:spec formalize REQ-1 --level full-proofs

# Verify Lean specs type-check
/dp:spec verify --lean

# Sync Topos ↔ Lean
/dp:spec sync --check-drift
```

---

## 7. Memory Integration

### 7.1 Storing Proofs and Strategies

```rust
// New node types for formal verification
pub enum NodeType {
    // ... existing types ...

    /// Lean theorem statement
    Theorem,
    /// Completed proof
    Proof,
    /// Proof strategy that worked
    ProofStrategy,
    /// Lemma extracted during proving
    Lemma,
    /// Failed proof attempt (for learning)
    FailedProof,
}

// Store successful proof strategies
pub struct ProofStrategyNode {
    /// Theorem pattern (generalized)
    pattern: String,
    /// Tactic sequence that worked
    tactics: Vec<String>,
    /// Domain
    domain: Domain,
    /// Success rate
    success_rate: f64,
    /// Contexts where it worked
    successful_contexts: Vec<String>,
}
```

### 7.2 Learning from Proofs

The memory system enables learning from proof attempts:

1. **Store successful strategies** → Reuse for similar theorems
2. **Store failed attempts** → Avoid repeating mistakes
3. **Extract lemmas** → Build domain-specific lemma library
4. **Track patterns** → Identify common proof structures

---

## 8. Implementation Phases

### Phase 1: Lean REPL Foundation (2-3 weeks)

| Task | Description |
|------|-------------|
| LeanRepl implementation | Subprocess management, JSON-RPC |
| Project management | Lake integration, dependency handling |
| Basic commands | Execute, type-check, tactic mode |
| Trajectory events | Lean-specific event types |
| Tests | REPL lifecycle, error handling |

**Deliverables**:
- `LeanRepl` implementing `ReplEnvironment`
- `LeanProjectManager` for project/dependency handling
- Integration with rlm-core orchestrator

### Phase 2: Topos Integration (1-2 weeks)

| Task | Description |
|------|-------------|
| Topos MCP client | Connect to topos MCP server |
| Link annotations | Parse `@lean` annotations in Topos |
| Reverse annotations | Parse `@topos` comments in Lean |
| Link index | Build and maintain bidirectional index |
| Tests | Parsing, linking, index updates |

**Deliverables**:
- Topos MCP integration in rlm-core
- Link index with persistence

### Phase 3: Dual-Track Sync (2 weeks)

| Task | Description |
|------|-------------|
| Drift detection | Compare Topos ↔ Lean semantically |
| Topos → Lean generation | Generate Lean from Topos changes |
| Lean → Topos generation | Update Topos from Lean changes |
| Sync CLI commands | `sync`, `drift`, `formalize` |
| Tests | Sync scenarios, conflict resolution |

**Deliverables**:
- `DualTrackSync` engine
- CLI commands for sync operations

### Phase 4: Spec Agent (2-3 weeks)

| Task | Description |
|------|-------------|
| Agent scaffolding | Spec agent configuration, lifecycle |
| Intake phase | NL parsing, intent extraction |
| Refine phase | Question generation, clarification |
| Formalize phase | Topos + Lean generation |
| Tests | End-to-end spec creation |

**Deliverables**:
- Spec agent implementation
- Integration with RLM orchestrator

### Phase 5: Proof Automation (2-3 weeks)

| Task | Description |
|------|-------------|
| Decidable tier | Basic tactic automation |
| Automation tier | aesop, linarith, ring integration |
| AI-assisted tier | LLM proof generation |
| Strategy memory | Store/retrieve successful strategies |
| Tests | Proof benchmarks per domain |

**Deliverables**:
- `AIProofAssistant` with progressive automation
- Strategy memory integration

### Phase 6: DP Integration (1-2 weeks)

| Task | Description |
|------|-------------|
| Spec coverage | Track formalization coverage |
| Proof status | Track proof completion |
| CLI commands | DP spec commands with Lean |
| Evidence gathering | Proof evidence in tasks |
| Tests | DP workflow integration |

**Deliverables**:
- Full DP integration
- Documentation

---

## 9. Success Criteria

### 9.1 Functional Requirements

- [ ] Lean REPL executes commands and tactics correctly
- [ ] Lean projects with mathlib dependencies work
- [ ] Topos ↔ Lean bidirectional linking works
- [ ] Drift detection catches spec divergence
- [ ] Spec agent generates valid Topos and Lean specs
- [ ] Progressive proof automation achieves >70% auto-proof rate on simple theorems
- [ ] DP integration tracks spec coverage and proof status

### 9.2 Performance Requirements

| Metric | Target |
|--------|--------|
| Lean REPL startup | < 5s (with cached build) |
| Simple tactic execution | < 500ms |
| Type checking | < 2s for typical module |
| AI proof attempt | < 30s per attempt |
| Drift detection | < 10s for 100 linked artifacts |

### 9.3 Quality Requirements

- [ ] >80% test coverage on core components
- [ ] All public APIs documented
- [ ] Proof strategies stored for reuse
- [ ] Graceful degradation when Lean unavailable

---

## 10. Open Questions

### 10.1 Architecture

1. **Lean version management**: How to handle projects requiring different Lean versions?
   - Proposal: Per-project Lean version via elan, similar to rustup

2. **Mathlib dependency**: Full mathlib is large (~2GB). Cache strategy?
   - Proposal: Lazy download, shared cache across projects

3. **Proof persistence**: Store proofs in memory or external files?
   - Proposal: Lean files are source of truth, memory stores metadata

### 10.2 Workflow

1. **Human review of AI proofs**: How to surface AI-generated proofs for review?
   - Proposal: Mark AI proofs with attribute, review during DP review phase

2. **Proof maintenance**: How to handle proofs that break when specs change?
   - Proposal: Drift detection flags broken proofs, prioritize in tasks

3. **Partial formalization**: How to handle specs that can't be fully formalized?
   - Proposal: `sorry` with confidence level, track as technical debt

---

## 11. References

### Lean Ecosystem
- [leanprover-community/repl](https://github.com/leanprover-community/repl) - Official Lean REPL
- [LeanInteract](https://github.com/augustepoiroux/LeanInteract) - Python interface
- [lean-spec](https://github.com/paulch42/lean-spec) - Program specification
- [LeanDojo](https://leandojo.org/) - AI-driven theorem proving
- [Mathlib](https://github.com/leanprover-community/mathlib4) - Mathematics library

### Formal Verification
- [AWS Cedar verification](https://lean-lang.org/) - Industrial Lean usage
- [Martin Kleppmann on AI + FV](https://martin.kleppmann.com/2025/12/08/ai-formal-verification.html)
- [DeepSeek-Prover-V2](https://arxiv.org/abs/2504.07612) - AI theorem proving

### Topos
- [Topos GitHub](https://github.com/rand/topos) - Semantic contract language
- Architecture, language spec, and MCP integration docs

### rlm-core
- [Unified RLM Library Design](./unified-rlm-library-design.md) - Core architecture
- [ADR-001](./adr/ADR-001-unified-rlm-library.md) - Architectural decisions
