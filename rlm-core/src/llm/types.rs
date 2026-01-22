//! LLM types for requests, responses, and model definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Provider {
    Anthropic,
    OpenAI,
    OpenRouter,
    #[cfg(feature = "gemini")]
    Google,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::OpenAI => write!(f, "openai"),
            Self::OpenRouter => write!(f, "openrouter"),
            #[cfg(feature = "gemini")]
            Self::Google => write!(f, "google"),
        }
    }
}

/// Model tier for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// Most capable, highest cost (e.g., Claude Opus, GPT-4)
    Flagship = 0,
    /// Balanced capability and cost (e.g., Claude Sonnet, GPT-4o)
    Balanced = 1,
    /// Fast and cheap (e.g., Claude Haiku, GPT-4o-mini)
    Fast = 2,
}

/// Model definition with pricing and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Model identifier (e.g., "claude-3-5-sonnet-20241022")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Provider
    pub provider: Provider,
    /// Tier classification
    pub tier: ModelTier,
    /// Maximum context window (tokens)
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output: u32,
    /// Input cost per million tokens (USD)
    pub input_cost_per_m: f64,
    /// Output cost per million tokens (USD)
    pub output_cost_per_m: f64,
    /// Supports prompt caching
    pub supports_caching: bool,
    /// Supports vision/images
    pub supports_vision: bool,
    /// Supports tool use
    pub supports_tools: bool,
}

impl ModelSpec {
    /// Calculate cost for given token usage.
    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_m;
        input_cost + output_cost
    }
}

/// Well-known models.
impl ModelSpec {
    pub fn claude_opus() -> Self {
        Self {
            id: "claude-3-opus-20240229".to_string(),
            name: "Claude 3 Opus".to_string(),
            provider: Provider::Anthropic,
            tier: ModelTier::Flagship,
            context_window: 200_000,
            max_output: 4096,
            input_cost_per_m: 15.0,
            output_cost_per_m: 75.0,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }

    pub fn claude_sonnet() -> Self {
        Self {
            id: "claude-3-5-sonnet-20241022".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            provider: Provider::Anthropic,
            tier: ModelTier::Balanced,
            context_window: 200_000,
            max_output: 8192,
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }

    pub fn claude_haiku() -> Self {
        Self {
            id: "claude-3-5-haiku-20241022".to_string(),
            name: "Claude 3.5 Haiku".to_string(),
            provider: Provider::Anthropic,
            tier: ModelTier::Fast,
            context_window: 200_000,
            max_output: 8192,
            input_cost_per_m: 0.8,
            output_cost_per_m: 4.0,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }

    pub fn gpt4o() -> Self {
        Self {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: Provider::OpenAI,
            tier: ModelTier::Balanced,
            context_window: 128_000,
            max_output: 16384,
            input_cost_per_m: 2.5,
            output_cost_per_m: 10.0,
            supports_caching: false,
            supports_vision: true,
            supports_tools: true,
        }
    }

    pub fn gpt4o_mini() -> Self {
        Self {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o Mini".to_string(),
            provider: Provider::OpenAI,
            tier: ModelTier::Fast,
            context_window: 128_000,
            max_output: 16384,
            input_cost_per_m: 0.15,
            output_cost_per_m: 0.60,
            supports_caching: false,
            supports_vision: true,
            supports_tools: true,
        }
    }

    // Google/Gemini models (requires "gemini" feature)

    #[cfg(feature = "gemini")]
    pub fn gemini_2_0_flash() -> Self {
        Self {
            id: "gemini-2.0-flash".to_string(),
            name: "Gemini 2.0 Flash".to_string(),
            provider: Provider::Google,
            tier: ModelTier::Fast,
            context_window: 1_000_000,
            max_output: 8192,
            input_cost_per_m: 0.075,
            output_cost_per_m: 0.30,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }

    #[cfg(feature = "gemini")]
    pub fn gemini_1_5_pro() -> Self {
        Self {
            id: "gemini-1.5-pro".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            provider: Provider::Google,
            tier: ModelTier::Balanced,
            context_window: 2_000_000,
            max_output: 8192,
            input_cost_per_m: 1.25,
            output_cost_per_m: 5.00,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }

    #[cfg(feature = "gemini")]
    pub fn gemini_1_5_flash() -> Self {
        Self {
            id: "gemini-1.5-flash".to_string(),
            name: "Gemini 1.5 Flash".to_string(),
            provider: Provider::Google,
            tier: ModelTier::Fast,
            context_window: 1_000_000,
            max_output: 8192,
            input_cost_per_m: 0.075,
            output_cost_per_m: 0.30,
            supports_caching: true,
            supports_vision: true,
            supports_tools: true,
        }
    }
}

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Cache control for prompt caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            cache_control: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            cache_control: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            cache_control: None,
        }
    }

    /// Mark this message for caching.
    pub fn with_cache(mut self) -> Self {
        self.cache_control = Some(CacheControl::Ephemeral);
        self
    }
}

/// Cache control directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheControl {
    Ephemeral,
}

/// Completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model to use (overrides router decision if set)
    pub model: Option<String>,
    /// System prompt
    pub system: Option<String>,
    /// Conversation messages
    pub messages: Vec<ChatMessage>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 - 1.0)
    pub temperature: Option<f64>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Enable prompt caching
    pub enable_caching: bool,
    /// Metadata for tracking
    pub metadata: Option<HashMap<String, String>>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            model: None,
            system: None,
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            stop: None,
            enable_caching: false,
            metadata: None,
        }
    }
}

