//! Smart router for query-aware model selection.
//!
//! Routes queries to appropriate models based on:
//! - Query type (architecture, multi-file, debugging, extraction, simple)
//! - Recursion depth (deeper calls use cheaper models)
//! - Budget constraints
//! - Provider availability

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
}
