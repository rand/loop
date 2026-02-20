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
use super::mcp::{
    McpToolRegistry, MemoryQueryInput, MemoryStoreInput, RlmExecuteInput, RlmStatusInput,
    ToolHandler,
};
use super::skills::{RlmSkill, SkillRegistry};
use super::types::{
    AdapterConfig, AdapterStatus, CompactData, MemoryStatus, PromptEnhancement, RequestContext,
    ResponseMetadata, RlmRequest, RlmResponse, SessionContext,
};
use crate::complexity::PatternClassifier;
use crate::context::ExternalizedContext;
use crate::error::{Error, Result};
use crate::llm::TokenUsage as LlmTokenUsage;
use crate::memory::{Node, NodeId, NodeQuery, NodeType, SqliteMemoryStore, Tier};
use crate::orchestrator::{ExecutionMode, OrchestrationRoutingRuntime};
use crate::repl::{ReplConfig, ReplHandle};
use crate::signature::{FieldSpec, FieldType, SubmitResult};
use crate::trajectory::{
    BudgetManager, CollectingEmitter, CostComponent, CostSummary,
    TokenUsage as TrajectoryTokenUsage, TrajectoryEmitter,
};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use std::time::Instant;

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
    /// Trajectory emitter (reserved for future event streaming)
    #[allow(dead_code)]
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

#[derive(Clone)]
struct AdapterRuntime {
    config: AdapterConfig,
    memory: Arc<SqliteMemoryStore>,
    budget: Arc<BudgetManager>,
    classifier: PatternClassifier,
    mode: Arc<RwLock<ExecutionMode>>,
    status: Arc<RwLock<AdapterStatus>>,
    executing: Arc<RwLock<bool>>,
}

impl AdapterRuntime {
    fn new(
        config: AdapterConfig,
        memory: Arc<SqliteMemoryStore>,
        budget: Arc<BudgetManager>,
        classifier: PatternClassifier,
        mode: Arc<RwLock<ExecutionMode>>,
        status: Arc<RwLock<AdapterStatus>>,
        executing: Arc<RwLock<bool>>,
    ) -> Self {
        Self {
            config,
            memory,
            budget,
            classifier,
            mode,
            status,
            executing,
        }
    }

    fn execute_request(&self, request: RlmRequest) -> Result<RlmResponse> {
        {
            let mut executing = self
                .executing
                .write()
                .map_err(|_| Error::Internal("Lock error".into()))?;
            if *executing {
                return Err(Error::Internal("RLM already executing".into()));
            }
            *executing = true;
        }

        let _guard = ExecutionGuard {
            executing: self.executing.clone(),
            status: self.status.clone(),
        };

        {
            let mut status = self
                .status
                .write()
                .map_err(|_| Error::Internal("Lock error".into()))?;
            status.is_executing = true;
            status.touch();
        }

        let mode = request
            .mode
            .unwrap_or_else(|| *self.mode.read().unwrap_or_else(|_| panic!("Lock poisoned")));
        let session_ctx = build_session_context(&request.context);
        let decision = self
            .classifier
            .should_activate(&request.query, &session_ctx);
        let should_activate = request.force_activation || decision.should_activate;

        if !should_activate {
            return Ok(
                RlmResponse::skip(decision.reason.clone(), mode).with_metadata(ResponseMetadata {
                    complexity_score: decision.score,
                    signals: decision
                        .signals
                        .active_signals()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ..Default::default()
                }),
            );
        }

        let final_mode = if self.config.auto_escalate {
            ExecutionMode::from_signals(&decision.signals)
        } else {
            mode
        };

        if let Some(max_budget) = request.max_budget_usd {
            if self.budget.state().current_cost_usd >= max_budget {
                return Err(Error::budget_exhausted("Request budget"));
            }
        }

        let start_time = Instant::now();
        let externalized = ExternalizedContext::from_session(&session_ctx, &request.query);
        let root_prompt = externalized.root_prompt();
        let memory_hits = self.memory.search_content(&request.query, 5)?;
        let answer = self.execute_repl_program(
            &request.query,
            final_mode,
            &root_prompt,
            &externalized,
            &session_ctx,
            &memory_hits,
        )?;

        let mut routing_runtime = OrchestrationRoutingRuntime::for_mode(final_mode);
        let (routing_decision, tier) = routing_runtime.route_recursive(&request.query, 0);
        let usage = LlmTokenUsage {
            input_tokens: estimate_tokens(&root_prompt)
                + estimate_tokens(&request.query)
                + (memory_hits.len() as u64 * 12),
            output_tokens: estimate_tokens(&answer),
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };
        let cost = routing_decision
            .model
            .calculate_cost(usage.input_tokens, usage.output_tokens);
        routing_runtime.record_usage(&routing_decision, &usage, Some(cost), tier);
        self.budget.record_cost(cost, usage.total());

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut cost_summary = CostSummary::new();
        cost_summary.add(CostComponent::Orchestration, trajectory_usage(&usage), cost);

        let metadata = ResponseMetadata {
            complexity_score: decision.score,
            signals: decision
                .signals
                .active_signals()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            duration_ms,
            max_depth_reached: 1,
            used_repl: true,
            memory_queries: 1,
            memory_stores: 0,
        };

        Ok(RlmResponse::success(answer, final_mode, cost_summary)
            .with_reason(decision.reason)
            .with_metadata(metadata))
    }