impl CompletionRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn with_message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    pub fn with_caching(mut self, enable: bool) -> Self {
        self.enable_caching = enable;
        self
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Tokens read from cache (if caching enabled)
    pub cache_read_tokens: Option<u64>,
    /// Tokens written to cache (if caching enabled)
    pub cache_creation_tokens: Option<u64>,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Calculate effective input tokens (accounting for cache)
    pub fn effective_input_tokens(&self) -> u64 {
        let cache_read = self.cache_read_tokens.unwrap_or(0);
        // Cache reads are typically 90% cheaper
        self.input_tokens - cache_read + (cache_read / 10)
    }
}

/// Completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Response ID
    pub id: String,
    /// Model used
    pub model: String,
    /// Generated content
    pub content: String,
    /// Stop reason
    pub stop_reason: Option<StopReason>,
    /// Token usage
    pub usage: TokenUsage,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
    /// Cost in USD (if calculable)
    pub cost: Option<f64>,
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

/// Embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// Model to use
    pub model: Option<String>,
    /// Texts to embed
    pub texts: Vec<String>,
}

/// Embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Model used
    pub model: String,
    /// Embedding vectors
    pub embeddings: Vec<Vec<f32>>,
    /// Token usage
    pub usage: TokenUsage,
}

/// Cost tracking for a component or session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total cache read tokens
    pub total_cache_read_tokens: u64,
    /// Total cache creation tokens
    pub total_cache_creation_tokens: u64,
    /// Total cost in USD
    pub total_cost: f64,
    /// Number of requests
    pub request_count: u64,
    /// Per-model breakdown
    pub by_model: HashMap<String, ModelCosts>,
    /// Costs from root-level (premium) model calls
    #[serde(default)]
    pub root_costs: TierCosts,
    /// Costs from recursive (budget) model calls
    #[serde(default)]
    pub recursive_costs: TierCosts,
}

/// Costs breakdown by model tier (for dual-model optimization).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierCosts {
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens generated
    pub output_tokens: u64,
    /// Total cost in USD
    pub cost: f64,
    /// Number of requests
    pub request_count: u64,
}

/// Costs for a specific model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCosts {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
    pub request_count: u64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record usage from a completion response.
    pub fn record(&mut self, model: &str, usage: &TokenUsage, cost: Option<f64>) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cache_read_tokens += usage.cache_read_tokens.unwrap_or(0);
        self.total_cache_creation_tokens += usage.cache_creation_tokens.unwrap_or(0);
        self.request_count += 1;

        if let Some(c) = cost {
            self.total_cost += c;
        }

        let model_costs = self.by_model.entry(model.to_string()).or_default();
        model_costs.input_tokens += usage.input_tokens;
        model_costs.output_tokens += usage.output_tokens;
        model_costs.request_count += 1;
        if let Some(c) = cost {
            model_costs.cost += c;
        }
    }

    /// Merge another tracker into this one.
    pub fn merge(&mut self, other: &CostTracker) {
        self.total_input_tokens += other.total_input_tokens;
        self.total_output_tokens += other.total_output_tokens;
        self.total_cache_read_tokens += other.total_cache_read_tokens;
        self.total_cache_creation_tokens += other.total_cache_creation_tokens;
        self.total_cost += other.total_cost;
        self.request_count += other.request_count;

        for (model, costs) in &other.by_model {
            let entry = self.by_model.entry(model.clone()).or_default();
            entry.input_tokens += costs.input_tokens;
            entry.output_tokens += costs.output_tokens;
            entry.cost += costs.cost;
            entry.request_count += costs.request_count;
        }

        // Merge tier costs
        self.root_costs.merge(&other.root_costs);
        self.recursive_costs.merge(&other.recursive_costs);
    }

    /// Record usage with tier information (for dual-model optimization).
    ///
    /// # Arguments
    ///
    /// * `model` - The model name
    /// * `usage` - Token usage from the response
    /// * `cost` - Calculated cost in USD
    /// * `is_root` - Whether this was a root (premium) or recursive (budget) call
    pub fn record_with_tier(
        &mut self,
        model: &str,
        usage: &TokenUsage,
        cost: Option<f64>,
        is_root: bool,
    ) {
        // Record in main tracker
        self.record(model, usage, cost);

        // Record in appropriate tier
        let tier_costs = if is_root {
            &mut self.root_costs
        } else {
            &mut self.recursive_costs
        };

        tier_costs.input_tokens += usage.input_tokens;
        tier_costs.output_tokens += usage.output_tokens;
        tier_costs.request_count += 1;
        if let Some(c) = cost {
            tier_costs.cost += c;
        }
    }

    /// Get a breakdown report of costs by tier.
    pub fn tier_breakdown(&self) -> TierBreakdown {
        let root_pct = if self.total_cost > 0.0 {
            (self.root_costs.cost / self.total_cost) * 100.0
        } else {
            0.0
        };

        let recursive_pct = if self.total_cost > 0.0 {
            (self.recursive_costs.cost / self.total_cost) * 100.0
        } else {
            0.0
        };

        TierBreakdown {
            root_cost: self.root_costs.cost,
            root_requests: self.root_costs.request_count,
            root_tokens: self.root_costs.input_tokens + self.root_costs.output_tokens,
            root_percentage: root_pct,
            recursive_cost: self.recursive_costs.cost,
            recursive_requests: self.recursive_costs.request_count,
            recursive_tokens: self.recursive_costs.input_tokens + self.recursive_costs.output_tokens,
            recursive_percentage: recursive_pct,
            total_cost: self.total_cost,
            estimated_single_model_cost: self.estimate_single_model_cost(),
            savings_percentage: self.calculate_savings_percentage(),
        }
    }

    /// Estimate what the cost would have been using only the root model.
    ///
    /// This helps quantify savings from dual-model optimization.
    fn estimate_single_model_cost(&self) -> f64 {
        // Rough estimate: assume recursive calls would cost 3x more with premium model
        // This is based on typical Opus vs Haiku pricing ratios
        self.root_costs.cost + (self.recursive_costs.cost * 3.0)
    }

    /// Calculate the percentage savings from dual-model optimization.
    fn calculate_savings_percentage(&self) -> f64 {
        let estimated_single = self.estimate_single_model_cost();
        if estimated_single > 0.0 {
            ((estimated_single - self.total_cost) / estimated_single) * 100.0
        } else {
            0.0
        }
    }
}

