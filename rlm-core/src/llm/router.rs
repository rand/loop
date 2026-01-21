//! Smart router for query-aware model selection.
//!
//! Routes queries to appropriate models based on:
//! - Query type (architecture, multi-file, debugging, extraction, simple)
//! - Recursion depth (deeper calls use cheaper models)
//! - Budget constraints
//! - Provider availability
//!
//! # Dual-Model Optimization
//!
//! For RLM orchestration, the router supports dual-model configuration:
//! - Premium model (e.g., Opus) for root orchestration decisions
//! - Budget model (e.g., Haiku) for recursive sub-queries
//!
//! This can achieve 30-50% cost savings without significant quality loss.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::types::{ModelSpec, ModelTier, Provider};

/// Query type classification for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// Architecture and design decisions
    Architecture,
    /// Multi-file analysis and refactoring
    MultiFile,
    /// Debugging and error analysis
    Debugging,
    /// Information extraction
    Extraction,
    /// Simple queries and tasks
    Simple,
}

impl QueryType {
    /// Classify a query based on content analysis.
    pub fn classify(query: &str) -> Self {
        static PATTERNS: OnceLock<QueryPatterns> = OnceLock::new();
        let patterns = PATTERNS.get_or_init(QueryPatterns::new);

        let query_lower = query.to_lowercase();

        // Check patterns in order of specificity
        if patterns.architecture.is_match(&query_lower) {
            QueryType::Architecture
        } else if patterns.multi_file.is_match(&query_lower) {
            QueryType::MultiFile
        } else if patterns.debugging.is_match(&query_lower) {
            QueryType::Debugging
        } else if patterns.extraction.is_match(&query_lower) {
            QueryType::Extraction
        } else {
            QueryType::Simple
        }
    }

    /// Get the recommended model tier for this query type at depth 0.
    pub fn base_tier(&self) -> ModelTier {
        match self {
            QueryType::Architecture => ModelTier::Flagship,
            QueryType::MultiFile => ModelTier::Flagship,
            QueryType::Debugging => ModelTier::Balanced,
            QueryType::Extraction => ModelTier::Balanced,
            QueryType::Simple => ModelTier::Fast,
        }
    }
}

/// Compiled regex patterns for query classification.
struct QueryPatterns {
    architecture: Regex,
    multi_file: Regex,
    debugging: Regex,
    extraction: Regex,
}

impl QueryPatterns {
    fn new() -> Self {
        Self {
            architecture: Regex::new(
                r"(?x)
                architect|design|structure|refactor|
                pattern|system|service|component|
                how\s+should|what\s+approach|
                trade.?off|alternative|option|
                scale|performance|security
                "
            ).unwrap(),
            multi_file: Regex::new(
                r"(?x)
                all\s+files|multiple\s+files|across|
                codebase|project|module|package|
                every|find\s+all|search|grep|
                dependency|import|reference|
                rename|move|reorganize
                "
            ).unwrap(),
            debugging: Regex::new(
                r"(?x)
                debug|error|bug|issue|problem|
                fail|crash|exception|stack|trace|
                why\s+does|why\s+is|what.s\s+wrong|
                doesn.t\s+work|not\s+working|broken|
                fix|diagnose|investigate|root\s+cause
                "
            ).unwrap(),
            extraction: Regex::new(
                r"(?x)
                extract|parse|summarize|list|
                what\s+is|what\s+are|describe|explain|
                get|find|show|tell\s+me|give\s+me|
                count|how\s+many|identify
                "
            ).unwrap(),
        }
    }
}

// =============================================================================
// Dual-Model Configuration for RLM Optimization
// =============================================================================

