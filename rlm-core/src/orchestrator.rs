//! Orchestrator trait and related types for RLM execution.
//!
//! The orchestrator is the central component that manages the RLM loop:
//! 1. EXTERNALIZE: Store context as manipulable variables
//! 2. ANALYZE: Assess complexity, select strategy
//! 3. DECOMPOSE: Partition context if needed
//! 4. EXECUTE: Run code in REPL, make sub-calls
//! 5. SYNTHESIZE: Combine results into final answer

use crate::complexity::{ActivationDecision, TaskComplexitySignals};
use crate::context::SessionContext;
use crate::error::Result;
use crate::trajectory::TrajectoryEvent;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Result of a recursive RLM sub-call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecursiveResult {
    /// The answer/result from the sub-call
    pub content: String,
    /// Depth at which this was executed
    pub depth: u32,
    /// Whether a REPL was spawned
    pub used_repl: bool,
    /// Token usage for this call
    pub tokens_used: u64,
    /// Cost in USD
    pub cost_usd: f64,
}

/// Configuration for the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum recursion depth (default: 3)
    pub max_depth: u32,
    /// Whether to spawn REPL by default
    pub default_spawn_repl: bool,
    /// Timeout for REPL execution in milliseconds
    pub repl_timeout_ms: u64,
    /// Maximum tokens per recursive call
    pub max_tokens_per_call: u64,
    /// Total token budget for execution
    pub total_token_budget: u64,
    /// Total cost budget in USD
    pub cost_budget_usd: f64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            default_spawn_repl: true,
            repl_timeout_ms: 30_000,
            max_tokens_per_call: 4096,
            total_token_budget: 100_000,
            cost_budget_usd: 1.0,
        }
    }
}

/// Execution mode for the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Micro mode: minimal cost, REPL-only ($0.01)
    Micro,
    /// Fast mode: quick responses ($0.05)
    Fast,
    /// Balanced mode: default for complex tasks ($0.25)
    Balanced,
    /// Thorough mode: deep analysis ($1.00)
    Thorough,
}

impl ExecutionMode {
    /// Get the typical cost budget for this mode.
    pub fn typical_budget_usd(&self) -> f64 {
        match self {
            Self::Micro => 0.01,
            Self::Fast => 0.05,
            Self::Balanced => 0.25,
            Self::Thorough => 1.00,
        }
    }

    /// Get the max recursion depth for this mode.
    pub fn max_depth(&self) -> u32 {
        match self {
            Self::Micro => 1,
            Self::Fast => 2,
            Self::Balanced => 3,
            Self::Thorough => 5,
        }
    }

    /// Select mode based on complexity signals.
    pub fn from_signals(signals: &TaskComplexitySignals) -> Self {
        if signals.user_wants_fast {
            return Self::Fast;
        }

        if signals.user_wants_thorough
            || signals.architecture_analysis
            || signals.requires_exhaustive_search
        {
            return Self::Thorough;
        }

        if signals.has_strong_signal() {
            return Self::Balanced;
        }

        let score = signals.score();
        if score >= 5 {
            Self::Balanced
        } else if score >= 2 {
            Self::Fast
        } else {
            Self::Micro
        }
    }
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Micro => write!(f, "micro"),
            Self::Fast => write!(f, "fast"),
            Self::Balanced => write!(f, "balanced"),
            Self::Thorough => write!(f, "thorough"),
        }
    }
}

/// A boxed stream of trajectory events.
pub type TrajectoryStream = Pin<Box<dyn Stream<Item = TrajectoryEvent> + Send>>;

/// The core orchestrator trait that defines the RLM interface.
///
/// Implementations must be thread-safe (`Send + Sync`) to allow
/// concurrent usage across async tasks.
#[async_trait]
pub trait Orchestrator: Send + Sync {
    /// Determine if RLM should activate for this query.
    ///
    /// This is a synchronous check that examines the query and context
    /// to make an activation decision without any I/O.
    fn should_activate(&self, query: &str, context: &SessionContext) -> ActivationDecision;

