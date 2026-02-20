//! LLM client abstraction with smart routing.
//!
//! This module provides a unified interface for working with multiple LLM providers
//! (Anthropic, OpenAI, OpenRouter) with intelligent routing based on query type,
//! recursion depth, and budget constraints.
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::llm::{
//!     AnthropicClient, ClientConfig, SmartRouter, RoutingContext,
//!     CompletionRequest, ChatMessage,
//! };
//!
//! // Create a client
//! let client = AnthropicClient::new(
//!     ClientConfig::new("your-api-key")
//!         .with_default_model("claude-3-5-sonnet-20241022")
//! );
//!
//! // Use smart routing
//! let router = SmartRouter::new();
//! let context = RoutingContext::new().with_depth(0);
//! let decision = router.route("Analyze the architecture", &context);
//!
//! // Make a request
//! let request = CompletionRequest::new()
//!     .with_model(&decision.model.id)
//!     .with_message(ChatMessage::user("Hello"));
//!
//! let response = client.complete(request).await?;
//! ```

mod batch;
mod cache;
mod client;
mod router;
mod types;

pub use batch::{
    BatchConfig, BatchExecutor, BatchQueryResult, BatchedLLMQuery, BatchedQueryResults,
    DEFAULT_MAX_PARALLEL,
};
pub use cache::{
    apply_cache_markers, find_cache_breakpoints, CacheEntry, CacheKey, CacheStats, PromptCache,
};
pub use client::{
    AnthropicClient, ClientConfig, LLMClient, MultiProviderClient, OpenAIClient, TrackedClient,
};
#[cfg(feature = "gemini")]
pub use client::GoogleClient;
pub use router::{
    DualModelConfig, QueryType, RoutingContext, RoutingDecision, SmartRouter, SwitchStrategy,
    TierDefaults,
};
pub use types::{
    CacheControl, ChatMessage, ChatRole, CompletionRequest, CompletionResponse, CostTracker,
    EmbeddingRequest, EmbeddingResponse, ModelCosts, ModelSpec, ModelTier, Provider, StopReason,
    ModelCallTier, TierBreakdown, TierCosts, TokenUsage,
};