    fn execute_repl_program(
        &self,
        query: &str,
        final_mode: ExecutionMode,
        root_prompt: &str,
        externalized: &ExternalizedContext,
        session_ctx: &crate::context::SessionContext,
        memory_hits: &[Node],
    ) -> Result<String> {
        let mut repl = ReplHandle::spawn(local_repl_config())?;
        repl.register_signature(
            vec![FieldSpec::new("answer", FieldType::String)],
            Some("ClaudeCodeAdapterOutput"),
        )?;
        seed_context_variables(&mut repl, session_ctx)?;
        repl.set_variable("request_query", serde_json::json!(query))?;
        repl.set_variable("execution_mode", serde_json::json!(final_mode.to_string()))?;
        repl.set_variable("root_prompt", serde_json::json!(root_prompt))?;
        repl.set_variable(
            "context_overview",
            serde_json::json!(externalized
                .variables
                .values()
                .map(|var| format!("{} ({})", var.name, var.summary))
                .collect::<Vec<_>>()),
        )?;
        repl.set_variable(
            "memory_hits",
            serde_json::json!(memory_hits
                .iter()
                .map(|node| node.content.clone())
                .collect::<Vec<_>>()),
        )?;

        let exec = repl.execute(
            r#"
context_labels = context_overview if isinstance(context_overview, list) else []
hits = memory_hits if isinstance(memory_hits, list) else []
lines = [
    f"RLM mode: {execution_mode}",
    f"Query: {request_query}",
    f"Root prompt size: {len(root_prompt)} chars",
    f"Context variables: {', '.join(context_labels) if context_labels else 'none'}",
    f"Memory matches: {len(hits)}",
]
if hits:
    lines.append("Relevant memory:")
    for item in hits[:3]:
        if isinstance(item, str) and item.strip():
            lines.append(f"- {item[:160]}")
answer = "\n".join(lines)
SUBMIT({"answer": answer})
"#,
        )?;
        extract_answer(exec)
    }

    fn status(&self) -> AdapterStatus {
        let mut status = self
            .status
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| AdapterStatus::new(ExecutionMode::Micro, None));

        status.budget = self.budget.state();
        status.mode = *self.mode.read().unwrap_or_else(|_| panic!("Lock poisoned"));
        status.is_executing = *self
            .executing
            .read()
            .unwrap_or_else(|_| panic!("Lock poisoned"));

        if let Ok(stats) = self.memory.stats() {
            status.memory_stats = MemoryStatus {
                total_nodes: stats.total_nodes,
                nodes_by_tier: stats
                    .nodes_by_tier
                    .iter()
                    .map(|(k, v)| (k.to_string(), *v as u64))
                    .collect(),
                total_edges: stats.total_edges,
                is_persisted: self.config.memory_path.is_some(),
            };
        }

