//! Deployment adapters for rlm-core.
//!
//! This module provides adapters for different deployment contexts:
//!
//! - **CLI**: Trace visualization command surface for binary wrappers
//! - **Claude Code**: MCP tools and hooks for Claude Code plugin integration
//! - **TUI**: Bubble Tea TUI integration for terminal user interfaces (Go)
//!
//! Each adapter translates the core rlm-core functionality into the
//! interface expected by its deployment target.

pub mod cli;
pub mod claude_code;
pub mod tui;

// Re-export primary types for convenience
pub use cli::{
    suggested_output_path, trace_visualize, trace_visualize_from_json, HtmlPreset,
    TraceVisualizeFormat, TraceVisualizeOptions, TraceVisualizeResult,
};

pub use claude_code::{
    AdapterConfig, AdapterStatus, ClaudeCodeAdapter, CompactData, HookContext, HookHandler,
    HookResult, HookTrigger, McpTool, McpToolRegistry, PromptEnhancement, RlmRequest, RlmResponse,
    RlmSkill, SessionContext as AdapterSessionContext,
};

pub use tui::{
    BudgetPanelData, BudgetUpdate, EventBridge, EventStyle, ExecutionStatus, MemoryNodeView,
    MemoryPanelData, ReplEntry, ReplPanelData, ReplStatus, TierCounts, TUIAdapter, TUIConfig,
    TUIEvent, TracePanelData, TraceEventView, StatusUpdate,
};