    /// Run the main orchestration loop.
    ///
    /// Returns a stream of trajectory events that can be consumed
    /// to observe execution progress. The stream completes when
    /// orchestration finishes (either with a final answer or error).
    async fn run(&self, query: &str, context: &SessionContext) -> Result<TrajectoryStream>;

    /// Execute a recursive sub-call.
    ///
    /// This is used internally during orchestration to make nested
    /// LLM calls with managed depth and REPL access.
    ///
    /// # Arguments
    /// * `query` - The sub-query to execute
    /// * `context` - Additional context for the sub-call
    /// * `depth` - Current recursion depth
    /// * `spawn_repl` - Whether to spawn a REPL for this call
    async fn recursive_call(
        &self,
        query: &str,
        context: &str,
        depth: u32,
        spawn_repl: bool,
    ) -> Result<RecursiveResult>;

    /// Get the current execution mode.
    fn execution_mode(&self) -> ExecutionMode;

    /// Set the execution mode.
    fn set_execution_mode(&mut self, mode: ExecutionMode);

    /// Get the orchestrator configuration.
    fn config(&self) -> &OrchestratorConfig;
}

/// Builder for creating orchestrator instances with custom configuration.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorBuilder {
    config: OrchestratorConfig,
    mode: Option<ExecutionMode>,
}

impl OrchestratorBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum recursion depth.
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Set whether to spawn REPL by default.
    pub fn default_spawn_repl(mut self, spawn: bool) -> Self {
        self.config.default_spawn_repl = spawn;
        self
    }

    /// Set the REPL timeout in milliseconds.
    pub fn repl_timeout_ms(mut self, timeout: u64) -> Self {
        self.config.repl_timeout_ms = timeout;
        self
    }

    /// Set the total token budget.
    pub fn total_token_budget(mut self, budget: u64) -> Self {
        self.config.total_token_budget = budget;
        self
    }

    /// Set the cost budget in USD.
    pub fn cost_budget_usd(mut self, budget: f64) -> Self {
        self.config.cost_budget_usd = budget;
        self
    }

    /// Set the execution mode.
    pub fn execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Build the configuration.
    pub fn build_config(self) -> OrchestratorConfig {
        self.config
    }

    /// Get the execution mode, or default based on config.
    pub fn get_mode(&self) -> ExecutionMode {
        self.mode.unwrap_or(ExecutionMode::Balanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::TaskComplexitySignals;

    #[test]
    fn test_execution_mode_from_signals() {
        let mut signals = TaskComplexitySignals::default();
        assert_eq!(ExecutionMode::from_signals(&signals), ExecutionMode::Micro);

        signals.debugging_task = true;
        assert_eq!(ExecutionMode::from_signals(&signals), ExecutionMode::Fast);

        signals.architecture_analysis = true;
        assert_eq!(
            ExecutionMode::from_signals(&signals),
            ExecutionMode::Thorough
        );
    }

    #[test]
    fn test_execution_mode_user_override() {
        let mut signals = TaskComplexitySignals::default();
        signals.architecture_analysis = true;
        signals.user_wants_fast = true;

        // User intent overrides complexity
        assert_eq!(ExecutionMode::from_signals(&signals), ExecutionMode::Fast);
    }

    #[test]
    fn test_config_defaults() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_depth, 3);
        assert!(config.default_spawn_repl);
        assert_eq!(config.repl_timeout_ms, 30_000);
    }

    #[test]
    fn test_builder() {
        let config = OrchestratorBuilder::new()
            .max_depth(5)
            .cost_budget_usd(2.0)
            .build_config();

        assert_eq!(config.max_depth, 5);
        assert_eq!(config.cost_budget_usd, 2.0);
    }

    #[test]
    fn test_mode_budgets() {
        assert!(ExecutionMode::Micro.typical_budget_usd() < ExecutionMode::Fast.typical_budget_usd());
        assert!(
            ExecutionMode::Fast.typical_budget_usd() < ExecutionMode::Balanced.typical_budget_usd()
        );
        assert!(
            ExecutionMode::Balanced.typical_budget_usd()
                < ExecutionMode::Thorough.typical_budget_usd()
        );
    }
}