        status
    }

    fn execute_from_mcp(&self, input: RlmExecuteInput) -> Result<Value> {
        let mut request = RlmRequest::new(input.query);
        if let Some(mode) = input.mode {
            request.mode = Some(parse_execution_mode(&mode)?);
        }
        request.force_activation = input.force_activation.unwrap_or(false);
        request.max_budget_usd = input.max_budget_usd;
        let response = self.execute_request(request)?;
        serde_json::to_value(response).map_err(Error::from)
    }

    fn status_from_mcp(&self, _input: RlmStatusInput) -> Result<Value> {
        serde_json::to_value(self.status()).map_err(Error::from)
    }

    fn query_memory_from_mcp(&self, input: MemoryQueryInput) -> Result<Value> {
        let mut query = NodeQuery::new();

        if let Some(text) = input.text {
            query = query.text(text);
        }
        if let Some(node_types) = input.node_types {
            let parsed = node_types
                .iter()
                .map(|kind| parse_node_type(kind))
                .collect::<Result<Vec<_>>>()?;
            query = query.node_types(parsed);
        }
        if let Some(tiers) = input.tiers {
            let parsed = tiers
                .iter()
                .map(|tier| parse_tier(tier))
                .collect::<Result<Vec<_>>>()?;
            query = query.tiers(parsed);
        }
        if let Some(min_confidence) = input.min_confidence {
            query = query.min_confidence(min_confidence);
        }
        query = query.limit(input.limit.unwrap_or(10));

        let nodes = self.memory.query_nodes(&query)?;
        Ok(serde_json::json!({
            "nodes": nodes,
            "total_count": nodes.len()
        }))
    }

    fn store_memory_from_mcp(&self, input: MemoryStoreInput) -> Result<Value> {
        let mut node = Node::new(parse_node_type(&input.node_type)?, input.content);
        if let Some(subtype) = input.subtype {
            node = node.with_subtype(subtype);
        }
        if let Some(confidence) = input.confidence {
            node = node.with_confidence(confidence);
        }
        if let Some(tier) = input.tier {
            node = node.with_tier(parse_tier(&tier)?);
        }
        if let Some(metadata) = input.metadata {
            for (key, value) in metadata {
                node = node.with_metadata(key, value);
            }
        }

        let node_id = node.id.to_string();
        let tier = node.tier.to_string();
        let node_type = node.node_type.to_string();
        self.memory.add_node(&node)?;

        Ok(serde_json::json!({
            "success": true,
            "node_id": node_id,
            "tier": tier,
            "node_type": node_type
        }))
    }
}

