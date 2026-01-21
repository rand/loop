# SPEC-21: Dual-Model Optimization Strategy

> Cost-optimized model selection for RLM orchestration

**Status**: Draft
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-z6x

---

## Overview

Implement explicit dual-model configuration for RLM orchestration, enabling 30-50% cost savings by using premium models for root orchestration and budget models for recursive sub-queries.

## Background

Codecrack3's RLM-DSPy demonstrates significant cost savings:
- Premium model (Opus) for root orchestration decisions
- Budget model (Haiku) for recursive sub-queries and extraction
- Reported 33% better performance than GPT-5 at similar cost

## Requirements

### SPEC-21.01: DualModelConfig

Configuration for dual-model strategy.

```rust
/// Configuration for dual-model RLM execution
#[derive(Debug, Clone)]
pub struct DualModelConfig {
    /// Premium model for root orchestration
    pub root_model: ModelSpec,
    /// Budget model for recursive sub-queries
    pub recursive_model: ModelSpec,
    /// Strategy for switching between models
    pub switch_strategy: SwitchStrategy,
    /// Optional override for extraction model
    pub extraction_model: Option<ModelSpec>,
}

/// Strategy for deciding when to switch from root to recursive model
#[derive(Debug, Clone)]
pub enum SwitchStrategy {
    /// Switch at specified recursion depth
    Depth(u32),
    /// Switch after token budget consumed
    TokenBudget(u64),
    /// Switch based on query classification
    QueryType {
        /// Use root only for reasoning tasks
        reasoning_only: bool
    },
    /// Hybrid: switch at depth AND after tokens
    Hybrid {
        depth: u32,
        tokens: u64
    },
    /// Custom: user-provided function
    Custom(Arc<dyn Fn(&RoutingContext) -> bool + Send + Sync>),
}
```

**Acceptance Criteria**:
- [ ] DualModelConfig serializable to/from JSON
- [ ] SwitchStrategy covers common use cases
- [ ] Custom strategy allows user flexibility

### SPEC-21.02: SmartRouter Integration

Integration with existing SmartRouter.

```rust
impl SmartRouter {
    /// Route RLM query using dual-model config
    pub fn route_rlm(
        &self,
        query: &str,
        context: &RoutingContext,
        config: &DualModelConfig,
    ) -> ModelSpec {
        let use_root = match &config.switch_strategy {
            SwitchStrategy::Depth(d) => context.depth < *d,
            SwitchStrategy::TokenBudget(t) => context.tokens_used < *t,
            SwitchStrategy::QueryType { reasoning_only } => {
                !reasoning_only || self.classify_query(query).is_reasoning()
            }
            SwitchStrategy::Hybrid { depth, tokens } => {
                context.depth < *depth && context.tokens_used < *tokens
            }
            SwitchStrategy::Custom(f) => f(context),
        };

        if use_root {
            config.root_model.clone()
        } else {
            config.recursive_model.clone()
        }
    }
}

/// Context for routing decisions
#[derive(Debug, Clone)]
pub struct RoutingContext {
    pub depth: u32,
    pub tokens_used: u64,
    pub query_type: Option<QueryType>,
    pub parent_model: Option<ModelSpec>,
    pub budget_remaining: Option<f64>,
}
```

**Acceptance Criteria**:
- [ ] route_rlm() correctly applies switch strategy
- [ ] Fallback to single-model when config not provided
- [ ] RoutingContext captures all relevant state

### SPEC-21.03: Cost Tracking Integration

Detailed cost tracking by model tier.