/// Strategy for switching between root and recursive models.
///
/// Different strategies offer trade-offs between quality and cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SwitchStrategy {
    /// Switch at a specified recursion depth.
    ///
    /// - depth=1: Switch after root call (most aggressive)
    /// - depth=2: Allow one level of premium recursive calls
    /// - depth=3+: More premium calls, higher quality but more cost
    Depth {
        /// Depth at which to switch to recursive model
        depth: u32,
    },

    /// Switch after a token budget is consumed.
    ///
    /// Useful when you want to guarantee a certain amount of premium
    /// processing regardless of call structure.
    TokenBudget {
        /// Token budget for premium model
        tokens: u64,
    },

    /// Switch based on query classification.
    ///
    /// Allows premium model only for reasoning-heavy queries,
    /// using budget model for extraction and simple queries.
    QueryType {
        /// If true, only use premium for architecture/debugging queries
        reasoning_only: bool,
    },

    /// Hybrid strategy combining depth and token budget.
    ///
    /// Switches when EITHER condition is met (whichever comes first).
    Hybrid {
        /// Maximum depth for premium model
        depth: u32,
        /// Maximum tokens for premium model
        tokens: u64,
    },

    /// Always use the root model (no switching).
    ///
    /// Useful for quality-critical tasks where cost is not a concern.
    AlwaysRoot,

    /// Always use the recursive model (maximum savings).
    ///
    /// Useful for batch processing where quality can be traded for cost.
    AlwaysRecursive,
}

impl SwitchStrategy {
    /// Check if we should use the recursive (budget) model.
    pub fn should_use_recursive(
        &self,
        depth: u32,
        tokens_used: u64,
        query_type: Option<QueryType>,
    ) -> bool {
        match self {
            SwitchStrategy::Depth { depth: switch_depth } => depth >= *switch_depth,
            SwitchStrategy::TokenBudget { tokens } => tokens_used >= *tokens,
            SwitchStrategy::QueryType { reasoning_only } => {
                if *reasoning_only {
                    // Only use premium for architecture and debugging
                    query_type.map_or(true, |qt| {
                        !matches!(qt, QueryType::Architecture | QueryType::Debugging)
                    })
                } else {
                    false
                }
            }
            SwitchStrategy::Hybrid { depth: switch_depth, tokens } => {
                depth >= *switch_depth || tokens_used >= *tokens
            }
            SwitchStrategy::AlwaysRoot => false,
            SwitchStrategy::AlwaysRecursive => true,
        }
    }
}

impl Default for SwitchStrategy {
    fn default() -> Self {
        // Default: switch at depth 1 (balanced)
        SwitchStrategy::Depth { depth: 1 }
    }
}

