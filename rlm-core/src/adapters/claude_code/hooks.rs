//! Hook handlers for Claude Code lifecycle events.
//!
//! Hooks allow rlm-core to respond to Claude Code session events:
//!
//! - **SessionStart**: Initialize memory, load context
//! - **UserPromptSubmit**: Analyze prompt, suggest enhancements
//! - **PreCompact**: Extract important data before context compaction
//! - **PreToolUse**: Validate or modify tool calls
//! - **PostToolUse**: Process tool results

use super::types::{CompactData, PromptEnhancement, SessionContext};
use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// When a hook should be triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    /// When a Claude Code session starts
    SessionStart,
    /// When the user submits a prompt
    UserPromptSubmit,
    /// Before context window compaction
    PreCompact,
    /// Before a tool is executed
    PreToolUse,
    /// After a tool is executed
    PostToolUse,
    /// When a session ends
    SessionEnd,
}

impl std::fmt::Display for HookTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionStart => write!(f, "session_start"),
            Self::UserPromptSubmit => write!(f, "user_prompt_submit"),
            Self::PreCompact => write!(f, "pre_compact"),
            Self::PreToolUse => write!(f, "pre_tool_use"),
            Self::PostToolUse => write!(f, "post_tool_use"),
            Self::SessionEnd => write!(f, "session_end"),
        }
    }
}

/// Context provided to hook handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// The trigger that invoked this hook
    pub trigger: HookTrigger,
    /// Session context
    pub session: SessionContext,
    /// Hook-specific data
    pub data: HookData,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl HookContext {
    /// Create a new hook context.
    pub fn new(trigger: HookTrigger, session: SessionContext) -> Self {
        Self {
            trigger,
            session,
            data: HookData::None,
            metadata: HashMap::new(),
        }
    }

    /// Set the hook data.
    pub fn with_data(mut self, data: HookData) -> Self {
        self.data = data;
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Hook-specific data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookData {
    /// No additional data
    None,
    /// Data for UserPromptSubmit hook
    PromptSubmit {
        /// The user's prompt
        prompt: String,
        /// Recent conversation history
        recent_messages: Vec<String>,
    },
    /// Data for PreToolUse hook
    ToolUse {
        /// Tool name
        tool_name: String,
        /// Tool arguments
        arguments: Value,
    },
    /// Data for PostToolUse hook
    ToolResult {
        /// Tool name
        tool_name: String,
        /// Tool output
        output: String,
        /// Exit code (if applicable)
        exit_code: Option<i32>,
        /// Whether the tool succeeded
        success: bool,
    },
    /// Data for PreCompact hook
    Compact {
        /// Current context size in tokens (approximate)
        context_tokens: u64,
        /// Maximum context size
        max_tokens: u64,
        /// Messages that will be removed
        messages_to_remove: usize,
    },
}

/// Result from a hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Whether the hook executed successfully
    pub success: bool,
    /// Human-readable message
    pub message: Option<String>,
    /// Additional context to inject (depends on hook type)
    pub additional_context: Option<String>,
    /// Hook-specific result data
    pub data: HookResultData,
    /// Whether to abort the operation (only for Pre* hooks)
    pub abort: bool,
    /// Abort reason (if abort is true)
    pub abort_reason: Option<String>,
}

impl HookResult {
    /// Create a successful result with no additional action.
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
            additional_context: None,
            data: HookResultData::None,
            abort: false,
            abort_reason: None,
        }
    }

    /// Create a successful result with a message.
    pub fn ok_with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            additional_context: None,
            data: HookResultData::None,
            abort: false,
            abort_reason: None,
        }
    }

    /// Create a failed result.
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            additional_context: None,
            data: HookResultData::None,
            abort: false,
            abort_reason: None,
        }
    }

    /// Create an abort result (for Pre* hooks).
    pub fn abort(reason: impl Into<String>) -> Self {
        Self {
            success: true,
            message: None,
            additional_context: None,
            data: HookResultData::None,
            abort: true,
            abort_reason: Some(reason.into()),
        }
    }

    /// Set additional context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }

    /// Set result data.
    pub fn with_data(mut self, data: HookResultData) -> Self {
        self.data = data;
        self
    }
}

