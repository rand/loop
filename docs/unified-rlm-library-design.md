# Unified RLM Library Design

> A shared library supporting both Claude Code plugins and agentic TUIs
>
> Historical design artifact:
> - Checklist lines marked `[historical target]` are archival planning snapshots, not active backlog.
> - Authoritative live status is tracked in Beads (`bd status`) and:
>   - `docs/execution-plan/STATUS.md`
>   - `docs/execution-plan/TASK-REGISTRY.md`
>   - `docs/execution-plan/WORKBOARD.md`

## Executive Summary

This document proposes **rlm-core**, a Rust library with Python bindings that provides the foundational RLM (Recursive Language Model) capabilities shared between:

1. **Claude Code plugins** (like rlm-claude-code) - MCP tool exposure, hooks integration
2. **Agentic TUIs** (like recurse) - Bubble Tea integration, native terminal rendering

The library unifies the best capabilities from both systems while providing clean adapter interfaces for each deployment target.

---

## 1. Analysis: Common Patterns

After thorough analysis of both `recurse` and `rlm-claude-code`, the following shared patterns emerge:

### 1.1 Core RLM Loop

Both systems implement the same fundamental orchestration pattern:

```
┌─────────────────────────────────────────────────────────┐
│                    RLM Core Loop                         │
├─────────────────────────────────────────────────────────┤
│  1. EXTERNALIZE: Store context as manipulable variables  │
│  2. ANALYZE: Assess complexity, select strategy          │
│  3. DECOMPOSE: Partition context if needed               │
│  4. EXECUTE: Run code in REPL, make sub-calls            │
│  5. SYNTHESIZE: Combine results into final answer        │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Context Variables

Both systems externalize context using the same schema:

| Variable | Type | Description |
|----------|------|-------------|
| `conversation` | `List[Message]` | Full conversation history |
| `files` | `Dict[str, str]` | Cached file contents |
| `tool_outputs` | `List[ToolOutput]` | Recent tool execution results |
| `working_memory` | `Dict[str, Any]` | Session state |

### 1.3 REPL Environment

Both use Python REPLs with similar capabilities:

| Feature | Recurse | RLM-Claude-Code |
|---------|---------|-----------------|
| Sandbox | Subprocess with limits | RestrictedPython |
| Core helpers | `peek`, `search`, `llm` | `peek`, `search`, `summarize`, `llm` |
| Async handling | Native Go concurrency | DeferredOperation pattern |
| Tooling | uv, ruff, ty, pydantic | pydantic, hypothesis, cpmpy |

### 1.4 Recursive Sub-calls

Both implement depth-managed recursive LLM calls:

| Aspect | Recurse | RLM-Claude-Code |
|--------|---------|-----------------|
| Max depth | Configurable (default 5) | Configurable (default 2-3) |
| Model tiering | OpenRouter routing | Opus → Sonnet → Haiku by depth |
| Spawn REPL | Optional per sub-call | Optional per sub-call |
| Cost tracking | Per-call budget | Per-component cost |

### 1.5 Memory/State

| Feature | Recurse | RLM-Claude-Code |
|---------|---------|-----------------|
| Architecture | 3-tier hypergraph | Strategy cache + memory backend |
| Persistence | SQLite with embeddings | JSON/SQLite |
| Evolution | Consolidate/promote/decay | Cross-session promotion |
| Reasoning traces | Deciduous-style with git | Trajectory events |

### 1.6 Trajectory/Observability

Both stream execution events for visibility:

| Event Type | Recurse | RLM-Claude-Code |
|------------|---------|-----------------|
| Start | RLM trace view | `RLM_START` |
| Analysis | Decomposition events | `ANALYZE` |
| REPL | REPL view panel | `REPL_EXEC`, `REPL_RESULT` |
| Recursion | Nested traces | `RECURSE_START`, `RECURSE_END` |
| Completion | Outcome nodes | `FINAL`, `COST_REPORT` |

---

## 2. Unified Architecture

### 2.1 High-Level Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Application Layer                            │
│  ┌─────────────────────┐              ┌─────────────────────────┐    │
│  │   Claude Code       │              │    Agentic TUI          │    │
│  │   Plugin Adapter    │              │    (Bubble Tea)         │    │
│  │   • MCP tools       │              │    • Terminal renderer  │    │
│  │   • Hooks           │              │    • Keyboard handlers  │    │
│  │   • Skill system    │              │    • Panel system       │    │
│  └──────────┬──────────┘              └───────────┬─────────────┘    │
│             │                                     │                   │
│             └─────────────┬───────────────────────┘                   │
│                           │                                           │
│                           ▼                                           │
├─────────────────────────────────────────────────────────────────────┤
│                        rlm-core Library                              │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Orchestration Engine                      │    │
│  │  • Complexity classifier                                     │    │
│  │  • Strategy selector (direct/RLM/hybrid)                     │    │
│  │  • Recursive call manager                                    │    │
│  │  • Result synthesizer                                        │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐   │
│  │    REPL      │  │   Memory     │  │      Trajectory          │   │
│  │  Environment │  │   System     │  │      Streaming           │   │
│  │  • Sandbox   │  │  • Hypergraph│  │  • Event emission        │   │
│  │  • Helpers   │  │  • Tiers     │  │  • Export/replay         │   │
│  │  • Deferred  │  │  • Evolution │  │  • Cost tracking         │   │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                     LLM Abstraction                          │    │
│  │  • Multi-provider client (Anthropic, OpenAI, OpenRouter)     │    │
│  │  • Smart routing (by query type, depth, cost)                │    │
│  │  • Prompt caching support                                    │    │
│  │  • Rate limiting, retries                                    │    │
│  └─────────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────────┤
│                        Storage Layer                                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    SQLite + Extensions                       │    │
│  │  • Hypergraph schema (nodes, edges, membership)              │    │
│  │  • Trajectory storage                                        │    │
│  │  • Evolution audit log                                       │    │
│  │  • Session checkpoints                                       │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Core Components

#### 2.2.1 Orchestration Engine

The central brain that decides execution strategy and manages the RLM loop.

```rust
pub trait Orchestrator: Send + Sync {
    /// Determine if RLM should activate for this query
    fn should_activate(&self, query: &str, context: &SessionContext) -> ActivationDecision;

