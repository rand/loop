//! Main Claude Code adapter implementation.
//!
//! The adapter coordinates all rlm-core functionality for Claude Code integration:
//! - Orchestration execution
//! - Memory management
//! - Hook handling
//! - MCP tool dispatch

use super::hooks::{
    HookContext, HookData, HookHandler, HookRegistry, HookResult, HookResultData, HookTrigger,
    PreCompactHandler, PromptAnalysisHandler, SessionStartHandler,
};
use super::mcp::{McpToolRegistry, ToolHandler};
use super::skills::{RlmSkill, SkillRegistry};
use super::types::{
    AdapterConfig, AdapterStatus, CompactData, MemoryStatus, PromptEnhancement, RequestContext,
    ResponseMetadata, RlmRequest, RlmResponse, SessionContext,
};
use crate::complexity::PatternClassifier;
use crate::error::{Error, Result};
use crate::memory::{Node, NodeId, NodeQuery, NodeType, SqliteMemoryStore, Tier};
use crate::orchestrator::ExecutionMode;
use crate::trajectory::{BudgetManager, CostSummary, CollectingEmitter, TrajectoryEmitter};
use std::sync::{Arc, RwLock};

/// The main Claude Code adapter.
///
/// Coordinates RLM orchestration, memory, hooks, and MCP tools for
/// Claude Code plugin integration.
pub struct ClaudeCodeAdapter {
    /// Configuration
    config: AdapterConfig,
    /// Memory store
    memory: Arc<SqliteMemoryStore>,
    /// Budget manager
    budget: Arc<BudgetManager>,
    /// Trajectory emitter
    emitter: Arc<RwLock<CollectingEmitter>>,
    /// Hook registry
    hooks: Arc<RwLock<HookRegistry>>,
    /// MCP tool registry
    tools: Arc<RwLock<McpToolRegistry>>,
    /// Skill registry
    skills: Arc<SkillRegistry>,
    /// Complexity classifier
    classifier: PatternClassifier,
    /// Current execution mode
    mode: Arc<RwLock<ExecutionMode>>,
    /// Current status
    status: Arc<RwLock<AdapterStatus>>,
    /// Whether currently executing
    executing: Arc<RwLock<bool>>,
}

impl ClaudeCodeAdapter {
    /// Create a new adapter with the given configuration.
    pub fn new(config: AdapterConfig) -> Result<Self> {
        // Initialize memory store
        let memory = if let Some(ref path) = config.memory_path {
            SqliteMemoryStore::open(path)?
        } else {
            SqliteMemoryStore::in_memory()?
        };

        // Initialize budget manager
        let budget = BudgetManager::new(config.budget.clone());

        // Initialize emitter
        let mut emitter = CollectingEmitter::new();
        emitter.set_verbosity(config.verbosity);

        // Initialize hook registry with defaults
        let mut hooks = HookRegistry::new();
        hooks.register(Box::new(SessionStartHandler::new()));
        hooks.register(Box::new(PromptAnalysisHandler::new()));
        hooks.register(Box::new(PreCompactHandler::new()));

        // Initialize tool registry
        let tools = McpToolRegistry::with_defaults();

        // Initialize skill registry
        let skills = SkillRegistry::with_defaults();

        // Initialize classifier
        let classifier = PatternClassifier::with_threshold(config.escalation_threshold);

        // Initialize status
        let status = AdapterStatus::new(config.default_mode, config.session_id.clone());

        Ok(Self {
            config,
            memory: Arc::new(memory),
            budget: Arc::new(budget),
            emitter: Arc::new(RwLock::new(emitter)),
            hooks: Arc::new(RwLock::new(hooks)),
            tools: Arc::new(RwLock::new(tools)),
            skills: Arc::new(skills),
            classifier,
            mode: Arc::new(RwLock::new(status.mode)),
            status: Arc::new(RwLock::new(status)),
            executing: Arc::new(RwLock::new(false)),
        })
    }

    /// Create an adapter with default configuration.
    pub fn default_adapter() -> Result<Self> {
        Self::new(AdapterConfig::default())
    }

    /// Create an adapter for testing.
    pub fn testing() -> Result<Self> {
        Self::new(AdapterConfig::testing())
    }

    // =========================================================================
    // Core Execution
    // =========================================================================