/// Hook-specific result data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookResultData {
    /// No additional data
    None,
    /// Result from UserPromptSubmit hook
    PromptEnhancement(PromptEnhancement),
    /// Result from PreCompact hook
    CompactData(CompactData),
    /// Modified tool arguments (from PreToolUse)
    ModifiedToolArgs(Value),
}

/// Trait for implementing hook handlers.
#[async_trait]
pub trait HookHandler: Send + Sync {
    /// Get the name of this hook handler.
    fn name(&self) -> &str;

    /// Get the trigger this handler responds to.
    fn trigger(&self) -> HookTrigger;

    /// Get the priority (lower = higher priority, executed first).
    fn priority(&self) -> i32 {
        0
    }

    /// Execute the hook handler.
    async fn execute(&self, context: HookContext) -> Result<HookResult>;
}

/// Registry of hook handlers.
pub struct HookRegistry {
    handlers: Vec<Box<dyn HookHandler>>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a hook handler.
    pub fn register(&mut self, handler: Box<dyn HookHandler>) {
        self.handlers.push(handler);
        // Sort by priority (lower priority number = higher priority)
        self.handlers.sort_by_key(|h| h.priority());
    }

    /// Get all handlers for a specific trigger.
    pub fn handlers_for(&self, trigger: HookTrigger) -> Vec<&dyn HookHandler> {
        self.handlers
            .iter()
            .filter(|h| h.trigger() == trigger)
            .map(|h| h.as_ref())
            .collect()
    }