    /// Run the main orchestration loop
    async fn run(
        &self,
        query: &str,
        context: &SessionContext,
    ) -> impl Stream<Item = TrajectoryEvent>;

    /// Execute a recursive sub-call
    async fn recursive_call(
        &self,
        query: &str,
        context: &str,
        depth: u32,
        spawn_repl: bool,
    ) -> Result<RecursiveResult>;
}
```

#### 2.2.2 REPL Environment

Sandboxed Python execution with RLM helpers.

```rust
pub trait REPLEnvironment: Send + Sync {
    /// Execute code in the sandbox
    fn execute(&mut self, code: &str) -> ExecutionResult;

    /// Get/set variables
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn set_variable(&mut self, name: &str, value: Value);

    /// Handle deferred async operations
    fn get_pending_operations(&self) -> Vec<DeferredOperation>;
    fn resolve_operation(&mut self, id: &str, result: Value);

    /// Built-in helpers available in REPL
    // peek(var, start, end) -> slice of content
    // search(var, pattern, regex=False) -> matches
    // summarize(var, max_tokens=500) -> LLM summary (deferred)
    // llm(prompt, context) -> LLM call (deferred)
    // llm_batch(prompts, contexts) -> parallel LLM calls (deferred)
    // map_reduce(ctx, map_prompt, reduce_prompt) -> map-reduce pattern (deferred)
    // find_relevant(ctx, query, top_k) -> relevance-ranked chunks
}
```

#### 2.2.3 Memory System

Persistent hypergraph memory with tiered evolution.

```rust
pub trait MemoryStore: Send + Sync {
    /// Node operations
    async fn add_node(&self, node: Node) -> Result<NodeId>;
    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>>;
    async fn query_nodes(&self, query: &str, limit: usize) -> Result<Vec<Node>>;