impl TierCosts {
    /// Merge another TierCosts into this one.
    pub fn merge(&mut self, other: &TierCosts) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cost += other.cost;
        self.request_count += other.request_count;
    }

    /// Get total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Breakdown of costs by model tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBreakdown {
    /// Cost from root (premium) model
    pub root_cost: f64,
    /// Number of root model requests
    pub root_requests: u64,
    /// Total tokens from root model
    pub root_tokens: u64,
    /// Percentage of total cost from root model
    pub root_percentage: f64,

    /// Cost from recursive (budget) model
    pub recursive_cost: f64,
    /// Number of recursive model requests
    pub recursive_requests: u64,
    /// Total tokens from recursive model
    pub recursive_tokens: u64,
    /// Percentage of total cost from recursive model
    pub recursive_percentage: f64,

    /// Total actual cost
    pub total_cost: f64,
    /// Estimated cost if only using root model
    pub estimated_single_model_cost: f64,
    /// Percentage savings from dual-model optimization
    pub savings_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_cost_calculation() {
        let sonnet = ModelSpec::claude_sonnet();
        // 1M input + 500k output
        let cost = sonnet.calculate_cost(1_000_000, 500_000);
        // 1M * $3/M + 0.5M * $15/M = $3 + $7.5 = $10.5
        assert!((cost - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_chat_message_builder() {
        let msg = ChatMessage::user("Hello").with_cache();
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.cache_control, Some(CacheControl::Ephemeral));
    }

    #[test]
    fn test_completion_request_builder() {
        let req = CompletionRequest::new()
            .with_model("claude-3-5-sonnet-20241022")
            .with_system("You are helpful")
            .with_message(ChatMessage::user("Hi"))
            .with_max_tokens(1000)
            .with_temperature(0.7);

        assert_eq!(req.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(req.system, Some("You are helpful".to_string()));
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.max_tokens, Some(1000));
        assert_eq!(req.temperature, Some(0.7));
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();

        let usage1 = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: Some(200),
            cache_creation_tokens: None,
        };
        tracker.record("claude-3-5-sonnet", &usage1, Some(0.01));

        let usage2 = TokenUsage {
            input_tokens: 2000,
            output_tokens: 1000,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };
        tracker.record("claude-3-5-sonnet", &usage2, Some(0.02));

        assert_eq!(tracker.total_input_tokens, 3000);
        assert_eq!(tracker.total_output_tokens, 1500);
        assert_eq!(tracker.total_cache_read_tokens, 200);
        assert_eq!(tracker.request_count, 2);
        assert!((tracker.total_cost - 0.03).abs() < 0.001);

        let model_costs = tracker.by_model.get("claude-3-5-sonnet").unwrap();
        assert_eq!(model_costs.request_count, 2);
    }

    #[test]
    fn test_token_usage_effective() {
        let usage = TokenUsage {
            input_tokens: 10000,
            output_tokens: 500,
            cache_read_tokens: Some(8000),
            cache_creation_tokens: None,
        };
        // 10000 - 8000 + 800 = 2800 effective input tokens
        assert_eq!(usage.effective_input_tokens(), 2800);
    }

    #[test]
    fn test_model_tier_ordering() {
        assert!(ModelTier::Flagship < ModelTier::Balanced);
        assert!(ModelTier::Balanced < ModelTier::Fast);
    }
}