/// Configuration for dual-model RLM optimization.
///
/// Enables cost savings by using a premium model for root-level orchestration
/// and a budget model for recursive sub-queries.
///
/// # Example
///
/// ```
/// use rlm_core::llm::{DualModelConfig, SwitchStrategy, ModelSpec};
///
/// let config = DualModelConfig::balanced();
/// assert_eq!(config.switch_strategy, SwitchStrategy::Depth { depth: 2 });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualModelConfig {
    /// Premium model for root orchestration decisions.
    pub root_model: ModelSpec,

    /// Budget model for recursive sub-queries.
    pub recursive_model: ModelSpec,

    /// Strategy for determining when to switch models.
    pub switch_strategy: SwitchStrategy,

    /// Optional name for this configuration (for logging/debugging).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DualModelConfig {
    /// Create a new dual-model configuration.
    pub fn new(root_model: ModelSpec, recursive_model: ModelSpec) -> Self {
        Self {
            root_model,
            recursive_model,
            switch_strategy: SwitchStrategy::default(),
            name: None,
        }
    }

    /// Set the switch strategy.
    pub fn with_strategy(mut self, strategy: SwitchStrategy) -> Self {
        self.switch_strategy = strategy;
        self
    }

    /// Set a name for this configuration.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Aggressive configuration: maximize cost savings.
    ///
    /// - Uses Opus for root only
    /// - Switches to Haiku at depth 1
    /// - Expected savings: 40-50%
    pub fn aggressive() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth { depth: 1 },
            name: Some("aggressive".to_string()),
        }
    }

    /// Balanced configuration: balance quality and cost.
    ///
    /// - Uses Opus for root and first recursive level
    /// - Switches to Haiku at depth 2
    /// - Expected savings: 30-40%
    pub fn balanced() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth { depth: 2 },
            name: Some("balanced".to_string()),
        }
    }

    /// Quality-first configuration: prioritize output quality.
    ///
    /// - Uses Opus for root and two recursive levels
    /// - Switches to Sonnet (not Haiku) at depth 3
    /// - Expected savings: 15-25%
    pub fn quality_first() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_sonnet(),
            switch_strategy: SwitchStrategy::Depth { depth: 3 },
            name: Some("quality_first".to_string()),
        }
    }

    /// Budget configuration: maximize savings with acceptable quality.
    ///
    /// - Uses Sonnet for root
    /// - Switches to Haiku immediately
    /// - Expected savings: 50-60%
    pub fn budget() -> Self {
        Self {
            root_model: ModelSpec::claude_sonnet(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth { depth: 1 },
            name: Some("budget".to_string()),
        }
    }

    /// Token-limited configuration: premium until budget exhausted.
    ///
    /// - Uses Opus until token limit reached
    /// - Then switches to Haiku
    pub fn token_limited(premium_tokens: u64) -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::TokenBudget { tokens: premium_tokens },
            name: Some(format!("token_limited_{}", premium_tokens)),
        }
    }

    /// Select the appropriate model based on current state.
    pub fn select_model(
        &self,
        depth: u32,
        tokens_used: u64,
        query_type: Option<QueryType>,
    ) -> &ModelSpec {
        if self.switch_strategy.should_use_recursive(depth, tokens_used, query_type) {
            &self.recursive_model
        } else {
            &self.root_model
        }
    }

    /// Check if currently using the root (premium) model.
    pub fn is_using_root(&self, depth: u32, tokens_used: u64) -> bool {
        !self.switch_strategy.should_use_recursive(depth, tokens_used, None)
    }
}

impl Default for DualModelConfig {
    fn default() -> Self {
        Self::balanced()
    }
}

/// Context for routing decisions.
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// Current recursion depth
    pub depth: u32,
    /// Maximum allowed depth
    pub max_depth: u32,
    /// Remaining budget in USD
    pub remaining_budget: Option<f64>,
    /// Preferred provider (if any)
    pub preferred_provider: Option<Provider>,
    /// Require caching support
    pub require_caching: bool,
    /// Require vision support
    pub require_vision: bool,
    /// Require tool use support
    pub require_tools: bool,
}

impl RoutingContext {
    pub fn new() -> Self {
        Self {
            depth: 0,
            max_depth: 5,
            remaining_budget: None,
            preferred_provider: None,
            require_caching: false,
            require_vision: false,
            require_tools: false,
        }
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_budget(mut self, budget: f64) -> Self {
        self.remaining_budget = Some(budget);
        self
    }

    pub fn with_provider(mut self, provider: Provider) -> Self {
        self.preferred_provider = Some(provider);
        self
    }

    pub fn requiring_caching(mut self) -> Self {
        self.require_caching = true;
        self
    }

    pub fn requiring_vision(mut self) -> Self {
        self.require_vision = true;
        self
    }

    pub fn requiring_tools(mut self) -> Self {
        self.require_tools = true;
        self
    }
}

/// Routing decision output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected model
    pub model: ModelSpec,
    /// Query classification
    pub query_type: QueryType,
    /// Recommended tier
    pub tier: ModelTier,
    /// Reasoning for selection
    pub reason: String,
    /// Estimated cost (if calculable)
    pub estimated_cost: Option<f64>,
}

/// Smart router for model selection.
pub struct SmartRouter {
    /// Available models
    models: Vec<ModelSpec>,
    /// Default model for each tier
    tier_defaults: TierDefaults,
}

/// Default models for each tier.
#[derive(Debug, Clone)]
pub struct TierDefaults {
    pub flagship: ModelSpec,
    pub balanced: ModelSpec,
    pub fast: ModelSpec,
}