    /// Edge operations
    async fn add_edge(&self, edge: HyperEdge) -> Result<EdgeId>;
    async fn get_edges(&self, node_id: &NodeId) -> Result<Vec<HyperEdge>>;

    /// Tier management
    async fn consolidate(&self, from: Tier, to: Tier) -> Result<ConsolidationResult>;
    async fn promote(&self, node_ids: &[NodeId], reason: &str) -> Result<()>;
    async fn decay(&self, factor: f64, min_confidence: f64) -> Result<Vec<NodeId>>;

    /// Reasoning traces
    async fn log_decision(&self, trace: ReasoningTrace) -> Result<NodeId>;
    async fn get_trace(&self, root_id: &NodeId) -> Result<DecisionTree>;
}
```

#### 2.2.4 Trajectory System

Observable execution stream.

```rust
pub trait TrajectoryEmitter: Send + Sync {
    /// Emit a trajectory event
    async fn emit(&self, event: TrajectoryEvent);

    /// Subscribe to trajectory stream
    fn subscribe(&self) -> impl Stream<Item = TrajectoryEvent>;

    /// Export trajectory for replay
    fn export(&self, format: ExportFormat) -> Result<Vec<u8>>;

    /// Cost tracking
    fn record_cost(&self, component: CostComponent, tokens: TokenUsage);
    fn get_cost_summary(&self) -> CostSummary;
}
```

#### 2.2.5 Epistemic Verification (Strawberry Integration)

Information-theoretic hallucination detection based on Pythea/Strawberry methodology.

**Problem**: LLMs exhibit "procedural hallucinations" where they have correct information but fail to use it correctly, citing evidence that does not support claims or presenting confident answers disconnected from context.

**Solution**: Compute information budget for claims by comparing P(claim|evidence) vs P(claim|no evidence).

```rust
/// Core verification metrics based on Strawberry's trace budget system
pub struct BudgetResult {
    /// P0: Pseudo-prior - P(claim is true | WITHOUT evidence)
    pub p0: f64,
    /// P1: Posterior - P(claim is true | WITH evidence)
    pub p1: f64,
    /// Target confidence claimed
    pub target_confidence: f64,
    /// RequiredBits: KL(target || p0) - bits needed to reach target from baseline
    pub required_bits: f64,
    /// ObservedBits: KL(p1 || p0) - actual info gain from evidence
    pub observed_bits: f64,
    /// BudgetGap: RequiredBits - ObservedBits (positive = insufficient evidence)
    pub budget_gap: f64,
    /// Verification status
    pub status: VerificationStatus,
}

#[derive(Debug, Clone)]
pub enum VerificationStatus {
    Grounded,      // Claim supported by cited evidence
    Unsupported,   // Insufficient evidence supports claim
    Contradicted,  // Evidence contradicts claim
    Unverifiable,  // Cannot determine
}

pub trait EpistemicVerifier: Send + Sync {
    /// Extract atomic claims from response text
    async fn extract_claims(&self, response: &str) -> Result<Vec<Claim>>;

    /// Map claims to their cited evidence spans
    async fn map_evidence(&self, claims: &[Claim], context: &str) -> Result<Vec<ClaimEvidence>>;

    /// Verify a single claim against evidence (computes p0, p1, budget)
    async fn verify_claim(&self, claim: &str, evidence: &str, target_confidence: f64) -> Result<BudgetResult>;

    /// Verify entire response, returns hallucination report
    async fn verify_response(&self, response: &str, context: &str) -> Result<HallucinationReport>;

    /// Audit reasoning trace for procedural hallucinations
    async fn audit_trace(&self, steps: &[TraceStep]) -> Result<Vec<ClaimVerification>>;
}

/// KL divergence between Bernoulli distributions (in bits)
pub fn kl_bernoulli_bits(p: f64, q: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let q = q.clamp(1e-10, 1.0 - 1e-10);
    (p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln()) / 2_f64.ln()
}

