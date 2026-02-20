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

use super::types::{ModelCallTier, ModelSpec, ModelTier, Provider};

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
                ",
            )
            .unwrap(),
            multi_file: Regex::new(
                r"(?x)
                all\s+files|multiple\s+files|across|
                codebase|project|module|package|
                every|find\s+all|search|grep|
                dependency|import|reference|
                rename|move|reorganize
                ",
            )
            .unwrap(),
            debugging: Regex::new(
                r"(?x)
                debug|error|bug|issue|problem|
                fail|crash|exception|stack|trace|
                why\s+does|why\s+is|what.s\s+wrong|
                doesn.t\s+work|not\s+working|broken|
                fix|diagnose|investigate|root\s+cause
                ",
            )
            .unwrap(),
            extraction: Regex::new(
                r"(?x)
                extract|parse|summarize|list|
                what\s+is|what\s+are|describe|explain|
                get|find|show|tell\s+me|give\s+me|
                count|how\s+many|identify
                ",
            )
            .unwrap(),
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

    /// Custom policy for fine-grained switching control.
    ///
    /// This variant is serializable and allows explicit query-type overrides
    /// plus optional depth/token thresholds.
    Custom {
        /// Optional depth threshold for switching to recursive.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_root_depth: Option<u32>,
        /// Optional token threshold for switching to recursive.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_root_tokens: Option<u64>,
        /// Query types that should always use recursive model.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        force_recursive_for: Vec<QueryType>,
        /// Query types that should always use root model.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        force_root_for: Vec<QueryType>,
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
            SwitchStrategy::Depth {
                depth: switch_depth,
            } => depth >= *switch_depth,
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
            SwitchStrategy::Hybrid {
                depth: switch_depth,
                tokens,
            } => depth >= *switch_depth || tokens_used >= *tokens,
            SwitchStrategy::Custom {
                max_root_depth,
                max_root_tokens,
                force_recursive_for,
                force_root_for,
            } => {
                if let Some(query_type) = query_type {
                    if force_root_for.contains(&query_type) {
                        return false;
                    }
                    if force_recursive_for.contains(&query_type) {
                        return true;
                    }
                }

                max_root_depth.map_or(false, |limit| depth >= limit)
                    || max_root_tokens.map_or(false, |limit| tokens_used >= limit)
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

    /// Optional override model for extraction/fallback calls.
    ///
    /// If not set, extraction calls default to `recursive_model`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_model: Option<ModelSpec>,

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
            extraction_model: None,
            name: None,
        }
    }

    /// Set the switch strategy.
    pub fn with_strategy(mut self, strategy: SwitchStrategy) -> Self {
        self.switch_strategy = strategy;
        self
    }

    /// Configure a serializable custom switch strategy.
    pub fn with_custom_strategy(
        mut self,
        max_root_depth: Option<u32>,
        max_root_tokens: Option<u64>,
        force_recursive_for: Vec<QueryType>,
        force_root_for: Vec<QueryType>,
    ) -> Self {
        self.switch_strategy = SwitchStrategy::Custom {
            max_root_depth,
            max_root_tokens,
            force_recursive_for,
            force_root_for,
        };
        self
    }

    /// Set a name for this configuration.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set a dedicated extraction model.
    pub fn with_extraction_model(mut self, model: ModelSpec) -> Self {
        self.extraction_model = Some(model);
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
            extraction_model: Some(ModelSpec::claude_haiku()),
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
            extraction_model: Some(ModelSpec::claude_haiku()),
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
            extraction_model: Some(ModelSpec::claude_sonnet()),
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
            extraction_model: Some(ModelSpec::claude_haiku()),
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
            switch_strategy: SwitchStrategy::TokenBudget {
                tokens: premium_tokens,
            },
            extraction_model: Some(ModelSpec::claude_haiku()),
            name: Some(format!("token_limited_{}", premium_tokens)),
        }
    }

    /// Get the configured extraction model, defaulting to recursive model.
    pub fn extraction_model(&self) -> &ModelSpec {
        self.extraction_model
            .as_ref()
            .unwrap_or(&self.recursive_model)
    }

    /// Select the appropriate model based on current state.
    pub fn select_model(
        &self,
        depth: u32,
        tokens_used: u64,
        query_type: Option<QueryType>,
    ) -> &ModelSpec {
        if self
            .switch_strategy
            .should_use_recursive(depth, tokens_used, query_type)
        {
            &self.recursive_model
        } else {
            &self.root_model
        }
    }

    /// Select model for an explicit orchestration tier.
    pub fn select_model_for_tier(
        &self,
        _depth: u32,
        _tokens_used: u64,
        _query_type: Option<QueryType>,
        tier: ModelCallTier,
    ) -> &ModelSpec {
        match tier {
            ModelCallTier::Root => &self.root_model,
            ModelCallTier::Recursive => &self.recursive_model,
            ModelCallTier::Extraction => self.extraction_model(),
        }
    }

    /// Check if currently using the root (premium) model.
    pub fn is_using_root(&self, depth: u32, tokens_used: u64) -> bool {
        !self
            .switch_strategy
            .should_use_recursive(depth, tokens_used, None)
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
        let call_tier = if config.switch_strategy.should_use_recursive(
            context.depth,
            tokens_used,
            Some(query_type),
        ) {
            ModelCallTier::Recursive
        } else {
            ModelCallTier::Root
        };

        self.route_rlm_for_tier(query, context, config, tokens_used, call_tier)
    }

    /// Route an RLM query for an explicit orchestration call tier.
    ///
    /// This is used by orchestrator paths that know the phase of the call
    /// (root/recursive/extraction) and need deterministic tiered accounting.
    pub fn route_rlm_for_tier(
        &self,
        query: &str,
        context: &RoutingContext,
        config: &DualModelConfig,
        tokens_used: u64,
        call_tier: ModelCallTier,
    ) -> RoutingDecision {
        let query_type = QueryType::classify(query);

        let model = match call_tier {
            ModelCallTier::Root | ModelCallTier::Recursive => config.select_model_for_tier(
                context.depth,
                tokens_used,
                Some(query_type),
                call_tier,
            ),
            ModelCallTier::Extraction => config.select_model_for_tier(
                context.depth,
                tokens_used,
                Some(query_type),
                ModelCallTier::Extraction,
            ),
        };

        let tier_label = match call_tier {
            ModelCallTier::Root => "root",
            ModelCallTier::Recursive => "recursive",
            ModelCallTier::Extraction => "extraction",
        };

        let reason = format!(
            "RLM {} model at depth {} (strategy: {:?}, query: {:?})",
            tier_label, context.depth, config.switch_strategy, query_type,
        );

        RoutingDecision {
            model: model.clone(),
            query_type,
            tier: model.tier,
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
    use proptest::prelude::*;

    fn query_type_strategy() -> impl Strategy<Value = QueryType> {
        prop_oneof![
            Just(QueryType::Architecture),
            Just(QueryType::MultiFile),
            Just(QueryType::Debugging),
            Just(QueryType::Extraction),
            Just(QueryType::Simple),
        ]
    }

    fn switch_strategy_strategy() -> impl Strategy<Value = SwitchStrategy> {
        prop_oneof![
            (0u32..6).prop_map(|depth| SwitchStrategy::Depth { depth }),
            (0u64..50_000).prop_map(|tokens| SwitchStrategy::TokenBudget { tokens }),
            (0u32..6, 0u64..50_000)
                .prop_map(|(depth, tokens)| SwitchStrategy::Hybrid { depth, tokens }),
            any::<bool>().prop_map(|reasoning_only| SwitchStrategy::QueryType { reasoning_only }),
            (
                prop::option::of(0u32..6),
                prop::option::of(0u64..50_000),
                proptest::collection::vec(query_type_strategy(), 0..4),
                proptest::collection::vec(query_type_strategy(), 0..4)
            )
                .prop_map(
                    |(max_root_depth, max_root_tokens, force_recursive_for, force_root_for)| {
                        SwitchStrategy::Custom {
                            max_root_depth,
                            max_root_tokens,
                            force_recursive_for,
                            force_root_for,
                        }
                    }
                ),
            Just(SwitchStrategy::AlwaysRoot),
            Just(SwitchStrategy::AlwaysRecursive),
        ]
    }

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
        assert_eq!(config.recursive_model.id, "claude-3-5-haiku-20241022");
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
        let strategy = SwitchStrategy::Hybrid {
            depth: 2,
            tokens: 10000,
        };

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
        let strategy = SwitchStrategy::QueryType {
            reasoning_only: true,
        };

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
    fn test_switch_strategy_custom_thresholds() {
        let strategy = SwitchStrategy::Custom {
            max_root_depth: Some(2),
            max_root_tokens: Some(1000),
            force_recursive_for: vec![],
            force_root_for: vec![],
        };

        assert!(!strategy.should_use_recursive(1, 500, Some(QueryType::Architecture)));
        assert!(strategy.should_use_recursive(2, 500, Some(QueryType::Architecture)));
        assert!(strategy.should_use_recursive(1, 1000, Some(QueryType::Architecture)));
    }

    #[test]
    fn test_switch_strategy_custom_query_type_overrides() {
        let strategy = SwitchStrategy::Custom {
            max_root_depth: Some(10),
            max_root_tokens: Some(100_000),
            force_recursive_for: vec![QueryType::Extraction],
            force_root_for: vec![QueryType::Architecture],
        };

        assert!(strategy.should_use_recursive(0, 0, Some(QueryType::Extraction)));
        assert!(!strategy.should_use_recursive(99, 999_999, Some(QueryType::Architecture)));
    }

    #[test]
    fn test_route_rlm_with_custom_switch_strategy() {
        let router = SmartRouter::new();
        let config = DualModelConfig::new(ModelSpec::claude_opus(), ModelSpec::claude_haiku())
            .with_custom_strategy(
                Some(3),
                Some(2_000),
                vec![QueryType::Extraction],
                vec![QueryType::Architecture],
            );

        let context = RoutingContext::new().with_depth(0);
        let root_decision = router.route_rlm("Design the architecture", &context, &config, 0);
        assert_eq!(root_decision.model.id, config.root_model.id);

        let recursive_decision = router.route_rlm("Extract entities", &context, &config, 0);
        assert_eq!(recursive_decision.model.id, config.recursive_model.id);
    }

    #[test]
    fn test_dual_model_select_model() {
        let config = DualModelConfig::aggressive();

        // Depth 0: root model
        let model = config.select_model(0, 0, None);
        assert_eq!(model.id, "claude-3-opus-20240229");

        // Depth 1+: recursive model
        let model = config.select_model(1, 0, None);
        assert_eq!(model.id, "claude-3-5-haiku-20241022");
    }

    #[test]
    fn test_dual_model_extraction_model_defaults_to_recursive() {
        let config = DualModelConfig::new(ModelSpec::claude_opus(), ModelSpec::claude_haiku());
        assert_eq!(config.extraction_model().id, "claude-3-5-haiku-20241022");
    }

    #[test]
    fn test_dual_model_select_model_for_extraction_tier() {
        let config = DualModelConfig::quality_first();
        let model = config.select_model_for_tier(
            10,
            200_000,
            Some(QueryType::Extraction),
            ModelCallTier::Extraction,
        );
        assert_eq!(model.id, "claude-3-5-sonnet-20241022");
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
        assert_eq!(decision.model.id, "claude-3-5-haiku-20241022");
        assert!(decision.reason.contains("recursive"));
    }

    #[test]
    fn test_route_rlm_for_extraction_tier() {
        let router = SmartRouter::new();
        let config = DualModelConfig::aggressive();
        let context = RoutingContext::new().with_depth(4);

        let decision = router.route_rlm_for_tier(
            "Extract final answer from history",
            &context,
            &config,
            50_000,
            ModelCallTier::Extraction,
        );
        assert_eq!(decision.model.id, "claude-3-5-haiku-20241022");
        assert!(decision.reason.contains("extraction"));
    }

    #[test]
    fn test_route_rlm_tiered_cost_accounting() {
        let router = SmartRouter::new();
        let config = DualModelConfig::balanced();
        let mut tracker = crate::llm::CostTracker::new();

        let root_ctx = RoutingContext::new().with_depth(0);
        let root = router.route_rlm("Design architecture", &root_ctx, &config, 0);
        tracker.record_tiered(
            &root.model.id,
            &crate::llm::TokenUsage {
                input_tokens: 1000,
                output_tokens: 400,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            },
            Some(0.025),
            ModelCallTier::Root,
        );

        let recursive_ctx = RoutingContext::new().with_depth(3);
        let recursive = router.route_rlm("Extract entities", &recursive_ctx, &config, 1400);
        tracker.record_tiered(
            &recursive.model.id,
            &crate::llm::TokenUsage {
                input_tokens: 600,
                output_tokens: 220,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            },
            Some(0.003),
            ModelCallTier::Recursive,
        );

        let extraction = router.route_rlm_for_tier(
            "Extract final answer",
            &recursive_ctx,
            &config,
            2020,
            ModelCallTier::Extraction,
        );
        tracker.record_tiered(
            &extraction.model.id,
            &crate::llm::TokenUsage {
                input_tokens: 350,
                output_tokens: 100,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            },
            Some(0.0015),
            ModelCallTier::Extraction,
        );

        let breakdown = tracker.tier_breakdown();
        assert_eq!(breakdown.root_requests, 1);
        assert_eq!(breakdown.recursive_requests, 1);
        assert_eq!(breakdown.extraction_requests, 1);
        assert!(breakdown.total_cost > 0.0);
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

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]

        #[test]
        fn prop_depth_switch_strategy_is_monotonic(
            switch_depth in 0u32..10,
            depth_a in 0u32..16,
            depth_b in 0u32..16
        ) {
            let strategy = SwitchStrategy::Depth { depth: switch_depth };
            let (low, high) = if depth_a <= depth_b {
                (depth_a, depth_b)
            } else {
                (depth_b, depth_a)
            };

            let low_decision = strategy.should_use_recursive(low, 0, None);
            let high_decision = strategy.should_use_recursive(high, 0, None);
            prop_assert!(!(low_decision && !high_decision));
        }

        #[test]
        fn prop_token_budget_switch_strategy_is_monotonic(
            budget in 0u64..100_000,
            used_a in 0u64..120_000,
            used_b in 0u64..120_000
        ) {
            let strategy = SwitchStrategy::TokenBudget { tokens: budget };
            let (low, high) = if used_a <= used_b {
                (used_a, used_b)
            } else {
                (used_b, used_a)
            };

            let low_decision = strategy.should_use_recursive(0, low, None);
            let high_decision = strategy.should_use_recursive(0, high, None);
            prop_assert!(!(low_decision && !high_decision));
        }

        #[test]
        fn prop_dual_model_selection_matches_switch_strategy(
            strategy in switch_strategy_strategy(),
            depth in 0u32..12,
            tokens_used in 0u64..120_000,
            query_type in prop::option::of(query_type_strategy())
        ) {
            let config = DualModelConfig::new(ModelSpec::claude_opus(), ModelSpec::claude_haiku())
                .with_strategy(strategy.clone());

            let selected = config.select_model(depth, tokens_used, query_type);
            let should_recursive = strategy.should_use_recursive(depth, tokens_used, query_type);

            if should_recursive {
                prop_assert_eq!(selected.id.as_str(), config.recursive_model.id.as_str());
            } else {
                prop_assert_eq!(selected.id.as_str(), config.root_model.id.as_str());
            }
        }

        #[test]
        fn prop_custom_force_root_override_wins(
            query_type in query_type_strategy(),
            depth in 0u32..12,
            tokens_used in 0u64..120_000
        ) {
            let strategy = SwitchStrategy::Custom {
                max_root_depth: Some(0),
                max_root_tokens: Some(0),
                force_recursive_for: vec![query_type],
                force_root_for: vec![query_type],
            };

            prop_assert!(!strategy.should_use_recursive(depth, tokens_used, Some(query_type)));
        }

        #[test]
        fn prop_tiered_cost_accounting_request_counts_are_exact(
            root_calls in 0u8..6,
            recursive_calls in 0u8..6,
            extraction_calls in 0u8..6
        ) {
            let mut tracker = crate::llm::CostTracker::new();
            let usage = crate::llm::TokenUsage {
                input_tokens: 120,
                output_tokens: 40,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            };

            for _ in 0..root_calls {
                tracker.record_tiered("root", &usage, Some(0.01), ModelCallTier::Root);
            }
            for _ in 0..recursive_calls {
                tracker.record_tiered("recursive", &usage, Some(0.002), ModelCallTier::Recursive);
            }
            for _ in 0..extraction_calls {
                tracker.record_tiered("extraction", &usage, Some(0.001), ModelCallTier::Extraction);
            }

            let breakdown = tracker.tier_breakdown();
            prop_assert_eq!(breakdown.root_requests, root_calls as u64);
            prop_assert_eq!(breakdown.recursive_requests, recursive_calls as u64);
            prop_assert_eq!(breakdown.extraction_requests, extraction_calls as u64);
            prop_assert!(breakdown.total_cost >= 0.0);
        }
    }
}