    /// Execute RLM orchestration for a request.
    pub async fn execute(&self, request: RlmRequest) -> Result<RlmResponse> {
        // Check if already executing
        {
            let mut executing = self.executing.write().map_err(|_| Error::Internal("Lock error".into()))?;
            if *executing {
                return Err(Error::Internal("RLM already executing".into()));
            }
            *executing = true;
        }

        // Ensure we reset executing flag on exit
        let _guard = ExecutionGuard {
            executing: self.executing.clone(),
        };

        // Update status
        {
            let mut status = self.status.write().map_err(|_| Error::Internal("Lock error".into()))?;
            status.is_executing = true;
            status.touch();
        }

        // Determine execution mode
        let mode = request.mode.unwrap_or_else(|| {
            *self.mode.read().unwrap_or_else(|_| panic!("Lock poisoned"))
        });

        // Build session context for complexity analysis
        let session_ctx = self.build_session_context(&request.context);

        // Analyze complexity
        let decision = self.classifier.should_activate(&request.query, &session_ctx);

        // Determine if we should activate
        let should_activate = request.force_activation || decision.should_activate;

        if !should_activate {
            return Ok(RlmResponse::skip(decision.reason.clone(), mode)
                .with_metadata(ResponseMetadata {
                    complexity_score: decision.score,
                    signals: decision.signals.active_signals().iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                }));
        }

        // Auto-escalate mode if configured
        let final_mode = if self.config.auto_escalate {
            ExecutionMode::from_signals(&decision.signals)
        } else {
            mode
        };

        // Apply budget limit if specified
        if let Some(max_budget) = request.max_budget_usd {
            // Budget enforcement happens via the budget manager
            if self.budget.state().current_cost_usd >= max_budget {
                return Err(Error::budget_exhausted("Request budget"));
            }
        }

        // Execute orchestration
        // For now, this is a placeholder that demonstrates the flow
        // Full implementation would use the Orchestrator trait
        let start_time = std::time::Instant::now();

        // Simulate some work and record costs
        let usage = crate::trajectory::TokenUsage::new(1000, 500);
        let cost = crate::trajectory::Model::ClaudeSonnet4.calculate_cost(&usage);
        self.budget.record_cost(cost, usage.total());

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Build response
        let mut cost_summary = CostSummary::new();
        cost_summary.add(
            crate::trajectory::CostComponent::Orchestration,
            usage,
            cost,
        );

        let metadata = ResponseMetadata {
            complexity_score: decision.score,
            signals: decision.signals.active_signals().iter().map(|s| s.to_string()).collect(),
            duration_ms,
            max_depth_reached: 1,
            used_repl: false,
            memory_queries: 0,
            memory_stores: 0,
        };

        // Placeholder answer - real implementation would run full orchestration
        let answer = format!(
            "RLM Analysis (mode: {})\n\nQuery: {}\n\nComplexity Score: {}\nSignals: {:?}\n\n\
             [Placeholder: Full orchestration would analyze and respond here]",
            final_mode,
            request.query,
            decision.score,
            metadata.signals
        );

        Ok(RlmResponse::success(answer, final_mode, cost_summary)
            .with_reason(decision.reason)
            .with_metadata(metadata))
    }

    /// Get current adapter status.
    pub fn status(&self) -> AdapterStatus {
        let mut status = self.status.read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| AdapterStatus::new(ExecutionMode::Micro, None));

        // Update with current values
        status.budget = self.budget.state();
        status.mode = *self.mode.read().unwrap_or_else(|_| panic!("Lock poisoned"));
        status.is_executing = *self.executing.read().unwrap_or_else(|_| panic!("Lock poisoned"));

        // Update memory stats
        if let Ok(stats) = self.memory.stats() {
            status.memory_stats = MemoryStatus {
                total_nodes: stats.total_nodes,
                nodes_by_tier: stats.nodes_by_tier.iter()
                    .map(|(k, v)| (k.to_string(), *v as u64))
                    .collect(),
                total_edges: stats.total_edges,
                is_persisted: self.config.memory_path.is_some(),
            };
        }