impl ClaudeCodeAdapter {
    /// Create a new adapter with the given configuration.
    pub fn new(config: AdapterConfig) -> Result<Self> {
        let memory = if let Some(ref path) = config.memory_path {
            SqliteMemoryStore::open(path)?
        } else {
            SqliteMemoryStore::in_memory()?
        };
        let memory = Arc::new(memory);

        let budget = Arc::new(BudgetManager::new(config.budget.clone()));

        let mut emitter = CollectingEmitter::new();
        emitter.set_verbosity(config.verbosity);

        let mut hooks = HookRegistry::new();
        hooks.register(Box::new(SessionStartHandler::new()));
        hooks.register(Box::new(PromptAnalysisHandler::new()));
        hooks.register(Box::new(PreCompactHandler::new()));

        let skills = SkillRegistry::with_defaults();
        let classifier = PatternClassifier::with_threshold(config.escalation_threshold);
        let status = AdapterStatus::new(config.default_mode, config.session_id.clone());
        let mode = Arc::new(RwLock::new(status.mode));
        let status = Arc::new(RwLock::new(status));
        let executing = Arc::new(RwLock::new(false));

        let runtime = AdapterRuntime::new(
            config.clone(),
            memory.clone(),
            budget.clone(),
            classifier.clone(),
            mode.clone(),
            status.clone(),
            executing.clone(),
        );
        let mut tools = McpToolRegistry::with_defaults();
        Self::bind_live_mcp_handlers(&mut tools, runtime)?;

        Ok(Self {
            config,
            memory,
            budget,
            emitter: Arc::new(RwLock::new(emitter)),
            hooks: Arc::new(RwLock::new(hooks)),
            tools: Arc::new(RwLock::new(tools)),
            skills: Arc::new(skills),
            classifier,
            mode,
            status,
            executing,
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
        self.runtime().execute_request(request)
    }

    /// Get current adapter status.
    pub fn status(&self) -> AdapterStatus {
        self.runtime().status()
    }

    // =========================================================================
    // Hook Handling
    // =========================================================================

    /// Handle session start event.
    pub async fn handle_session_start(&self, context: SessionContext) -> Result<HookResult> {
        let hook_ctx = HookContext::new(HookTrigger::SessionStart, context);

        let hooks = self
            .hooks
            .read()
            .map_err(|_| Error::Internal("Lock error".into()))?;
        let results = hooks.execute(hook_ctx).await?;

        // Return first result or default
        Ok(results.into_iter().next().unwrap_or_else(HookResult::ok))
    }

    /// Handle user prompt submission.
    pub async fn handle_prompt_submit(
        &self,
        prompt: &str,
        context: SessionContext,
    ) -> Result<PromptEnhancement> {
        let hook_ctx = HookContext::new(HookTrigger::UserPromptSubmit, context).with_data(
            HookData::PromptSubmit {
                prompt: prompt.to_string(),
                recent_messages: Vec::new(),
            },
        );

        let hooks = self
            .hooks
            .read()
            .map_err(|_| Error::Internal("Lock error".into()))?;
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
        let hook_ctx =
            HookContext::new(HookTrigger::PreCompact, context).with_data(HookData::Compact {
                context_tokens: 100_000,
                max_tokens: 200_000,
                messages_to_remove: 10,
            });

        let hooks = self
            .hooks
            .read()
            .map_err(|_| Error::Internal("Lock error".into()))?;
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
        let mut hooks = self
            .hooks
            .write()
            .map_err(|_| Error::Internal("Lock error".into()))?;
        hooks.register(handler);
        Ok(())
    }

    // =========================================================================
    // MCP Tool Operations
    // =========================================================================

    /// Execute an MCP tool by name.
    pub fn execute_tool(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value> {
        let tools = self
            .tools
            .read()
            .map_err(|_| Error::Internal("Lock error".into()))?;
        tools.execute(name, input)
    }

    /// Get all available tools.
    pub fn available_tools(&self) -> Vec<String> {
        let tools = self
            .tools
            .read()
            .unwrap_or_else(|_| panic!("Lock poisoned"));
        tools.tools().iter().map(|t| t.name.clone()).collect()
    }

    /// Register a custom MCP tool.
    pub fn register_tool(&self, tool: super::mcp::McpTool, handler: ToolHandler) -> Result<()> {
        let mut tools = self
            .tools
            .write()
            .map_err(|_| Error::Internal("Lock error".into()))?;
        tools.register(tool, handler);
        Ok(())
    }

    /// Export tools schema for MCP.
    pub fn export_tools_schema(&self) -> serde_json::Value {
        let tools = self
            .tools
            .read()
            .unwrap_or_else(|_| panic!("Lock poisoned"));
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

    fn runtime(&self) -> AdapterRuntime {
        AdapterRuntime::new(
            self.config.clone(),
            self.memory.clone(),
            self.budget.clone(),
            self.classifier.clone(),
            self.mode.clone(),
            self.status.clone(),
            self.executing.clone(),
        )
    }

    fn bind_live_mcp_handlers(tools: &mut McpToolRegistry, runtime: AdapterRuntime) -> Result<()> {
        let execute_runtime = runtime.clone();
        tools.set_handler(
            "rlm_execute",
            Arc::new(move |input| {
                let request: RlmExecuteInput = serde_json::from_value(input)?;
                execute_runtime.execute_from_mcp(request)
            }),
        )?;

        let status_runtime = runtime.clone();
        tools.set_handler(
            "rlm_status",
            Arc::new(move |input| {
                let request: RlmStatusInput = serde_json::from_value(input)?;
                status_runtime.status_from_mcp(request)
            }),
        )?;

        let query_runtime = runtime.clone();
        tools.set_handler(
            "memory_query",
            Arc::new(move |input| {
                let request: MemoryQueryInput = serde_json::from_value(input)?;
                query_runtime.query_memory_from_mcp(request)
            }),
        )?;

        tools.set_handler(
            "memory_store",
            Arc::new(move |input| {
                let request: MemoryStoreInput = serde_json::from_value(input)?;
                runtime.store_memory_from_mcp(request)
            }),
        )?;

        Ok(())
    }
}

/// Guard to reset executing flag on drop.
struct ExecutionGuard {
    executing: Arc<RwLock<bool>>,
    status: Arc<RwLock<AdapterStatus>>,
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        if let Ok(mut executing) = self.executing.write() {
            *executing = false;
        }
        if let Ok(mut status) = self.status.write() {
            status.is_executing = false;
            status.touch();
        }
    }
}

fn build_session_context(request_ctx: &Option<RequestContext>) -> crate::context::SessionContext {
    let mut ctx = crate::context::SessionContext::new();

    if let Some(req_ctx) = request_ctx {
        for msg in &req_ctx.messages {
            let role = match msg.role.as_str() {
                "assistant" => crate::context::Role::Assistant,
                "system" => crate::context::Role::System,
                _ => crate::context::Role::User,
            };
            ctx.add_message(crate::context::Message::new(role, &msg.content));
        }

        for (path, content) in &req_ctx.files {
            ctx.cache_file(path, content);
        }

        for output in &req_ctx.tool_outputs {
            ctx.add_tool_output(crate::context::ToolOutput::new(
                &output.tool_name,
                &output.content,
            ));
        }

        for (key, value) in &req_ctx.working_memory {
            ctx.set_memory(key, value.clone());
        }
    }

    ctx
}

fn seed_context_variables(
    repl: &mut ReplHandle,
    session_ctx: &crate::context::SessionContext,
) -> Result<()> {
    if !session_ctx.messages.is_empty() {
        let messages: Vec<Value> = session_ctx
            .messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role.to_string(),
                    "content": msg.content,
                })
            })
            .collect();
        repl.set_variable("conversation", Value::Array(messages))?;
    }

    if !session_ctx.files.is_empty() {
        repl.set_variable("files", serde_json::json!(session_ctx.files))?;
    }

    if !session_ctx.tool_outputs.is_empty() {
        let outputs: Vec<Value> = session_ctx
            .tool_outputs
            .iter()
            .map(|output| {
                serde_json::json!({
                    "tool_name": output.tool_name,
                    "content": output.content,
                    "exit_code": output.exit_code,
                })
            })
            .collect();
        repl.set_variable("tool_outputs", Value::Array(outputs))?;
    }

    if !session_ctx.working_memory.is_empty() {
        repl.set_variable(
            "working_memory",
            serde_json::json!(session_ctx.working_memory),
        )?;
    }

    Ok(())
}