/// Compute full budget analysis
pub fn compute_budget(p0: f64, p1: f64, target: f64) -> BudgetResult {
    let required_bits = kl_bernoulli_bits(target, p0);
    let observed_bits = kl_bernoulli_bits(p1, p0);
    let budget_gap = required_bits - observed_bits;

    // Positive gap means evidence doesn't justify confidence
    let status = if budget_gap > 2.0 {
        VerificationStatus::Unsupported
    } else if p1 < 0.3 && p0 > 0.5 {
        VerificationStatus::Contradicted
    } else {
        VerificationStatus::Grounded
    };

    BudgetResult { p0, p1, target_confidence: target, required_bits, observed_bits, budget_gap, status }
}
```

**Integration Points**:

| Integration | Purpose |
|-------------|---------|
| **Memory Gate** | Verify facts before storing in hypergraph (reject unsupported claims) |
| **Output Verification** | Verify agent responses before returning to user |
| **Trace Auditing** | Audit RLM execution traces for procedural hallucinations |
| **REPL Functions** | Expose `verify_claim()`, `audit_reasoning()` in REPL |

**REPL Functions**:
```python
# Available in REPL sandbox
verify_claim(claim, evidence, confidence=0.95)  # -> ClaimVerification
audit_reasoning(steps, sources)                 # -> list[ClaimVerification]
evidence_dependence(question, answer, evidence) # -> float (0=independent, 1=dependent)
detect_hallucinations(response, context)        # -> HallucinationReport
```

**Verification Methods** (Claude-compatible, no logprobs required):
1. **Direct Evidence Check**: Ask verifier model if claim is supported
2. **Evidence Scrubbing**: Compare answers with/without evidence (semantic similarity)
3. **Claim Decomposition**: Break complex claims into atomic verifiable statements

#### 2.2.6 LLM Client Abstraction

Multi-provider client with smart routing.

```rust
pub trait LLMClient: Send + Sync {
    /// Complete a prompt
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Create embeddings
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>>;
}

pub trait SmartRouter: Send + Sync {
    /// Route a query to the best model
    fn route(&self, query: &str, context: &RoutingContext) -> RoutingDecision;
}
```

### 2.3 Platform Adapters

#### 2.3.1 Claude Code Plugin Adapter

Exposes rlm-core via MCP tools and hooks.

```python
# Python binding for Claude Code plugin
class ClaudeCodeAdapter:
    """Adapts rlm-core for Claude Code plugin integration."""

    def __init__(self, core: RLMCore):
        self.core = core

    # MCP Tools
    def rlm_execute(self, task: str) -> dict:
        """Execute task through RLM orchestration."""
        return self.core.run(task)

    def rlm_status(self) -> dict:
        """Get current RLM state and budget."""
        return self.core.get_status()

    def memory_query(self, query: str, limit: int = 10) -> list[dict]:
        """Query persistent memory."""
        return self.core.memory.query(query, limit)

    # Hooks
    def on_session_start(self, context: dict) -> dict:
        """Initialize RLM for session."""
        self.core.initialize(context)
        return {"status": "ready"}

    def on_prompt_submit(self, prompt: str, context: dict) -> dict:
        """Check complexity and inject RLM context."""
        activation = self.core.should_activate(prompt, context)
        return {"activate_rlm": activation.should_activate, "reason": activation.reason}
```

#### 2.3.2 TUI Adapter (Go)

Exposes rlm-core for Bubble Tea integration.

```go
// Go binding for TUI integration
type TUIAdapter struct {
    core *rlmcore.Core
}

// BubbleTea Model integration
func (a *TUIAdapter) Update(msg tea.Msg) tea.Cmd {
    switch msg := msg.(type) {
    case TrajectoryEvent:
        // Update TUI panels with trajectory events
        return a.handleTrajectoryEvent(msg)
    case REPLOutput:
        // Update REPL view
        return a.handleREPLOutput(msg)
    }
    return nil
}

