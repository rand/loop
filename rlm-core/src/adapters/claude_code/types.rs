//! Types for the Claude Code adapter.

use crate::memory::{Node, NodeId};
use crate::orchestrator::ExecutionMode;
use crate::trajectory::{BudgetConfig, BudgetState, CostSummary, Verbosity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Configuration for the Claude Code adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Path to memory database (None = in-memory)
    pub memory_path: Option<String>,
    /// Default execution mode
    pub default_mode: ExecutionMode,
    /// Budget configuration
    pub budget: BudgetConfig,
    /// Verbosity level for trajectory events
    pub verbosity: Verbosity,
    /// Whether to auto-escalate mode based on complexity
    pub auto_escalate: bool,
    /// Minimum complexity score to trigger auto-escalation
    pub escalation_threshold: i32,
    /// Whether to enable memory persistence
    pub persist_memory: bool,
    /// Session ID for tracking
    pub session_id: Option<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            memory_path: None,
            default_mode: ExecutionMode::Micro,
            budget: BudgetConfig::default(),
            verbosity: Verbosity::Normal,
            auto_escalate: true,
            escalation_threshold: 3,
            persist_memory: true,
            session_id: None,
        }
    }
}

impl AdapterConfig {
    /// Create a minimal config for testing.
    pub fn testing() -> Self {
        Self {
            memory_path: None,
            default_mode: ExecutionMode::Micro,
            budget: BudgetConfig::unlimited(),
            verbosity: Verbosity::Debug,
            auto_escalate: false,
            escalation_threshold: 10,
            persist_memory: false,
            session_id: Some("test".to_string()),
        }
    }

    /// Set the memory path.
    pub fn with_memory_path(mut self, path: impl Into<String>) -> Self {
        self.memory_path = Some(path.into());
        self
    }

    /// Set the default execution mode.
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.default_mode = mode;
        self
    }

    /// Set the budget configuration.
    pub fn with_budget(mut self, budget: BudgetConfig) -> Self {
        self.budget = budget;
        self
    }

    /// Set the session ID.
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }
}

/// Current status of the adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    /// Current execution mode
    pub mode: ExecutionMode,
    /// Budget state
    pub budget: BudgetState,
    /// Memory statistics
    pub memory_stats: MemoryStatus,
    /// Whether RLM is currently executing
    pub is_executing: bool,
    /// Session ID
    pub session_id: Option<String>,
    /// When the adapter was initialized
    pub initialized_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl AdapterStatus {
    /// Create initial status.
    pub fn new(mode: ExecutionMode, session_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            mode,
            budget: BudgetState::default(),
            memory_stats: MemoryStatus::default(),
            is_executing: false,
            session_id,
            initialized_at: now,
            last_activity: now,
        }
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Memory subsystem status.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStatus {
    /// Total nodes in memory
    pub total_nodes: u64,
    /// Nodes by tier
    pub nodes_by_tier: HashMap<String, u64>,
    /// Total edges
    pub total_edges: u64,
    /// Whether memory is persisted
    pub is_persisted: bool,
}

/// Request to execute RLM orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmRequest {
    /// The query to process
    pub query: String,
    /// Execution mode override (None = use current mode)
    pub mode: Option<ExecutionMode>,
    /// Additional context
    pub context: Option<RequestContext>,
    /// Whether to force RLM activation regardless of complexity
    pub force_activation: bool,
    /// Maximum budget for this request
    pub max_budget_usd: Option<f64>,
}

impl RlmRequest {
    /// Create a simple request with just a query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: None,
            context: None,
            force_activation: false,
            max_budget_usd: None,
        }
    }

    /// Set the execution mode.
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set additional context.
    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Force RLM activation.
    pub fn force_activation(mut self) -> Self {
        self.force_activation = true;
        self
    }

    /// Set maximum budget.
    pub fn with_budget(mut self, max_usd: f64) -> Self {
        self.max_budget_usd = Some(max_usd);
        self
    }
}

/// Additional context for an RLM request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestContext {
    /// Recent messages from the conversation
    pub messages: Vec<ContextMessage>,
    /// File contents by path
    pub files: HashMap<String, String>,
    /// Recent tool outputs
    pub tool_outputs: Vec<ToolOutputContext>,
    /// Working memory entries
    pub working_memory: HashMap<String, Value>,
}

impl RequestContext {
    /// Create empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message.
    pub fn with_message(mut self, role: &str, content: impl Into<String>) -> Self {
        self.messages.push(ContextMessage {
            role: role.to_string(),
            content: content.into(),
        });
        self
    }