fn local_repl_config() -> ReplConfig {
    let mut config = ReplConfig::default();
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    let local_python3 = manifest_dir.join("python/.venv/bin/python3");
    let local_python = manifest_dir.join("python/.venv/bin/python");
    if local_python3.exists() {
        config.python_path = local_python3.to_string_lossy().into_owned();
    } else if local_python.exists() {
        config.python_path = local_python.to_string_lossy().into_owned();
    }

    let local_package = manifest_dir.join("python");
    if local_package.exists() {
        config.repl_package_path = Some(local_package.to_string_lossy().into_owned());
    }

    config
}

fn extract_answer(exec: crate::repl::ExecuteResult) -> Result<String> {
    match exec.submit_result {
        Some(SubmitResult::Success { outputs, .. }) => outputs
            .get("answer")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .ok_or_else(|| Error::repl_execution("SUBMIT succeeded without string `answer` field")),
        Some(SubmitResult::ValidationError { errors, .. }) => {
            let joined = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            Err(Error::repl_execution(format!(
                "SUBMIT validation failed: {}",
                joined
            )))
        }
        Some(SubmitResult::NotSubmitted { reason }) => Err(Error::repl_execution(format!(
            "SUBMIT not called: {}",
            reason
        ))),
        None => {
            if exec.success {
                Err(Error::repl_execution(
                    "Execution completed without a SUBMIT result",
                ))
            } else {
                Err(Error::repl_execution(
                    exec.error
                        .unwrap_or_else(|| "REPL execution failed".to_string()),
                ))
            }
        }
    }
}