// Panels
func (a *TUIAdapter) RenderRLMTrace() string
func (a *TUIAdapter) RenderREPLView() string
func (a *TUIAdapter) RenderMemoryInspector() string
func (a *TUIAdapter) RenderBudgetStatus() string
```

---

## 3. Core Types

### 3.1 Session Context

```rust
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub messages: Vec<Message>,
    pub files: HashMap<String, String>,
    pub tool_outputs: Vec<ToolOutput>,
    pub working_memory: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub tool_name: String,
    pub content: String,
    pub exit_code: Option<i32>,
    pub timestamp: Option<DateTime<Utc>>,
}
```

### 3.2 Complexity Signals

```rust
#[derive(Debug, Clone)]
pub struct TaskComplexitySignals {
    // Prompt analysis
    pub references_multiple_files: bool,
    pub requires_cross_context_reasoning: bool,
    pub involves_temporal_reasoning: bool,
    pub asks_about_patterns: bool,
    pub debugging_task: bool,
    pub requires_exhaustive_search: bool,
    pub security_review_task: bool,
    pub architecture_analysis: bool,

    // User intent
    pub user_wants_thorough: bool,
    pub user_wants_fast: bool,

    // Context analysis
    pub context_has_multiple_domains: bool,
    pub recent_tool_outputs_large: bool,
    pub files_span_multiple_modules: bool,

    // Historical
    pub previous_turn_was_confused: bool,
    pub task_is_continuation: bool,
}

#[derive(Debug, Clone)]
pub struct ActivationDecision {
    pub should_activate: bool,
    pub reason: String,
    pub score: i32,
    pub signals: TaskComplexitySignals,
}
```

### 3.3 Memory Types

```rust
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub node_type: NodeType,
    pub subtype: Option<String>,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub tier: Tier,
    pub confidence: f64,
    pub provenance: Option<Provenance>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Entity,     // Code elements (files, functions, types)
    Fact,       // Extracted knowledge
    Experience, // Interaction patterns
    Decision,   // Reasoning trace node
    Snippet,    // Verbatim content with provenance
}

#[derive(Debug, Clone)]
pub enum Tier {
    Task,     // Working memory (single task)
    Session,  // Accumulated (current session)
    LongTerm, // Persistent (across sessions)
    Archive,  // Decayed but preserved
}

#[derive(Debug, Clone)]
pub struct HyperEdge {
    pub id: EdgeId,
    pub edge_type: EdgeType,
    pub label: Option<String>,
    pub weight: f64,
    pub members: Vec<EdgeMember>,
}

#[derive(Debug, Clone)]
pub struct EdgeMember {
    pub node_id: NodeId,
    pub role: String,  // subject, object, context, participant
    pub position: u32,
}
```

### 3.4 Trajectory Events

```rust
#[derive(Debug, Clone)]
pub enum TrajectoryEventType {
    RlmStart,
    Analyze,
    ReplExec,
    ReplResult,
    Reason,
    RecurseStart,
    RecurseEnd,
    Final,
    Error,
    ToolUse,
    CostReport,
    // Epistemic verification events (Strawberry integration)
    VerifyStart,      // Beginning verification of response/trace
    ClaimExtracted,   // Atomic claim identified
    EvidenceChecked,  // Claim verified against evidence
    BudgetComputed,   // p0, p1, budget gap calculated
    HallucinationFlag,// Claim flagged as potentially hallucinated
    VerifyComplete,   // Verification report ready
}