    /// Add a file.
    pub fn with_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }

    /// Add working memory.
    pub fn with_memory(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.working_memory.insert(key.into(), value.into());
        self
    }
}

/// A message in the request context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
}

/// Tool output in the request context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutputContext {
    pub tool_name: String,
    pub content: String,
    pub exit_code: Option<i32>,
}

/// Response from RLM execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmResponse {
    /// Whether RLM was activated
    pub activated: bool,
    /// Reason for activation/skip decision
    pub activation_reason: String,
    /// The execution mode used
    pub mode: ExecutionMode,
    /// Final answer/result (if completed)
    pub answer: Option<String>,
    /// Cost summary
    pub cost: CostSummary,
    /// Whether execution completed successfully
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution metadata
    pub metadata: ResponseMetadata,
}

impl RlmResponse {
    /// Create a skip response (RLM not activated).
    pub fn skip(reason: impl Into<String>, mode: ExecutionMode) -> Self {
        Self {
            activated: false,
            activation_reason: reason.into(),
            mode,
            answer: None,
            cost: CostSummary::default(),
            success: true,
            error: None,
            metadata: ResponseMetadata::default(),
        }
    }

    /// Create a success response.
    pub fn success(answer: impl Into<String>, mode: ExecutionMode, cost: CostSummary) -> Self {
        Self {
            activated: true,
            activation_reason: "Complexity threshold exceeded".to_string(),
            mode,
            answer: Some(answer.into()),
            cost,
            success: true,
            error: None,
            metadata: ResponseMetadata::default(),
        }
    }

    /// Create an error response.
    pub fn error(error: impl Into<String>, mode: ExecutionMode) -> Self {
        Self {
            activated: true,
            activation_reason: String::new(),
            mode,
            answer: None,
            cost: CostSummary::default(),
            success: false,
            error: Some(error.into()),
            metadata: ResponseMetadata::default(),
        }
    }

    /// Set the activation reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.activation_reason = reason.into();
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Metadata from RLM execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Complexity score that triggered activation
    pub complexity_score: i32,
    /// Signals that were detected
    pub signals: Vec<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Recursion depth reached
    pub max_depth_reached: u32,
    /// Whether REPL was used
    pub used_repl: bool,
    /// Number of memory queries
    pub memory_queries: u32,
    /// Number of memory stores
    pub memory_stores: u32,
}

/// Session context for hook handlers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionContext {
    /// Session identifier
    pub session_id: String,
    /// Working directory
    pub working_directory: Option<String>,
    /// Project root (if detected)
    pub project_root: Option<String>,
    /// Git branch (if in a git repo)
    pub git_branch: Option<String>,
    /// Environment variables to expose
    pub env_vars: HashMap<String, String>,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}

impl SessionContext {
    /// Create a new session context.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            working_directory: None,
            project_root: None,
            git_branch: None,
            env_vars: HashMap::new(),
            started_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set working directory.
    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Set project root.
    pub fn with_project_root(mut self, root: impl Into<String>) -> Self {
        self.project_root = Some(root.into());
        self
    }

    /// Set git branch.
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Add environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Enhancement data for user prompts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptEnhancement {
    /// Additional context to prepend
    pub prepend_context: Option<String>,
    /// Additional context to append
    pub append_context: Option<String>,
    /// Suggested mode based on prompt analysis
    pub suggested_mode: Option<ExecutionMode>,
    /// Whether RLM should be activated
    pub should_activate_rlm: bool,
    /// Complexity signals detected
    pub signals: Vec<String>,
    /// Relevant memory nodes to include
    pub relevant_memories: Vec<RelevantMemory>,
}

impl PromptEnhancement {
    /// Create an empty enhancement (no changes).
    pub fn none() -> Self {
        Self::default()
    }

    /// Create enhancement with context prepended.
    pub fn with_prepend(context: impl Into<String>) -> Self {
        Self {
            prepend_context: Some(context.into()),
            ..Self::default()
        }
    }

    /// Set suggested mode.
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.suggested_mode = Some(mode);
        self
    }

    /// Set activation flag.
    pub fn with_activation(mut self, activate: bool) -> Self {
        self.should_activate_rlm = activate;
        self
    }

    /// Add a relevant memory.
    pub fn with_memory(mut self, memory: RelevantMemory) -> Self {
        self.relevant_memories.push(memory);
        self
    }
}

