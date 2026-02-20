# SPEC-21: Dual-Model Optimization Strategy

> Cost-optimized model selection for RLM orchestration

**Status**: Partially implemented (router and orchestrator-boundary dual-model routing are implemented; remaining dual-model strategy refinements are tracked in `loop-azq`)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-z6x

---

## Overview

Implement explicit dual-model configuration for RLM orchestration, enabling 30-50% cost savings by using premium models for root orchestration and budget models for recursive sub-queries.

## Implementation Snapshot (2026-02-20)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-21.01 DualModelConfig | Implemented (without custom function strategy) | `rlm-core/src/llm/router.rs` |
| SPEC-21.02 SmartRouter integration | Implemented | `SmartRouter::route_rlm`, `route_rlm_for_tier`, `route_with_config` in `rlm-core/src/llm/router.rs` |
| SPEC-21.03 Tiered cost tracking | Implemented | `CostTracker::record_tiered` + `TierBreakdown` in `rlm-core/src/llm/types.rs` |
| SPEC-21.04 Default configurations | Implemented | `DualModelConfig::{aggressive,balanced,quality_first,budget,token_limited}` |
| Orchestrator mode boundary integration (`M7-T04`) | Implemented | `ExecutionMode::default_dual_model_config`, `OrchestrationRoutingRuntime`, `OrchestratorBuilder::dual_model` in `rlm-core/src/orchestrator.rs` |

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
    Depth { depth: u32 },
    /// Switch after token budget consumed
    TokenBudget { tokens: u64 },
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
    /// Always stay on root model
    AlwaysRoot,
    /// Always stay on recursive model
    AlwaysRecursive,
}
```

**Acceptance Criteria**:
- [ ] DualModelConfig serializable to/from JSON
- [ ] SwitchStrategy covers common use cases
- [ ] Custom strategy allows user flexibility (currently deferred)

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
        tokens_used: u64,
    ) -> RoutingDecision {
        let use_root = match &config.switch_strategy {
            SwitchStrategy::Depth { depth } => context.depth < *depth,
            SwitchStrategy::TokenBudget { tokens } => tokens_used < *tokens,
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
    pub max_depth: u32,
    pub remaining_budget: Option<f64>,
    pub preferred_provider: Option<Provider>,
    pub require_caching: bool,
    pub require_vision: bool,
    pub require_tools: bool,
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
        tier: ModelCallTier,
        tokens: TokenUsage,
        cost_usd: f64,
    );

    /// Get cost report with tier breakdown
    pub fn tier_breakdown(&self) -> TierBreakdown;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelCallTier {
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
pub struct TierBreakdown {
    pub root_cost: f64,
    pub recursive_cost: f64,
    pub extraction_cost: f64,
    pub total_cost: f64,
    pub estimated_single_model_cost: f64,
    pub savings_percentage: f64,
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
            switch_strategy: SwitchStrategy::Depth { depth: 1 },
            extraction_model: Some(ModelSpec::claude_haiku()),
        }
    }

    /// Balanced: switch at depth 2
    pub fn balanced() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth { depth: 2 },
            extraction_model: Some(ModelSpec::claude_haiku()),
        }
    }

    /// Quality first: switch at depth 3
    pub fn quality_first() -> Self {
        Self {
            root_model: ModelSpec::claude_opus(),
            recursive_model: ModelSpec::claude_sonnet(),
            switch_strategy: SwitchStrategy::Depth { depth: 3 },
            extraction_model: Some(ModelSpec::claude_sonnet()),
        }
    }

    /// Budget-focused profile
    pub fn budget() -> Self {
        Self {
            root_model: ModelSpec::claude_sonnet(),
            recursive_model: ModelSpec::claude_haiku(),
            switch_strategy: SwitchStrategy::Depth { depth: 1 },
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
impl OrchestratorBuilder {
    pub fn dual_model(self, config: DualModelConfig) -> Self;
}

pub struct OrchestrationRoutingRuntime { /* ... */ }
impl OrchestrationRoutingRuntime {
    pub fn for_mode(mode: ExecutionMode) -> Self;
    pub fn route_recursive(&self, query: &str, depth: u32) -> (RoutingDecision, ModelCallTier);
    pub fn route_extraction(&self, query: &str, depth: u32) -> (RoutingDecision, ModelCallTier);
    pub fn record_usage(&mut self, decision: &RoutingDecision, usage: &TokenUsage, cost: Option<f64>, tier: ModelCallTier);
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
| `llm::router::tests::test_switch_strategy_depth` | Switch at correct depth | SPEC-21.01 |
| `llm::router::tests::test_switch_strategy_token_budget` | Switch after token budget | SPEC-21.01 |
| `llm::router::tests::test_switch_strategy_hybrid` | Hybrid strategy | SPEC-21.01 |
| `llm::router::tests::test_route_rlm` + `test_route_rlm_for_extraction_tier` | SmartRouter integration | SPEC-21.02 |
| `llm::router::tests::test_route_rlm_tiered_cost_accounting` + `llm::types::tests::test_cost_tracker_record_tiered_includes_extraction` | Tier cost tracking | SPEC-21.03 |
| `orchestrator::tests::test_orchestration_routing_runtime_tracks_root_recursive_extraction` | Orchestrator boundary routing + accounting integration | SPEC-21.02, SPEC-21.03 |
| `llm::router::tests::test_dual_model_config_aggressive` | Aggressive config | SPEC-21.04 |
| `llm::router::tests::test_dual_model_config_balanced` | Balanced config | SPEC-21.04 |

---

## References

- [Codecrack3 RLM-DSPy](https://github.com/codecrack3/Recursive-Language-Models-RLM-with-DSpy)
- Existing SmartRouter: `src/llm/router.rs`
- Existing CostTracker: `src/llm/cost.rs`