#[derive(Debug, Clone)]
pub struct TrajectoryEvent {
    pub event_type: TrajectoryEventType,
    pub depth: u32,
    pub content: String,
    pub metadata: Option<HashMap<String, Value>>,
    pub timestamp: DateTime<Utc>,
}
```

---

## 4. Key Design Decisions

### 4.1 Language Choice: Rust with Bindings

**Decision**: Core library in Rust with Python (PyO3) and Go (CGO) bindings.

**Rationale**:
- Performance: REPL sandbox, memory queries, and trajectory streaming benefit from Rust's zero-cost abstractions
- Safety: Memory safety critical for sandboxed execution
- Portability: Single codebase for both targets
- Python bindings: PyO3 provides excellent Python integration for Claude Code
- Go bindings: CGO allows integration with Bubble Tea TUI

**Alternatives Considered**:
- Pure Python: Would require rewriting for TUI
- Pure Go: Would require rewriting for Claude Code
- Dual implementation: Maintenance burden, feature drift

### 4.2 REPL Architecture: Subprocess vs Embedded

**Decision**: External Python subprocess with JSON-RPC IPC.

**Rationale**:
- Isolation: Crashes don't affect host process
- Resource limits: OS-level memory/CPU limits
- Ecosystem access: Full Python ecosystem available
- RestrictedPython limitations: Can't restrict all dangerous operations

**Implementation**:
```
┌─────────────┐  JSON-RPC  ┌─────────────┐
│  rlm-core   │◄──────────►│  Python     │
│  (Rust)     │  stdin/out │  Subprocess │
└─────────────┘            └─────────────┘
```

### 4.3 Memory: Unified Hypergraph

**Decision**: Full hypergraph memory with 3-tier evolution, unifying both systems.

**Rationale**:
- Recurse's hypergraph is more expressive than rlm-claude-code's simpler store
- Tier system (task/session/longterm) works for both contexts
- Evolution operations (consolidate/promote/decay) prevent unbounded growth
- Reasoning traces integrate naturally as memory nodes

### 4.4 Smart Routing: Query-Aware Model Selection

**Decision**: Unified smart router that considers query type, depth, and budget.

**Rationale**:
- Both systems benefit from model tiering (expensive for orchestration, cheap for extraction)
- Query classification helps select appropriate model
- Depth-based routing (Opus → Sonnet → Haiku) optimizes cost

**Routing Table**:

| Query Type | Depth 0 | Depth 1 | Depth 2+ |
|------------|---------|---------|----------|
| Architecture | Opus | Sonnet | Sonnet |
| Multi-file | Opus | Sonnet | Haiku |
| Debugging | Sonnet | Haiku | Haiku |
| Extraction | Sonnet | Haiku | Haiku |
| Simple | Haiku | Haiku | Haiku |

### 4.5 Trajectory: Unified Event Schema

**Decision**: Single trajectory event schema usable by both Claude Code and TUI.

**Rationale**:
- Same events render differently in each context
- Claude Code: Streaming text output
- TUI: Bubble Tea panel updates
- Both: Export to JSON for analysis/replay

---

## 5. Interface Contracts

### 5.1 Claude Code Plugin Contract

The plugin adapter must provide:

```python
# MCP Tools (exposed to Claude Code)
@mcp_tool
def rlm_execute(task: str, context: dict) -> AsyncIterator[dict]:
    """Stream trajectory events during RLM execution."""

@mcp_tool
def rlm_status() -> dict:
    """Return current RLM state, budget, depth."""

@mcp_tool
def memory_query(query: str, tier: str = "all", limit: int = 10) -> list[dict]:
    """Semantic search across memory tiers."""

@mcp_tool
def memory_store(content: str, node_type: str, confidence: float = 0.8) -> str:
    """Store fact/experience in memory, returns node ID."""

# Hooks
@hook("SessionStart")
def on_session_start(context: dict) -> dict:
    """Initialize RLM service for session."""

@hook("UserPromptSubmit")
def on_prompt_submit(prompt: str, context: dict) -> dict:
    """Check complexity, inject memory context."""

@hook("PreCompact")
def on_pre_compact(context: dict) -> dict:
    """Consolidate task memory before context compression."""
```

### 5.2 TUI Application Contract

The TUI adapter must provide:

```go
// Core integration
type TUIAdapter interface {
    // Initialize with configuration
    Init(config Config) error

    // Execute RLM task, returns event channel
    Execute(ctx context.Context, task string) (<-chan TrajectoryEvent, error)

    // Memory operations
    QueryMemory(query string, limit int) ([]Node, error)
    StoreMemory(node Node) (NodeID, error)

    // Budget tracking
    GetBudgetStatus() BudgetStatus
    SetBudgetLimits(limits BudgetLimits) error
}