impl Default for TierDefaults {
    fn default() -> Self {
        Self {
            flagship: ModelSpec::claude_opus(),
            balanced: ModelSpec::claude_sonnet(),
            fast: ModelSpec::claude_haiku(),
        }
    }
}

impl SmartRouter {
    /// Create a new router with default Anthropic models.
    pub fn new() -> Self {
        Self {
            models: vec![
                ModelSpec::claude_opus(),
                ModelSpec::claude_sonnet(),
                ModelSpec::claude_haiku(),
                ModelSpec::gpt4o(),
                ModelSpec::gpt4o_mini(),
            ],
            tier_defaults: TierDefaults::default(),
        }
    }

    /// Create with custom models.
    pub fn with_models(models: Vec<ModelSpec>) -> Self {
        // Find best model for each tier
        let flagship = models
            .iter()
            .filter(|m| m.tier == ModelTier::Flagship)
            .min_by(|a, b| a.input_cost_per_m.partial_cmp(&b.input_cost_per_m).unwrap())
            .cloned()
            .unwrap_or_else(ModelSpec::claude_opus);

        let balanced = models
            .iter()
            .filter(|m| m.tier == ModelTier::Balanced)
            .min_by(|a, b| a.input_cost_per_m.partial_cmp(&b.input_cost_per_m).unwrap())
            .cloned()
            .unwrap_or_else(ModelSpec::claude_sonnet);

        let fast = models
            .iter()
            .filter(|m| m.tier == ModelTier::Fast)
            .min_by(|a, b| a.input_cost_per_m.partial_cmp(&b.input_cost_per_m).unwrap())
            .cloned()
            .unwrap_or_else(ModelSpec::claude_haiku);

        Self {
            models,
            tier_defaults: TierDefaults {
                flagship,
                balanced,
                fast,
            },
        }
    }

    /// Set custom tier defaults.
    pub fn with_tier_defaults(mut self, defaults: TierDefaults) -> Self {
        self.tier_defaults = defaults;
        self
    }

    /// Route a query to the best model.
    pub fn route(&self, query: &str, context: &RoutingContext) -> RoutingDecision {
        let query_type = QueryType::classify(query);
        let base_tier = query_type.base_tier();

        // Adjust tier based on depth (deeper = cheaper)
        let adjusted_tier = self.adjust_tier_for_depth(base_tier, context.depth);

        // Find best model matching requirements
        let model = self.select_model(adjusted_tier, context);

        let reason = format!(
            "Query type '{}' at depth {} -> {} tier (adjusted from {})",
            format!("{:?}", query_type).to_lowercase(),
            context.depth,
            format!("{:?}", adjusted_tier).to_lowercase(),
            format!("{:?}", base_tier).to_lowercase(),
        );

        RoutingDecision {
            model,
            query_type,
            tier: adjusted_tier,
            reason,
            estimated_cost: None,
        }
    }

    /// Route an RLM query using dual-model configuration.
    ///
    /// This method is optimized for recursive language model orchestration,
    /// using a premium model for root decisions and a budget model for
    /// recursive sub-queries.
    ///
    /// # Arguments
    ///
    /// * `query` - The query text for classification
    /// * `context` - Routing context with depth, budget, etc.
    /// * `config` - Dual-model configuration
    /// * `tokens_used` - Total tokens used so far (for token-based switching)
    ///
    /// # Returns
    ///
    /// A routing decision indicating which model to use.
    pub fn route_rlm(
        &self,
        query: &str,
        context: &RoutingContext,
        config: &DualModelConfig,
        tokens_used: u64,
    ) -> RoutingDecision {
        let query_type = QueryType::classify(query);

        // Use dual-model config to select model
        let model = config.select_model(context.depth, tokens_used, Some(query_type));

        // Determine which tier we're using
        let tier = model.tier;
        let is_root = config.is_using_root(context.depth, tokens_used);

        let reason = format!(
            "RLM {} model at depth {} (strategy: {:?}, query: {:?})",
            if is_root { "root" } else { "recursive" },
            context.depth,
            config.switch_strategy,
            query_type,
        );

        RoutingDecision {
            model: model.clone(),
            query_type,
            tier,
            reason,
            estimated_cost: None,
        }
    }

