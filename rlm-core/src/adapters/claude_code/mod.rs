//! Claude Code plugin adapter for rlm-core.
//!
//! This module exposes rlm-core functionality via MCP tools and hooks
//! for integration with Claude Code plugins.
//!
//! ## Components
//!
//! - **Adapter**: Main entry point coordinating all functionality
//! - **MCP Tools**: Tool definitions for rlm_execute, rlm_status, memory_query, memory_store
//! - **Hooks**: Session lifecycle handlers (SessionStart, UserPromptSubmit, PreCompact)
//! - **Skills**: RLM exposed as discoverable skills
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::adapters::claude_code::{ClaudeCodeAdapter, AdapterConfig};
//!
//! let config = AdapterConfig::default();
//! let adapter = ClaudeCodeAdapter::new(config)?;
//!
//! // Handle session start
//! adapter.handle_session_start(session_context).await?;
//!
//! // Execute RLM
//! let response = adapter.execute(request).await?;
//! ```

mod adapter;
mod hooks;
mod mcp;
mod skills;
mod types;

pub use adapter::ClaudeCodeAdapter;
pub use hooks::{HookContext, HookHandler, HookResult, HookTrigger};
pub use mcp::{McpTool, McpToolRegistry};
pub use skills::RlmSkill;
pub use types::{
    AdapterConfig, AdapterStatus, CompactData, PromptEnhancement, RlmRequest, RlmResponse,
    SessionContext,
};