// Bubble Tea integration
type TUIModel interface {
    // Standard Bubble Tea
    Init() tea.Cmd
    Update(msg tea.Msg) (tea.Model, tea.Cmd)
    View() string

    // Panel rendering
    RenderRLMTrace() string
    RenderREPLView() string
    RenderMemoryInspector() string
    RenderReasoningTrace() string
    RenderBudgetStatus() string
}
```

---

## 6. Implementation Phases

### Phase 1: Core Engine (2-3 weeks)

**Goal**: Minimal RLM orchestration loop working in isolation.

| Task | Description |
|------|-------------|
| Types crate | Core types (SessionContext, TrajectoryEvent, etc.) |
| REPL subprocess | Python subprocess with JSON-RPC protocol |
| Basic orchestrator | Externalize → Execute → Synthesize loop |
| Complexity classifier | Pattern-based activation decision |
| Unit tests | Full coverage of core loop |

**Deliverables**:
- `rlm-core` Rust crate with basic orchestration
- Python bootstrap script for REPL
- Test harness

### Phase 2: Memory System (2 weeks)

**Goal**: Full hypergraph memory with tier evolution.

| Task | Description |
|------|-------------|
| SQLite schema | Nodes, hyperedges, membership, evolution log |
| Memory store | CRUD operations, semantic search |
| Tier management | Task/session/longterm lifecycle |
| Evolution ops | Consolidate, promote, decay algorithms |
| Integration tests | Memory persistence and evolution |

**Deliverables**:
- Memory subsystem integrated into rlm-core
- Schema migrations
- Evolution test suite

### Phase 3: LLM & Routing (1-2 weeks)

**Goal**: Multi-provider client with smart routing.

| Task | Description |
|------|-------------|
| Client abstraction | Anthropic, OpenAI, OpenRouter |
| Smart router | Query classification, model selection |
| Cost tracking | Per-component token/cost accounting |
| Prompt caching | Cache key generation, hit tracking |

**Deliverables**:
- LLM client with routing
- Cost tracking dashboard data

### Phase 4: Python Bindings (1 week)

**Goal**: PyO3 bindings for Claude Code plugin.

| Task | Description |
|------|-------------|
| PyO3 setup | Build configuration, CI |
| Core bindings | Orchestrator, Memory, Trajectory |
| Async support | asyncio integration |
| Package | PyPI-publishable package |

**Deliverables**:
- `rlm-core` Python package
- Example Claude Code plugin using bindings

### Phase 5: Go Bindings (1 week)

**Goal**: CGO bindings for TUI integration.

| Task | Description |
|------|-------------|
| CGO setup | Build configuration, CI |
| Core bindings | Orchestrator, Memory, Trajectory |
| Channel bridge | Go channels ↔ Rust streams |
| Package | Go module |

**Deliverables**:
- `rlm-core` Go module
- Example Bubble Tea integration

### Phase 6: Epistemic Verification (1-2 weeks)

**Goal**: Information-theoretic hallucination detection (Strawberry integration).

| Task | Description |
|------|-------------|
| KL divergence | Bernoulli KL, budget computation, interval arithmetic |
| Claim extractor | Parse responses into atomic claims |
| Evidence scrubber | Mask cited evidence for p0 estimation |
| Verification backend | Self/Haiku/external verifier options |
| Memory gate | Reject ungrounded facts from hypergraph storage |
| REPL functions | `verify_claim()`, `audit_reasoning()`, etc. |

**Deliverables**:
- `epistemic` module with verification traits and types
- Memory gate integration (SPEC-08.15-18)
- REPL verification functions
- Verification test suite

### Phase 7: Adapters (2 weeks)

**Goal**: Full adapters for both deployment targets.

| Task | Description |
|------|-------------|
| Claude Code adapter | MCP tools, hooks, skill |
| TUI adapter | Bubble Tea model, panel renderers |
| Integration tests | End-to-end tests for both contexts |
| Documentation | Usage guides, API reference |

**Deliverables**:
- Complete rlm-claude-code plugin using rlm-core
- TUI application example
- Documentation

---

## 7. Success Criteria

### 7.1 Functional Requirements

- [historical target] RLM orchestration loop works identically in both contexts
- [historical target] Memory persists and evolves correctly across sessions
- [historical target] Trajectory events stream to both Claude Code and TUI
- [historical target] Smart routing selects appropriate models
- [historical target] Cost tracking accurate across all components
- [historical target] REPL sandbox prevents unauthorized operations
- [historical target] Epistemic verification detects >80% of procedural hallucinations
- [historical target] Memory gate rejects ungrounded facts (budget gap > threshold)
- [historical target] Verification adds <500ms latency in sample mode

### 7.2 Performance Requirements

| Metric | Target |
|--------|--------|
| REPL execution latency | < 100ms for simple operations |
| Memory query latency | < 200ms for semantic search |
| Trajectory event latency | < 10ms per event |
| Cold start time | < 2s for full initialization |

### 7.3 Quality Requirements

- [historical target] > 80% test coverage on core crate
- [historical target] All public APIs documented
- [historical target] No memory leaks in long-running sessions
- [historical target] Graceful degradation when services unavailable

---

## 8. Migration Path

### 8.1 rlm-claude-code Migration

1. **Phase 1**: Add rlm-core as optional dependency, feature-flagged
2. **Phase 2**: Migrate orchestrator to use rlm-core
3. **Phase 3**: Migrate memory to rlm-core hypergraph
4. **Phase 4**: Remove legacy Python implementation
5. **Phase 5**: Full rlm-core native

### 8.2 recurse Migration

1. **Phase 1**: Add rlm-core Go bindings, parallel implementation
2. **Phase 2**: Migrate RLM service to use rlm-core
3. **Phase 3**: Migrate memory to rlm-core hypergraph
4. **Phase 4**: Remove legacy Go implementation
5. **Phase 5**: Full rlm-core native with Go UI layer

---

## 9. Open Questions

### 9.1 Architecture

1. **Embedding storage**: Should embeddings be stored in SQLite (portable) or external vector store (scalable)?
   - Proposal: SQLite with VSS extension for small scale, optional Qdrant adapter for large

2. **Cross-project memory**: How should memory be shared across projects?
   - Proposal: Per-project by default, optional global tier

3. **Checkpoint format**: Binary or JSON for session checkpoints?
   - Proposal: MessagePack for balance of size and inspectability

### 9.2 Implementation

1. **REPL language**: Should we support REPLs other than Python?
   - Proposal: Python only initially, interface allows future extension

2. **Async runtime**: Tokio vs async-std for Rust async?
   - Proposal: Tokio (more mature, better ecosystem)

3. **Error handling**: How to surface errors across language boundaries?
   - Proposal: Structured error types with codes and messages

---

## 10. References

### Research Papers
- [RLM Paper](https://arxiv.org/abs/2512.24601) - Prompts as manipulable objects
- [HGMem Paper](https://arxiv.org/abs/2512.23959) - Hypergraph memory
- [ACE Paper](https://arxiv.org/abs/2510.04618) - Structured context evolution
- [MemEvolve Paper](https://arxiv.org/abs/2512.18746) - Memory architecture adaptation
- [Semantic Entropy](https://www.nature.com/articles/s41586-024-07421-0) - Entropy-based confabulation detection

### Projects
- [Pythea/Strawberry](https://github.com/leochlon/pythea/tree/main/strawberry) - Hallucination detection toolkit (KL-based trace budget)
- [Deciduous](https://github.com/notactuallytreyanastasio/deciduous) - Decision tree reasoning
- [recurse](https://github.com/rand/recurse) - Agentic TUI with RLM (SPEC-08: Hallucination Detection)
- [rlm-claude-code](https://github.com/rand/rlm-claude-code) - Claude Code plugin (SPEC-16: Epistemic Verification)

### Implementation
- [PyO3](https://pyo3.rs/) - Rust-Python bindings
- [CGO](https://pkg.go.dev/cmd/cgo) - Rust-Go interop via C FFI