    /// Execute all handlers for a trigger.
    pub async fn execute(&self, context: HookContext) -> Result<Vec<HookResult>> {
        let handlers = self.handlers_for(context.trigger);
        let mut results = Vec::with_capacity(handlers.len());

        for handler in handlers {
            let result = handler.execute(context.clone()).await?;

            // If any handler requests abort, stop processing
            if result.abort {
                results.push(result);
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Check if any handlers are registered for a trigger.
    #[allow(dead_code)] // Public API for external consumers
    pub fn has_handlers(&self, trigger: HookTrigger) -> bool {
        self.handlers.iter().any(|h| h.trigger() == trigger)
    }

    /// Get count of registered handlers.
    #[allow(dead_code)] // Public API for external consumers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

// =============================================================================
// Built-in Hook Handlers
// =============================================================================

/// Handler for session start - initializes memory and loads context.
pub struct SessionStartHandler {
    name: String,
}

impl SessionStartHandler {
    pub fn new() -> Self {
        Self {
            name: "session_start_handler".to_string(),
        }
    }
}

impl Default for SessionStartHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HookHandler for SessionStartHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn trigger(&self) -> HookTrigger {
        HookTrigger::SessionStart
    }

    async fn execute(&self, context: HookContext) -> Result<HookResult> {
        let mut result = HookResult::ok_with_message("Session initialized");

        // Add project context if available
        if let Some(ref project_root) = context.session.project_root {
            let context_msg = format!(
                "Project root: {}\nSession ID: {}",
                project_root, context.session.session_id
            );
            result = result.with_context(context_msg);
        }

        Ok(result)
    }
}

/// Handler for user prompt submission - analyzes prompt for complexity.
pub struct PromptAnalysisHandler {
    name: String,
}

impl PromptAnalysisHandler {
    pub fn new() -> Self {
        Self {
            name: "prompt_analysis_handler".to_string(),
        }
    }
}

impl Default for PromptAnalysisHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HookHandler for PromptAnalysisHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn trigger(&self) -> HookTrigger {
        HookTrigger::UserPromptSubmit
    }

    async fn execute(&self, context: HookContext) -> Result<HookResult> {
        if let HookData::PromptSubmit { prompt, .. } = &context.data {
            // Use the complexity classifier to analyze the prompt
            let classifier = crate::complexity::PatternClassifier::new();
            let session_ctx = crate::context::SessionContext::new();
            let decision = classifier.should_activate(prompt, &session_ctx);

            let enhancement = PromptEnhancement::none()
                .with_activation(decision.should_activate)
                .with_mode(crate::orchestrator::ExecutionMode::from_signals(&decision.signals));

            Ok(HookResult::ok()
                .with_data(HookResultData::PromptEnhancement(enhancement)))
        } else {
            Ok(HookResult::ok())
        }
    }
}

/// Handler for pre-compact - extracts important data before compaction.
pub struct PreCompactHandler {
    name: String,
}

impl PreCompactHandler {
    pub fn new() -> Self {
        Self {
            name: "pre_compact_handler".to_string(),
        }
    }
}

impl Default for PreCompactHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HookHandler for PreCompactHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn trigger(&self) -> HookTrigger {
        HookTrigger::PreCompact
    }

    async fn execute(&self, context: HookContext) -> Result<HookResult> {
        let compact_data = CompactData::new()
            .with_summary(format!(
                "Session {} context compacted",
                context.session.session_id
            ));

        Ok(HookResult::ok()
            .with_data(HookResultData::CompactData(compact_data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_trigger_display() {
        assert_eq!(HookTrigger::SessionStart.to_string(), "session_start");
        assert_eq!(HookTrigger::UserPromptSubmit.to_string(), "user_prompt_submit");
        assert_eq!(HookTrigger::PreCompact.to_string(), "pre_compact");
    }

    #[test]
    fn test_hook_context_builder() {
        let session = SessionContext::new("test");
        let context = HookContext::new(HookTrigger::SessionStart, session)
            .with_data(HookData::None)
            .with_metadata("key", "value");

        assert_eq!(context.trigger, HookTrigger::SessionStart);
        assert!(context.metadata.contains_key("key"));
    }

    #[test]
    fn test_hook_result_ok() {
        let result = HookResult::ok();
        assert!(result.success);
        assert!(!result.abort);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_hook_result_abort() {
        let result = HookResult::abort("Security concern");
        assert!(result.success);
        assert!(result.abort);
        assert_eq!(result.abort_reason, Some("Security concern".to_string()));
    }

    #[test]
    fn test_hook_result_fail() {
        let result = HookResult::fail("Something went wrong");
        assert!(!result.success);
        assert!(!result.abort);
        assert_eq!(result.message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_hook_registry() {
        let mut registry = HookRegistry::new();
        registry.register(Box::new(SessionStartHandler::new()));
        registry.register(Box::new(PromptAnalysisHandler::new()));

        assert_eq!(registry.handler_count(), 2);
        assert!(registry.has_handlers(HookTrigger::SessionStart));
        assert!(registry.has_handlers(HookTrigger::UserPromptSubmit));
        assert!(!registry.has_handlers(HookTrigger::PostToolUse));
    }

    #[tokio::test]
    async fn test_session_start_handler() {
        let handler = SessionStartHandler::new();
        let session = SessionContext::new("test")
            .with_project_root("/home/user/project");
        let context = HookContext::new(HookTrigger::SessionStart, session);

        let result = handler.execute(context).await.unwrap();
        assert!(result.success);
        assert!(result.additional_context.is_some());
    }

    #[tokio::test]
    async fn test_prompt_analysis_handler() {
        let handler = PromptAnalysisHandler::new();
        let session = SessionContext::new("test");
        let context = HookContext::new(HookTrigger::UserPromptSubmit, session)
            .with_data(HookData::PromptSubmit {
                prompt: "Analyze the architecture and find all security issues".to_string(),
                recent_messages: vec![],
            });

        let result = handler.execute(context).await.unwrap();
        assert!(result.success);

        if let HookResultData::PromptEnhancement(enhancement) = result.data {
            assert!(enhancement.should_activate_rlm);
        } else {
            panic!("Expected PromptEnhancement data");
        }
    }
}