```rust
impl CostTracker {
    /// Record usage with tier annotation
    pub fn record_tiered(
        &mut self,
        model: &str,
        tier: ModelTier,
        tokens: TokenUsage,
        cost_usd: f64,
    );

    /// Get cost breakdown by tier
    pub fn cost_by_tier(&self) -> HashMap<ModelTier, TierCost>;

    /// Get cost report with tier breakdown
    pub fn tiered_report(&self) -> TieredCostReport;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelTier {
    Root,       // Premium orchestration model
    Recursive,  // Budget recursive model
    Extraction, // Extraction/fallback model
}

#[derive(Debug, Clone)]
pub struct TierCost {
    pub tier: ModelTier,
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone)]
pub struct TieredCostReport {
    pub tiers: Vec<TierCost>,
    pub total_cost_usd: f64,
    pub savings_vs_root_only: f64,  // Estimated savings
}
```

**Acceptance Criteria**:
- [ ] Cost tracked separately by tier
- [ ] Savings calculation accurate
- [ ] Report includes tier breakdown

### SPEC-21.04: Default Configurations

Pre-built configurations for common use cases.

```rust
impl DualModelConfig {
    /// Aggressive cost savings: switch at depth 1
    pub fn aggressive() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth(1),
            extraction_model: Some(ModelSpec::claude_haiku()),
        }
    }

    /// Balanced: switch at depth 2
    pub fn balanced() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_sonnet(),
            switch_strategy: SwitchStrategy::Depth(2),
            extraction_model: Some(ModelSpec::claude_haiku()),
        }
    }

    /// Quality first: switch at depth 3
    pub fn quality_first() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_sonnet(),
            switch_strategy: SwitchStrategy::Depth(3),
            extraction_model: Some(ModelSpec::claude_sonnet()),
        }
    }

    /// Budget constrained: hybrid strategy
    pub fn budget_constrained(max_usd: f64) -> Self {
        let token_budget = (max_usd / 0.00001) as u64; // Approximate
        Self {
            root_model: ModelSpec::claude_sonnet(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Hybrid {
                depth: 1,
                tokens: token_budget,
            },
            extraction_model: Some(ModelSpec::claude_haiku()),
        }
    }
}
```

**Expected Cost Savings**:

| Config | Root Model | Recursive | Est. Savings |
|--------|------------|-----------|--------------|
| Aggressive | Opus | Haiku | 40-50% |
| Balanced | Opus | Sonnet | 25-35% |
| Quality First | Opus | Sonnet | 15-25% |
| Budget Constrained | Sonnet | Haiku | 50-60% |

**Acceptance Criteria**:
- [ ] All default configs compile and work
- [ ] Documented expected savings
- [ ] Easy to customize from defaults

---

## Integration Points

### Orchestrator Integration

```rust
impl<O: Orchestrator> O {
    pub fn with_dual_model(self, config: DualModelConfig) -> Self;
    pub fn dual_model_config(&self) -> Option<&DualModelConfig>;
}
```

### ExecutionMode Integration

```rust
impl ExecutionMode {
    pub fn default_dual_model_config(&self) -> DualModelConfig {
        match self {
            ExecutionMode::Micro => DualModelConfig::aggressive(),
            ExecutionMode::Fast => DualModelConfig::aggressive(),
            ExecutionMode::Balanced => DualModelConfig::balanced(),
            ExecutionMode::Thorough => DualModelConfig::quality_first(),
        }
    }
}
```

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_switch_depth` | Switch at correct depth | SPEC-21.01 |
| `test_switch_tokens` | Switch after token budget | SPEC-21.01 |
| `test_switch_hybrid` | Hybrid strategy | SPEC-21.01 |
| `test_route_rlm` | SmartRouter integration | SPEC-21.02 |
| `test_cost_by_tier` | Tier cost tracking | SPEC-21.03 |
| `test_savings_calculation` | Savings estimate | SPEC-21.03 |
| `test_default_aggressive` | Aggressive config | SPEC-21.04 |
| `test_default_balanced` | Balanced config | SPEC-21.04 |

---

## References

- [Codecrack3 RLM-DSPy](https://github.com/codecrack3/Recursive-Language-Models-RLM-with-DSpy)
- Existing SmartRouter: `src/llm/router.rs`
- Existing CostTracker: `src/llm/cost.rs`