    /// Route with optional dual-model config.
    ///
    /// If a dual-model config is provided, uses `route_rlm`.
    /// Otherwise, falls back to standard `route`.
    pub fn route_with_config(
        &self,
        query: &str,
        context: &RoutingContext,
        dual_config: Option<&DualModelConfig>,
        tokens_used: u64,
    ) -> RoutingDecision {
        match dual_config {
            Some(config) => self.route_rlm(query, context, config, tokens_used),
            None => self.route(query, context),
        }
    }

    /// Adjust tier based on recursion depth.
    fn adjust_tier_for_depth(&self, base: ModelTier, depth: u32) -> ModelTier {
        match depth {
            0 => base,
            1 => match base {
                ModelTier::Flagship => ModelTier::Balanced,
                _ => base,
            },
            _ => ModelTier::Fast,
        }
    }

    /// Select the best model for the tier and constraints.
    fn select_model(&self, tier: ModelTier, context: &RoutingContext) -> ModelSpec {
        // Filter models by requirements
        let candidates: Vec<_> = self
            .models
            .iter()
            .filter(|m| {
                // Check tier
                m.tier <= tier
                    // Check provider preference
                    && context.preferred_provider.map_or(true, |p| m.provider == p)
                    // Check capability requirements
                    && (!context.require_caching || m.supports_caching)
                    && (!context.require_vision || m.supports_vision)
                    && (!context.require_tools || m.supports_tools)
                    // Check budget (rough estimate for 10k tokens)
                    && context.remaining_budget.map_or(true, |b| {
                        m.calculate_cost(10_000, 1_000) < b
                    })
            })
            .collect();

        // Pick the best candidate (prefer exact tier match, then cheapest)
        candidates
            .iter()
            .filter(|m| m.tier == tier)
            .min_by(|a, b| a.input_cost_per_m.partial_cmp(&b.input_cost_per_m).unwrap())
            .cloned()
            .cloned()
            .or_else(|| {
                candidates
                    .iter()
                    .min_by(|a, b| a.input_cost_per_m.partial_cmp(&b.input_cost_per_m).unwrap())
                    .cloned()
                    .cloned()
            })
            .unwrap_or_else(|| self.tier_default(tier))
    }

    /// Get the default model for a tier.
    fn tier_default(&self, tier: ModelTier) -> ModelSpec {
        match tier {
            ModelTier::Flagship => self.tier_defaults.flagship.clone(),
            ModelTier::Balanced => self.tier_defaults.balanced.clone(),
            ModelTier::Fast => self.tier_defaults.fast.clone(),
        }
    }

    /// Get all available models.
    pub fn models(&self) -> &[ModelSpec] {
        &self.models
    }

    /// Add a model to the router.
    pub fn add_model(&mut self, model: ModelSpec) {
        self.models.push(model);
    }
}

impl Default for SmartRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_classification() {
        // Architecture queries
        assert_eq!(
            QueryType::classify("How should I architect this service?"),
            QueryType::Architecture
        );
        assert_eq!(
            QueryType::classify("What's the best design pattern here?"),
            QueryType::Architecture
        );

        // Multi-file queries
        assert_eq!(
            QueryType::classify("Find all files that import this module"),
            QueryType::MultiFile
        );
        assert_eq!(
            QueryType::classify("Search across the codebase for usages"),
            QueryType::MultiFile
        );

        // Debugging queries
        assert_eq!(
            QueryType::classify("Why does this error happen?"),
            QueryType::Debugging
        );
        assert_eq!(
            QueryType::classify("Debug this stack trace"),
            QueryType::Debugging
        );

        // Extraction queries
        assert_eq!(
            QueryType::classify("What is this function doing?"),
            QueryType::Extraction
        );
        assert_eq!(
            QueryType::classify("Summarize this code"),
            QueryType::Extraction
        );