/// A relevant memory node for prompt enhancement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantMemory {
    /// Node ID
    pub id: String,
    /// Content summary
    pub content: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f64,
    /// Node type
    pub node_type: String,
}

impl RelevantMemory {
    /// Create from a memory node.
    pub fn from_node(node: &Node, relevance: f64) -> Self {
        Self {
            id: node.id.to_string(),
            content: node.content.clone(),
            relevance,
            node_type: node.node_type.to_string(),
        }
    }
}

/// Data to preserve during context compaction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompactData {
    /// Critical facts to preserve
    pub critical_facts: Vec<String>,
    /// Important decisions made
    pub decisions: Vec<String>,
    /// Memory nodes to promote to long-term
    pub nodes_to_promote: Vec<NodeId>,
    /// Summary of work done
    pub work_summary: Option<String>,
    /// State to restore after compaction
    pub restore_state: HashMap<String, Value>,
}

impl CompactData {
    /// Create empty compact data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a critical fact.
    pub fn with_fact(mut self, fact: impl Into<String>) -> Self {
        self.critical_facts.push(fact.into());
        self
    }

    /// Add a decision.
    pub fn with_decision(mut self, decision: impl Into<String>) -> Self {
        self.decisions.push(decision.into());
        self
    }

    /// Add a node to promote.
    pub fn with_promotion(mut self, node_id: NodeId) -> Self {
        self.nodes_to_promote.push(node_id);
        self
    }

    /// Set work summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.work_summary = Some(summary.into());
        self
    }

    /// Add restore state.
    pub fn with_state(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.restore_state.insert(key.into(), value.into());
        self
    }

    /// Check if there's any data to compact.
    pub fn is_empty(&self) -> bool {
        self.critical_facts.is_empty()
            && self.decisions.is_empty()
            && self.nodes_to_promote.is_empty()
            && self.work_summary.is_none()
            && self.restore_state.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_config_default() {
        let config = AdapterConfig::default();
        assert_eq!(config.default_mode, ExecutionMode::Micro);
        assert!(config.auto_escalate);
        assert!(config.persist_memory);
    }

    #[test]
    fn test_adapter_config_testing() {
        let config = AdapterConfig::testing();
        assert!(!config.auto_escalate);
        assert!(!config.persist_memory);
        assert_eq!(config.session_id, Some("test".to_string()));
    }

    #[test]
    fn test_rlm_request_builder() {
        let request = RlmRequest::new("Analyze the codebase")
            .with_mode(ExecutionMode::Thorough)
            .force_activation()
            .with_budget(2.0);

        assert_eq!(request.query, "Analyze the codebase");
        assert_eq!(request.mode, Some(ExecutionMode::Thorough));
        assert!(request.force_activation);
        assert_eq!(request.max_budget_usd, Some(2.0));
    }

    #[test]
    fn test_rlm_response_skip() {
        let response = RlmResponse::skip("Simple query", ExecutionMode::Micro);
        assert!(!response.activated);
        assert!(response.success);
        assert!(response.answer.is_none());
    }

    #[test]
    fn test_rlm_response_success() {
        let response =
            RlmResponse::success("Here's the analysis", ExecutionMode::Balanced, CostSummary::new());
        assert!(response.activated);
        assert!(response.success);
        assert_eq!(response.answer, Some("Here's the analysis".to_string()));
    }

    #[test]
    fn test_session_context_builder() {
        let ctx = SessionContext::new("test-session")
            .with_working_directory("/home/user/project")
            .with_git_branch("main");

        assert_eq!(ctx.session_id, "test-session");
        assert_eq!(ctx.working_directory, Some("/home/user/project".to_string()));
        assert_eq!(ctx.git_branch, Some("main".to_string()));
    }

    #[test]
    fn test_compact_data_builder() {
        let data = CompactData::new()
            .with_fact("The API uses REST")
            .with_decision("Chose SQLite for storage")
            .with_summary("Implemented authentication");

        assert_eq!(data.critical_facts.len(), 1);
        assert_eq!(data.decisions.len(), 1);
        assert!(data.work_summary.is_some());
        assert!(!data.is_empty());
    }

    #[test]
    fn test_prompt_enhancement() {
        let enhancement = PromptEnhancement::with_prepend("Context: Working on auth module")
            .with_mode(ExecutionMode::Balanced)
            .with_activation(true);

        assert!(enhancement.prepend_context.is_some());
        assert_eq!(enhancement.suggested_mode, Some(ExecutionMode::Balanced));
        assert!(enhancement.should_activate_rlm);
    }
}