fn estimate_tokens(text: &str) -> u64 {
    ((text.chars().count() as u64).saturating_add(3) / 4).max(1)
}

fn trajectory_usage(usage: &LlmTokenUsage) -> TrajectoryTokenUsage {
    TrajectoryTokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_creation_tokens: usage.cache_creation_tokens.unwrap_or(0),
        cache_read_tokens: usage.cache_read_tokens.unwrap_or(0),
    }
}

fn parse_execution_mode(raw: &str) -> Result<ExecutionMode> {
    match raw.to_ascii_lowercase().as_str() {
        "micro" => Ok(ExecutionMode::Micro),
        "fast" => Ok(ExecutionMode::Fast),
        "balanced" => Ok(ExecutionMode::Balanced),
        "thorough" => Ok(ExecutionMode::Thorough),
        _ => Err(Error::Config(format!(
            "Unsupported execution mode: {}",
            raw
        ))),
    }
}

fn parse_node_type(raw: &str) -> Result<NodeType> {
    match raw.to_ascii_lowercase().as_str() {
        "entity" => Ok(NodeType::Entity),
        "fact" => Ok(NodeType::Fact),
        "experience" => Ok(NodeType::Experience),
        "decision" => Ok(NodeType::Decision),
        "snippet" => Ok(NodeType::Snippet),
        _ => Err(Error::Config(format!("Unsupported node_type: {}", raw))),
    }
}