        status
    }

    // =========================================================================
    // Hook Handling
    // =========================================================================

    /// Handle session start event.
    pub async fn handle_session_start(&self, context: SessionContext) -> Result<HookResult> {
        let hook_ctx = HookContext::new(HookTrigger::SessionStart, context);

        let hooks = self.hooks.read().map_err(|_| Error::Internal("Lock error".into()))?;
        let results = hooks.execute(hook_ctx).await?;

        // Return first result or default
        Ok(results.into_iter().next().unwrap_or_else(HookResult::ok))
    }

    /// Handle user prompt submission.
    pub async fn handle_prompt_submit(&self, prompt: &str, context: SessionContext) -> Result<PromptEnhancement> {
        let hook_ctx = HookContext::new(HookTrigger::UserPromptSubmit, context)
            .with_data(HookData::PromptSubmit {
                prompt: prompt.to_string(),
                recent_messages: Vec::new(),
            });

        let hooks = self.hooks.read().map_err(|_| Error::Internal("Lock error".into()))?;
        let results = hooks.execute(hook_ctx).await?;

        // Extract enhancement from results
        for result in results {
            if let HookResultData::PromptEnhancement(enhancement) = result.data {
                return Ok(enhancement);
            }
        }

        Ok(PromptEnhancement::none())
    }

    /// Handle pre-compact event.
    pub async fn handle_pre_compact(&self, context: SessionContext) -> Result<CompactData> {
        let hook_ctx = HookContext::new(HookTrigger::PreCompact, context)
            .with_data(HookData::Compact {
                context_tokens: 100_000,
                max_tokens: 200_000,
                messages_to_remove: 10,
            });

        let hooks = self.hooks.read().map_err(|_| Error::Internal("Lock error".into()))?;
        let results = hooks.execute(hook_ctx).await?;

        // Extract compact data from results
        for result in results {
            if let HookResultData::CompactData(data) = result.data {
                return Ok(data);
            }
        }

        Ok(CompactData::new())
    }

    /// Register a custom hook handler.
    pub fn register_hook(&self, handler: Box<dyn HookHandler>) -> Result<()> {
        let mut hooks = self.hooks.write().map_err(|_| Error::Internal("Lock error".into()))?;
        hooks.register(handler);
        Ok(())
    }

    // =========================================================================
    // MCP Tool Operations
    // =========================================================================

    /// Execute an MCP tool by name.
    pub fn execute_tool(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value> {
        let tools = self.tools.read().map_err(|_| Error::Internal("Lock error".into()))?;
        tools.execute(name, input)
    }

    /// Get all available tools.
    pub fn available_tools(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap_or_else(|_| panic!("Lock poisoned"));
        tools.tools().iter().map(|t| t.name.clone()).collect()
    }

    /// Register a custom MCP tool.
    pub fn register_tool(
        &self,
        tool: super::mcp::McpTool,
        handler: ToolHandler,
    ) -> Result<()> {
        let mut tools = self.tools.write().map_err(|_| Error::Internal("Lock error".into()))?;
        tools.register(tool, handler);
        Ok(())
    }

    /// Export tools schema for MCP.
    pub fn export_tools_schema(&self) -> serde_json::Value {
        let tools = self.tools.read().unwrap_or_else(|_| panic!("Lock poisoned"));
        tools.export_schema()
    }

    // =========================================================================
    // Memory Operations
    // =========================================================================

    /// Query memory for relevant nodes.
    pub fn query_memory(&self, query: &NodeQuery) -> Result<Vec<Node>> {
        self.memory.query_nodes(query)
    }

    /// Search memory by text.
    pub fn search_memory(&self, text: &str, limit: usize) -> Result<Vec<Node>> {
        self.memory.search_content(text, limit)
    }

    /// Store a node in memory.
    pub fn store_memory(&self, node: &Node) -> Result<()> {
        self.memory.add_node(node)
    }

    /// Store a simple fact.
    pub fn store_fact(&self, content: &str, confidence: f64) -> Result<NodeId> {
        let node = Node::new(NodeType::Fact, content)
            .with_confidence(confidence)
            .with_tier(Tier::Session);
        self.memory.add_node(&node)?;
        Ok(node.id)
    }

    /// Promote nodes to a higher tier.
    pub fn promote_memories(&self, ids: &[NodeId], reason: &str) -> Result<Vec<NodeId>> {
        self.memory.promote(ids, reason)
    }

    // =========================================================================
    // Mode Management
    // =========================================================================

    /// Get current execution mode.
    pub fn current_mode(&self) -> ExecutionMode {
        *self.mode.read().unwrap_or_else(|_| panic!("Lock poisoned"))
    }

    /// Set execution mode.
    pub fn set_mode(&self, mode: ExecutionMode) {
        if let Ok(mut m) = self.mode.write() {
            *m = mode;
        }
        if let Ok(mut s) = self.status.write() {
            s.mode = mode;
            s.touch();
        }
    }

    // =========================================================================
    // Skill Operations
    // =========================================================================

    /// Find matching skills for a query.
    pub fn find_skills(&self, query: &str) -> Vec<&RlmSkill> {
        self.skills.find_matching(query)
    }

    /// Get a skill by name.
    pub fn get_skill(&self, name: &str) -> Option<&RlmSkill> {
        self.skills.get(name)
    }

    /// Export skills for discovery.
    pub fn export_skills(&self) -> String {
        self.skills.export_discovery()
    }

    // =========================================================================
    // Configuration
    // =========================================================================

    /// Get the current configuration.
    pub fn config(&self) -> &AdapterConfig {
        &self.config
    }

    /// Check if memory is persisted.
    pub fn is_memory_persisted(&self) -> bool {
        self.config.memory_path.is_some()
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn build_session_context(&self, request_ctx: &Option<RequestContext>) -> crate::context::SessionContext {
        let mut ctx = crate::context::SessionContext::new();

        if let Some(ref req_ctx) = request_ctx {
            // Add messages
            for msg in &req_ctx.messages {
                let role = match msg.role.as_str() {
                    "user" => crate::context::Role::User,
                    "assistant" => crate::context::Role::Assistant,
                    "system" => crate::context::Role::System,
                    _ => crate::context::Role::User,
                };
                ctx.add_message(crate::context::Message::new(role, &msg.content));
            }

            // Add files
            for (path, content) in &req_ctx.files {
                ctx.cache_file(path, content);
            }

            // Add tool outputs
            for output in &req_ctx.tool_outputs {
                ctx.add_tool_output(crate::context::ToolOutput::new(&output.tool_name, &output.content));
            }

            // Add working memory
            for (key, value) in &req_ctx.working_memory {
                ctx.set_memory(key, value.clone());
            }
        }

        ctx
    }
}

/// Guard to reset executing flag on drop.
struct ExecutionGuard {
    executing: Arc<RwLock<bool>>,
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        if let Ok(mut executing) = self.executing.write() {
            *executing = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        assert_eq!(adapter.current_mode(), ExecutionMode::Micro);
    }

    #[test]
    fn test_adapter_status() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        let status = adapter.status();

        assert_eq!(status.mode, ExecutionMode::Micro);
        assert!(!status.is_executing);
    }

    #[test]
    fn test_mode_change() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        adapter.set_mode(ExecutionMode::Thorough);
        assert_eq!(adapter.current_mode(), ExecutionMode::Thorough);
    }

    #[test]
    fn test_available_tools() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        let tools = adapter.available_tools();

        assert!(tools.contains(&"rlm_execute".to_string()));
        assert!(tools.contains(&"rlm_status".to_string()));
        assert!(tools.contains(&"memory_query".to_string()));
        assert!(tools.contains(&"memory_store".to_string()));
    }

    #[test]
    fn test_find_skills() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        let skills = adapter.find_skills("analyze the architecture");

        assert!(!skills.is_empty());
    }

    #[test]
    fn test_store_and_query_memory() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let id = adapter.store_fact("Test fact", 0.9).unwrap();

        let query = NodeQuery::new().node_types(vec![NodeType::Fact]);
        let results = adapter.query_memory(&query).unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().any(|n| n.id == id));
    }

    #[tokio::test]
    async fn test_execute_skip() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let request = RlmRequest::new("what is 2 + 2");
        let response = adapter.execute(request).await.unwrap();

        assert!(!response.activated);
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_execute_activate() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let request = RlmRequest::new("analyze the architecture and find all security issues")
            .force_activation();
        let response = adapter.execute(request).await.unwrap();

        assert!(response.activated);
        assert!(response.success);
        assert!(response.answer.is_some());
    }

    #[tokio::test]
    async fn test_handle_session_start() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let context = SessionContext::new("test-session")
            .with_project_root("/home/user/project");

        let result = adapter.handle_session_start(context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_handle_prompt_submit() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let context = SessionContext::new("test-session");
        let enhancement = adapter
            .handle_prompt_submit("analyze the architecture", context)
            .await
            .unwrap();

        assert!(enhancement.should_activate_rlm);
    }

    #[tokio::test]
    async fn test_handle_pre_compact() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let context = SessionContext::new("test-session");
        let data = adapter.handle_pre_compact(context).await.unwrap();

        assert!(data.work_summary.is_some());
    }

    #[test]
    fn test_export_tools_schema() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        let schema = adapter.export_tools_schema();

        assert!(schema.is_object());
        assert!(schema.get("tools").is_some());
    }

    #[test]
    fn test_export_skills() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        let skills = adapter.export_skills();

        assert!(skills.contains("# RLM Skills"));
        assert!(skills.contains("rlm_execute"));
    }
}
