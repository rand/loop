//! Session context types for RLM orchestration.
//!
//! The context module provides the core types for representing conversation state,
//! messages, and tool outputs that flow through the RLM orchestration loop.
//!
//! # Context Externalization
//!
//! To prevent "context rot" (performance degradation from lengthy prompts),
//! the module supports externalizing context as REPL variables:
//!
//! ```rust,ignore
//! use rlm_core::context::{SessionContext, ExternalizedContext};
//!
//! let ctx = SessionContext::new();
//! ctx.add_user_message("Analyze the auth system");
//! ctx.cache_file("/src/auth.rs", "fn authenticate() { ... }");
//!
//! // Externalize: full context becomes REPL variables
//! let externalized = ExternalizedContext::from_session(&ctx, "Analyze the auth system");
//!
//! // Root prompt only includes query + variable summaries
//! let prompt = externalized.root_prompt();
//! // -> "Query: Analyze the auth system\n\nAvailable context variables:\n- conversation: 1 messages...\n- files: 1 files..."
//!
//! // Full context accessible via REPL helpers
//! // peek(conversation, 0, 5)  -> first 5 messages
//! // search(files, "auth")     -> files matching pattern
//! ```

mod externalize;
mod types;

pub use externalize::{
    ContextSizeTracker, ContextVarType, ContextVariable, ExternalizationConfig,
    ExternalizedContext, SizeConfig, SizeWarning, VariableAccessHelper,
};
pub use types::{Message, Role, SessionContext, ToolOutput};