fn parse_tier(raw: &str) -> Result<Tier> {
    match raw.to_ascii_lowercase().as_str() {
        "task" => Ok(Tier::Task),
        "session" => Ok(Tier::Session),
        "longterm" | "long_term" => Ok(Tier::LongTerm),
        "archive" => Ok(Tier::Archive),
        _ => Err(Error::Config(format!("Unsupported tier: {}", raw))),
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
        assert!(tools.contains(&"trace_visualize".to_string()));
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
        assert!(response.metadata.used_repl);
        assert!(response.cost.total_cost_usd > 0.0);
        assert!(!response
            .answer
            .as_deref()
            .unwrap_or_default()
            .contains("[Placeholder"));
    }

    #[tokio::test]
    async fn test_execute_e2e_incident_triage_ooda_flow() {
        let config =
            AdapterConfig::default().with_budget(crate::trajectory::BudgetConfig::unlimited());
        let adapter = ClaudeCodeAdapter::new(config).unwrap();

        adapter
            .store_fact(
                "perform thorough architecture security analysis across auth files explain why triagebeacon login failures increased include concrete signals",
                0.92,
            )
            .unwrap();
        adapter
            .store_fact(
                "Session middleware retries token refresh twice before failing",
                0.88,
            )
            .unwrap();

        let context = RequestContext::new()
            .with_message(
                "user",
                "Production login failures started after key rotation",
            )
            .with_message(
                "assistant",
                "I will inspect auth middleware, session handling, and test failures.",
            )
            .with_file(
                "src/auth/middleware.rs",
                "fn validate_token(token: &str) -> Result<UserId> { /* ... */ }",
            )
            .with_file(
                "src/session/store.rs",
                "pub fn refresh_session(user_id: UserId) -> Result<()> { /* ... */ }",
            )
            .with_tool_output(
                "pytest -q tests/auth/test_login_flow.py",
                "FAILED test_login_flow::test_rotation_handles_expiry",
                Some(1),
            )
            .with_memory("incident_id", serde_json::json!("INC-2048"))
            .with_memory("focus", serde_json::json!("auth"));

        let request = RlmRequest::new(
            "Perform a thorough architecture and security analysis across auth files, explain why triagebeacon login failures increased, and include concrete signals.",
        )
        .with_mode(ExecutionMode::Fast)
        .with_context(context);

        let response = adapter.execute(request).await.unwrap();
        let answer = response
            .answer
            .as_deref()
            .expect("activated path must produce an answer");

        // Observe: context and memory become explicit execution inputs.
        assert!(answer.contains("Context variables:"));
        assert!(answer.contains("Memory matches:"));

        // Orient + Decide: complexity signals and auto-escalation are explicit.
        assert!(response.activated);
        assert!(response.success);
        assert_eq!(response.mode, ExecutionMode::Thorough);
        assert!(response.metadata.complexity_score >= 3);
        assert!(response
            .metadata
            .signals
            .iter()
            .any(|signal| signal == "architecture_analysis"));

        // Act: execution produces REPL output + accounting metadata.
        assert!(answer.contains("RLM mode: thorough"));
        assert!(response.metadata.used_repl);
        assert_eq!(response.metadata.memory_queries, 1);
        assert!(response.cost.total_cost_usd > 0.0);
    }

    #[tokio::test]
    async fn test_execute_e2e_fast_path_skip_with_context_noise() {
        let config =
            AdapterConfig::default().with_budget(crate::trajectory::BudgetConfig::unlimited());
        let adapter = ClaudeCodeAdapter::new(config).unwrap();

        let context = RequestContext::new()
            .with_message("user", "Need a quick status ping.")
            .with_file("src/auth/middleware.rs", "auth middleware snapshot")
            .with_file("src/session/store.rs", "session store snapshot")
            .with_tool_output("git status", "working tree clean", Some(0))
            .with_memory("ticket", serde_json::json!("OPS-11"));

        let request = RlmRequest::new("quick status update only").with_context(context);
        let response = adapter.execute(request).await.unwrap();

        assert!(!response.activated);
        assert!(response.success);
        assert_eq!(response.activation_reason, "simple_task");
        assert!(response.answer.is_none());
        assert!(response
            .metadata
            .signals
            .iter()
            .any(|signal| signal == "user_fast"));
        assert_eq!(response.cost.total_cost_usd, 0.0);
    }

    #[tokio::test]
    async fn test_execute_activate_budget_failure() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let request = RlmRequest::new("analyze the architecture")
            .force_activation()
            .with_budget(0.0);
        let err = adapter
            .execute(request)
            .await
            .expect_err("expected budget failure");
        assert!(err.to_string().contains("Budget exhausted"));
    }

    #[test]
    fn test_execute_tool_memory_handlers_live() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let store = adapter
            .execute_tool(
                "memory_store",
                serde_json::json!({
                    "content": "JWT auth is enabled",
                    "node_type": "fact",
                    "tier": "session",
                    "confidence": 0.9
                }),
            )
            .expect("memory_store should succeed");
        assert_eq!(store.get("success").and_then(Value::as_bool), Some(true));

        let query = adapter
            .execute_tool(
                "memory_query",
                serde_json::json!({
                    "text": "JWT auth",
                    "node_types": ["fact"],
                    "limit": 5
                }),
            )
            .expect("memory_query should succeed");
        assert!(
            query
                .get("total_count")
                .and_then(Value::as_u64)
                .unwrap_or_default()
                >= 1
        );
    }

    #[test]
    fn test_execute_tool_rlm_status_live() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();
        adapter.store_fact("status fact", 0.8).unwrap();

        let status = adapter
            .execute_tool("rlm_status", serde_json::json!({}))
            .expect("rlm_status should succeed");
        assert_eq!(status.get("mode").and_then(Value::as_str), Some("micro"));
        assert!(
            status
                .get("memory_stats")
                .and_then(|v| v.get("total_nodes"))
                .and_then(Value::as_u64)
                .unwrap_or_default()
                >= 1
        );
    }

    #[tokio::test]
    async fn test_handle_session_start() {
        let adapter = ClaudeCodeAdapter::testing().unwrap();

        let context = SessionContext::new("test-session").with_project_root("/home/user/project");

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