        // Simple queries
        assert_eq!(QueryType::classify("Hello"), QueryType::Simple);
        assert_eq!(QueryType::classify("Thanks!"), QueryType::Simple);
    }

    #[test]
    fn test_base_tier_mapping() {
        assert_eq!(QueryType::Architecture.base_tier(), ModelTier::Flagship);
        assert_eq!(QueryType::MultiFile.base_tier(), ModelTier::Flagship);
        assert_eq!(QueryType::Debugging.base_tier(), ModelTier::Balanced);
        assert_eq!(QueryType::Extraction.base_tier(), ModelTier::Balanced);
        assert_eq!(QueryType::Simple.base_tier(), ModelTier::Fast);
    }

    #[test]
    fn test_router_depth_adjustment() {
        let router = SmartRouter::new();
        let context = RoutingContext::new();

        // Depth 0: Use base tier
        let decision = router.route("Design a new architecture", &context);
        assert_eq!(decision.tier, ModelTier::Flagship);

        // Depth 1: Downgrade flagship to balanced
        let decision = router.route("Design a new architecture", &context.clone().with_depth(1));
        assert_eq!(decision.tier, ModelTier::Balanced);

        // Depth 2+: Use fast tier
        let decision = router.route("Design a new architecture", &context.clone().with_depth(2));
        assert_eq!(decision.tier, ModelTier::Fast);
    }

    #[test]
    fn test_router_provider_preference() {
        let router = SmartRouter::new();
        let context = RoutingContext::new().with_provider(Provider::OpenAI);

        let decision = router.route("Simple question", &context);
        assert_eq!(decision.model.provider, Provider::OpenAI);
    }

    #[test]
    fn test_router_caching_requirement() {
        let router = SmartRouter::new();
        let context = RoutingContext::new().requiring_caching();

        let decision = router.route("Architecture question", &context);
        assert!(decision.model.supports_caching);
    }

    #[test]
    fn test_routing_context_builder() {
        let context = RoutingContext::new()
            .with_depth(2)
            .with_max_depth(5)
            .with_budget(1.0)
            .with_provider(Provider::Anthropic)
            .requiring_caching()
            .requiring_vision()
            .requiring_tools();

        assert_eq!(context.depth, 2);
        assert_eq!(context.max_depth, 5);
        assert_eq!(context.remaining_budget, Some(1.0));
        assert_eq!(context.preferred_provider, Some(Provider::Anthropic));
        assert!(context.require_caching);
        assert!(context.require_vision);
        assert!(context.require_tools);
    }

    #[test]
    fn test_simple_query_uses_fast_tier() {
        let router = SmartRouter::new();
        let context = RoutingContext::new();

        let decision = router.route("Hello, how are you?", &context);
        assert_eq!(decision.query_type, QueryType::Simple);
        assert_eq!(decision.tier, ModelTier::Fast);
    }

    #[test]
    fn test_debugging_query_uses_balanced_tier() {
        let router = SmartRouter::new();
        let context = RoutingContext::new();

        let decision = router.route("Why is this test failing?", &context);
        assert_eq!(decision.query_type, QueryType::Debugging);
        assert_eq!(decision.tier, ModelTier::Balanced);
    }

    // ==========================================================================
    // Dual-Model Configuration Tests
    // ==========================================================================

    #[test]
    fn test_dual_model_config_aggressive() {
        let config = DualModelConfig::aggressive();
        assert_eq!(config.root_model.id, "claude-3-opus-20240229");
        assert_eq!(config.recursive_model.id, "claude-3-haiku-20240307");
        assert_eq!(config.switch_strategy, SwitchStrategy::Depth { depth: 1 });
    }

    #[test]
    fn test_dual_model_config_balanced() {
        let config = DualModelConfig::balanced();
        assert_eq!(config.switch_strategy, SwitchStrategy::Depth { depth: 2 });
    }

    #[test]
    fn test_dual_model_config_quality_first() {
        let config = DualModelConfig::quality_first();
        assert_eq!(config.recursive_model.id, "claude-3-5-sonnet-20241022");
        assert_eq!(config.switch_strategy, SwitchStrategy::Depth { depth: 3 });
    }

    #[test]
    fn test_switch_strategy_depth() {
        let strategy = SwitchStrategy::Depth { depth: 2 };

        // At depth 0 and 1, use root model
        assert!(!strategy.should_use_recursive(0, 0, None));
        assert!(!strategy.should_use_recursive(1, 0, None));

        // At depth 2+, use recursive model
        assert!(strategy.should_use_recursive(2, 0, None));
        assert!(strategy.should_use_recursive(3, 0, None));
    }

    #[test]
    fn test_switch_strategy_token_budget() {
        let strategy = SwitchStrategy::TokenBudget { tokens: 10000 };

        // Under budget, use root model
        assert!(!strategy.should_use_recursive(0, 5000, None));
        assert!(!strategy.should_use_recursive(5, 9999, None));

        // At or over budget, use recursive model
        assert!(strategy.should_use_recursive(0, 10000, None));
        assert!(strategy.should_use_recursive(0, 15000, None));
    }

    #[test]
    fn test_switch_strategy_hybrid() {
        let strategy = SwitchStrategy::Hybrid { depth: 2, tokens: 10000 };

        // Neither condition met
        assert!(!strategy.should_use_recursive(1, 5000, None));

        // Depth condition met
        assert!(strategy.should_use_recursive(2, 5000, None));

        // Token condition met
        assert!(strategy.should_use_recursive(1, 10000, None));

        // Both conditions met
        assert!(strategy.should_use_recursive(3, 15000, None));
    }

    #[test]
    fn test_switch_strategy_query_type() {
        let strategy = SwitchStrategy::QueryType { reasoning_only: true };

        // Architecture queries use root
        assert!(!strategy.should_use_recursive(0, 0, Some(QueryType::Architecture)));

        // Debugging queries use root
        assert!(!strategy.should_use_recursive(0, 0, Some(QueryType::Debugging)));

        // Simple queries use recursive
        assert!(strategy.should_use_recursive(0, 0, Some(QueryType::Simple)));

        // Extraction queries use recursive
        assert!(strategy.should_use_recursive(0, 0, Some(QueryType::Extraction)));
    }

    #[test]
    fn test_dual_model_select_model() {
        let config = DualModelConfig::aggressive();

        // Depth 0: root model
        let model = config.select_model(0, 0, None);
        assert_eq!(model.id, "claude-3-opus-20240229");

        // Depth 1+: recursive model
        let model = config.select_model(1, 0, None);
        assert_eq!(model.id, "claude-3-haiku-20240307");
    }

    #[test]
    fn test_route_rlm() {
        let router = SmartRouter::new();
        let config = DualModelConfig::aggressive();

        // At depth 0, should use root model
        let context = RoutingContext::new().with_depth(0);
        let decision = router.route_rlm("Design an architecture", &context, &config, 0);
        assert_eq!(decision.model.id, "claude-3-opus-20240229");
        assert!(decision.reason.contains("root"));

        // At depth 1, should use recursive model
        let context = RoutingContext::new().with_depth(1);
        let decision = router.route_rlm("Extract entities", &context, &config, 0);
        assert_eq!(decision.model.id, "claude-3-haiku-20240307");
        assert!(decision.reason.contains("recursive"));
    }

    #[test]
    fn test_route_with_config_fallback() {
        let router = SmartRouter::new();
        let context = RoutingContext::new();

        // Without config, should use standard routing
        let decision = router.route_with_config("Simple question", &context, None, 0);
        assert_eq!(decision.tier, ModelTier::Fast);

        // With config, should use dual-model routing
        let config = DualModelConfig::aggressive();
        let decision = router.route_with_config("Simple question", &context, Some(&config), 0);
        assert_eq!(decision.model.id, "claude-3-opus-20240229"); // Root at depth 0
    }
}
