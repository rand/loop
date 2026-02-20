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
use crate::llm::{
    CostTracker, DualModelConfig, ModelCallTier, RoutingContext, RoutingDecision, SmartRouter,
    TokenUsage,
};
use crate::signature::{
    ExecutionLimits, ExecutionResult, FallbackExtractor, FallbackTrigger, ReplHistory, Signature,
    SubmitResult,
};
use crate::trajectory::TrajectoryEvent;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Instant;

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
    /// Optional dual-model routing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dual_model: Option<DualModelConfig>,
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
            dual_model: None,
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

    /// Get the default dual-model configuration for this execution mode.
    pub fn default_dual_model_config(&self) -> DualModelConfig {
        match self {
            Self::Micro | Self::Fast => DualModelConfig::aggressive(),
            Self::Balanced => DualModelConfig::balanced(),
            Self::Thorough => DualModelConfig::quality_first(),
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

    /// Set dual-model configuration explicitly.
    pub fn dual_model(mut self, dual_model: DualModelConfig) -> Self {
        self.config.dual_model = Some(dual_model);
        self
    }

    /// Set the execution mode.
    pub fn execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Build the configuration.
    pub fn build_config(self) -> OrchestratorConfig {
        let mut config = self.config;
        if config.dual_model.is_none() {
            if let Some(mode) = self.mode {
                config.dual_model = Some(mode.default_dual_model_config());
            }
        }
        config
    }

    /// Get the execution mode, or default based on config.
    pub fn get_mode(&self) -> ExecutionMode {
        self.mode.unwrap_or(ExecutionMode::Balanced)
    }
}

/// Orchestrator-side dual-model routing runtime.
///
/// This bridges `SmartRouter` dual-model decisions into orchestration paths and
/// keeps tiered cost accounting (`root`/`recursive`/`extraction`) in sync with
/// model selection.
pub struct OrchestrationRoutingRuntime {
    router: SmartRouter,
    dual_model: DualModelConfig,
    cost_tracker: CostTracker,
    tokens_used: u64,
}

impl OrchestrationRoutingRuntime {
    /// Create a routing runtime from execution mode defaults.
    pub fn for_mode(mode: ExecutionMode) -> Self {
        Self::new(SmartRouter::new(), mode.default_dual_model_config())
    }

    /// Create a routing runtime with explicit router and dual-model config.
    pub fn new(router: SmartRouter, dual_model: DualModelConfig) -> Self {
        Self {
            router,
            dual_model,
            cost_tracker: CostTracker::new(),
            tokens_used: 0,
        }
    }

    /// Access the active dual-model config.
    pub fn dual_model_config(&self) -> &DualModelConfig {
        &self.dual_model
    }

    /// Route a root/recursive orchestration call at a given depth.
    pub fn route_recursive(&self, query: &str, depth: u32) -> (RoutingDecision, ModelCallTier) {
        let context = RoutingContext::new().with_depth(depth);
        let decision = self
            .router
            .route_rlm(query, &context, &self.dual_model, self.tokens_used);
        let tier = if self.dual_model.is_using_root(depth, self.tokens_used) {
            ModelCallTier::Root
        } else {
            ModelCallTier::Recursive
        };
        (decision, tier)
    }

    /// Route an extraction/fallback call at a given depth.
    pub fn route_extraction(&self, query: &str, depth: u32) -> (RoutingDecision, ModelCallTier) {
        let context = RoutingContext::new().with_depth(depth);
        let decision = self.router.route_rlm_for_tier(
            query,
            &context,
            &self.dual_model,
            self.tokens_used,
            ModelCallTier::Extraction,
        );
        (decision, ModelCallTier::Extraction)
    }

    /// Record token/cost usage for an orchestration call.
    pub fn record_usage(
        &mut self,
        decision: &RoutingDecision,
        usage: &TokenUsage,
        cost: Option<f64>,
        tier: ModelCallTier,
    ) {
        self.cost_tracker
            .record_tiered(&decision.model.id, usage, cost, tier);
        self.tokens_used += usage.input_tokens + usage.output_tokens;
    }

    /// Read current tiered cost tracker state.
    pub fn cost_tracker(&self) -> &CostTracker {
        &self.cost_tracker
    }

    /// Total tokens recorded by this runtime.
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used
    }
}

/// Single execution step consumed by [`FallbackLoop`].
#[derive(Debug, Clone, Default)]
pub struct FallbackLoopStep {
    /// Code executed in this step.
    pub code: String,
    /// Number of LLM calls made during this step.
    pub llm_calls: usize,
    /// Captured stdout from the step.
    pub stdout: String,
    /// Captured stderr from the step.
    pub stderr: String,
    /// Optional SUBMIT result produced by the step.
    pub submit_result: Option<SubmitResult>,
    /// Full variable snapshot after the step.
    pub variables: HashMap<String, Value>,
}

impl FallbackLoopStep {
    /// Create a new step with code content.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            ..Self::default()
        }
    }

    /// Set the number of LLM calls recorded for this step.
    pub fn with_llm_calls(mut self, llm_calls: usize) -> Self {
        self.llm_calls = llm_calls;
        self
    }

    /// Set the captured stdout for this step.
    pub fn with_stdout(mut self, stdout: impl Into<String>) -> Self {
        self.stdout = stdout.into();
        self
    }

    /// Set the captured stderr for this step.
    pub fn with_stderr(mut self, stderr: impl Into<String>) -> Self {
        self.stderr = stderr.into();
        self
    }

    /// Set the SUBMIT result for this step.
    pub fn with_submit_result(mut self, submit_result: SubmitResult) -> Self {
        self.submit_result = Some(submit_result);
        self
    }

    /// Set the variable snapshot for this step.
    pub fn with_variables(mut self, variables: HashMap<String, Value>) -> Self {
        self.variables = variables;
        self
    }
}

/// Minimal fallback-aware execution loop used by orchestrator integrations.
///
/// This wires SPEC-27 fallback trigger checks into an iterative runtime path:
/// - successful `SUBMIT` exits with `ExecutionResult::Submitted`
/// - submit validation failures terminate without fallback extraction
/// - max-iteration / max-llm-call / timeout limits trigger fallback extraction
pub struct FallbackLoop<S: Signature> {
    extractor: FallbackExtractor<S>,
    limits: ExecutionLimits,
}

impl<S: Signature> FallbackLoop<S> {
    /// Create a fallback loop with default extractor configuration.
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            extractor: FallbackExtractor::new(),
            limits,
        }
    }

    /// Create a fallback loop with a custom extractor.
    pub fn with_extractor(limits: ExecutionLimits, extractor: FallbackExtractor<S>) -> Self {
        Self { extractor, limits }
    }

    /// Run the loop until SUBMIT success, fallback extraction, or terminal failure.
    pub fn run<NextStep, ExtractResponse>(
        &self,
        mut next_step: NextStep,
        mut extract_response: ExtractResponse,
    ) -> Result<ExecutionResult<S::Outputs>>
    where
        NextStep: FnMut() -> Result<Option<FallbackLoopStep>>,
        ExtractResponse: FnMut(&str, FallbackTrigger) -> Result<String>,
    {
        let mut history = ReplHistory::new();
        let mut variables = HashMap::new();
        let started = Instant::now();

        loop {
            history.total_time_ms = started.elapsed().as_millis() as u64;
            if let Some(trigger) = self.extractor.should_trigger(&history, &self.limits) {
                return self.extract_with_trigger(
                    &history,
                    &variables,
                    trigger,
                    &mut extract_response,
                );
            }

            let Some(step) = next_step()? else {
                return Ok(ExecutionResult::failed(
                    "Execution ended before SUBMIT and before fallback trigger",
                    FallbackTrigger::Manual,
                ));
            };

            let timestamp_ms = started.elapsed().as_millis() as u64;
            self.record_step(&mut history, &step, timestamp_ms);
            variables = step.variables;

            if let Some(submit_result) = step.submit_result {
                match submit_result {
                    SubmitResult::Success { outputs, .. } => {
                        let parsed = match serde_json::from_value(outputs) {
                            Ok(parsed) => parsed,
                            Err(err) => {
                                return Ok(ExecutionResult::failed(
                                    format!("SUBMIT outputs failed signature decode: {}", err),
                                    FallbackTrigger::Manual,
                                ));
                            }
                        };
                        return Ok(ExecutionResult::submitted(parsed));
                    }
                    SubmitResult::ValidationError { errors, .. } => {
                        let joined = errors
                            .into_iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<_>>()
                            .join("; ");
                        return Ok(ExecutionResult::failed(
                            format!("SUBMIT validation failed: {}", joined),
                            FallbackTrigger::Manual,
                        ));
                    }
                    SubmitResult::NotSubmitted { reason } => {
                        history.add_error(format!("SUBMIT not called: {}", reason), timestamp_ms);
                    }
                }
            }

            history.total_time_ms = started.elapsed().as_millis() as u64;
            if let Some(trigger) = self.extractor.should_trigger(&history, &self.limits) {
                return self.extract_with_trigger(
                    &history,
                    &variables,
                    trigger,
                    &mut extract_response,
                );
            }
        }
    }

    fn record_step(&self, history: &mut ReplHistory, step: &FallbackLoopStep, timestamp_ms: u64) {
        history.add_code(step.code.clone(), timestamp_ms);

        if !step.stdout.trim().is_empty() {
            history.add_output(step.stdout.clone(), timestamp_ms);
        }

        if !step.stderr.trim().is_empty() {
            history.add_error(step.stderr.clone(), timestamp_ms);
        }

        for _ in 0..step.llm_calls {
            history.add_llm_query("[orchestrator llm call]", timestamp_ms);
        }
    }

    fn extract_with_trigger<ExtractResponse>(
        &self,
        history: &ReplHistory,
        variables: &HashMap<String, Value>,
        trigger: FallbackTrigger,
        extract_response: &mut ExtractResponse,
    ) -> Result<ExecutionResult<S::Outputs>>
    where
        ExtractResponse: FnMut(&str, FallbackTrigger) -> Result<String>,
    {
        let prompt = self.extractor.extraction_prompt(history, variables);
        let response = extract_response(&prompt, trigger)?;
        Ok(self.extractor.parse_extraction_response(&response, trigger))
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
        assert!(
            ExecutionMode::Micro.typical_budget_usd() < ExecutionMode::Fast.typical_budget_usd()
        );
        assert!(
            ExecutionMode::Fast.typical_budget_usd() < ExecutionMode::Balanced.typical_budget_usd()
        );
        assert!(
            ExecutionMode::Balanced.typical_budget_usd()
                < ExecutionMode::Thorough.typical_budget_usd()
        );
    }

    #[test]
    fn test_execution_mode_default_dual_model_config() {
        let micro = ExecutionMode::Micro.default_dual_model_config();
        let thorough = ExecutionMode::Thorough.default_dual_model_config();
        assert_eq!(
            micro.switch_strategy,
            crate::llm::SwitchStrategy::Depth { depth: 1 }
        );
        assert_eq!(
            thorough.switch_strategy,
            crate::llm::SwitchStrategy::Depth { depth: 3 }
        );
    }

    #[test]
    fn test_builder_applies_mode_default_dual_model() {
        let config = OrchestratorBuilder::new()
            .execution_mode(ExecutionMode::Fast)
            .build_config();

        let dual = config
            .dual_model
            .expect("expected dual-model config from mode");
        assert_eq!(
            dual.switch_strategy,
            crate::llm::SwitchStrategy::Depth { depth: 1 }
        );
    }

    #[test]
    fn test_orchestration_routing_runtime_tracks_root_recursive_extraction() {
        let mut runtime = OrchestrationRoutingRuntime::for_mode(ExecutionMode::Balanced);

        let (root_decision, root_tier) = runtime.route_recursive("Design system architecture", 0);
        assert_eq!(root_tier, ModelCallTier::Root);
        assert_eq!(
            root_decision.model.id,
            runtime.dual_model_config().root_model.id
        );

        let root_usage = TokenUsage {
            input_tokens: 1200,
            output_tokens: 600,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };
        runtime.record_usage(&root_decision, &root_usage, Some(0.03), root_tier);

        let (recursive_decision, recursive_tier) = runtime.route_recursive("Extract findings", 2);
        assert_eq!(recursive_tier, ModelCallTier::Recursive);
        assert_eq!(
            recursive_decision.model.id,
            runtime.dual_model_config().recursive_model.id
        );

        let recursive_usage = TokenUsage {
            input_tokens: 500,
            output_tokens: 200,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };
        runtime.record_usage(
            &recursive_decision,
            &recursive_usage,
            Some(0.004),
            recursive_tier,
        );

        let (extraction_decision, extraction_tier) =
            runtime.route_extraction("Extract final answer", 2);
        assert_eq!(extraction_tier, ModelCallTier::Extraction);
        assert_eq!(
            extraction_decision.model.id,
            runtime.dual_model_config().extraction_model().id
        );

        let extraction_usage = TokenUsage {
            input_tokens: 300,
            output_tokens: 120,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };
        runtime.record_usage(
            &extraction_decision,
            &extraction_usage,
            Some(0.002),
            extraction_tier,
        );

        assert_eq!(runtime.tokens_used(), 2920);
        let breakdown = runtime.cost_tracker().tier_breakdown();
        assert_eq!(breakdown.root_requests, 1);
        assert_eq!(breakdown.recursive_requests, 1);
        assert_eq!(breakdown.extraction_requests, 1);
    }

    mod fallback {
        use super::*;
        use crate::signature::{FieldSpec, FieldType, SubmitError};
        use serde::{Deserialize, Serialize};
        use serde_json::json;
        use std::collections::VecDeque;

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TestOutputs {
            answer: String,
        }

        struct TestSignature;

        impl Signature for TestSignature {
            type Inputs = ();
            type Outputs = TestOutputs;

            fn instructions() -> &'static str {
                "Test"
            }

            fn input_fields() -> Vec<FieldSpec> {
                Vec::new()
            }

            fn output_fields() -> Vec<FieldSpec> {
                vec![FieldSpec::new("answer", FieldType::String)]
            }
        }

        #[test]
        fn test_submit_success_bypasses_fallback_extraction() {
            let loop_runner = FallbackLoop::<TestSignature>::new(ExecutionLimits::new(1, 1, 1));
            let mut steps =
                VecDeque::from(vec![FallbackLoopStep::new("SUBMIT({'answer': 'done'})")
                    .with_submit_result(SubmitResult::success(json!({"answer": "done"})))]);

            let mut fallback_called = false;
            let result = loop_runner
                .run(
                    || Ok(steps.pop_front()),
                    |_prompt, _trigger| {
                        fallback_called = true;
                        Ok("{\"answer\":\"fallback\",\"_confidence\":0.1}".to_string())
                    },
                )
                .unwrap();

            assert!(result.is_submitted());
            assert_eq!(result.outputs().unwrap().answer, "done");
            assert!(!fallback_called);
        }

        #[test]
        fn test_max_iterations_triggers_fallback_extraction() {
            let loop_runner =
                FallbackLoop::<TestSignature>::new(ExecutionLimits::new(1, 10, 60_000));
            let mut vars = HashMap::new();
            vars.insert("answer".to_string(), json!("from_vars"));

            let mut steps = VecDeque::from(vec![
                FallbackLoopStep::new("x = 'from_vars'").with_variables(vars)
            ]);

            let result = loop_runner
                .run(
                    || Ok(steps.pop_front()),
                    |prompt, trigger| {
                        assert_eq!(trigger, FallbackTrigger::MaxIterations);
                        assert!(prompt.contains("x = 'from_vars'"));
                        assert!(prompt.contains("from_vars"));
                        Ok("{\"answer\":\"from_vars\",\"_confidence\":0.8}".to_string())
                    },
                )
                .unwrap();

            match result {
                ExecutionResult::Extracted { trigger_reason, .. } => {
                    assert_eq!(trigger_reason, FallbackTrigger::MaxIterations);
                }
                other => panic!("expected extracted fallback result, got {:?}", other),
            }
        }

        #[test]
        fn test_max_llm_calls_triggers_fallback_extraction() {
            let loop_runner =
                FallbackLoop::<TestSignature>::new(ExecutionLimits::new(10, 1, 60_000));
            let mut steps = VecDeque::from(vec![
                FallbackLoopStep::new("LLM_QUERY('hello')").with_llm_calls(1)
            ]);

            let result = loop_runner
                .run(
                    || Ok(steps.pop_front()),
                    |_prompt, trigger| {
                        assert_eq!(trigger, FallbackTrigger::MaxLLMCalls);
                        Ok("{\"answer\":\"llm_limit\",\"_confidence\":0.6}".to_string())
                    },
                )
                .unwrap();

            match result {
                ExecutionResult::Extracted { trigger_reason, .. } => {
                    assert_eq!(trigger_reason, FallbackTrigger::MaxLLMCalls);
                }
                other => panic!("expected extracted fallback result, got {:?}", other),
            }
        }

        #[test]
        fn test_timeout_triggers_fallback_before_step_execution() {
            let loop_runner = FallbackLoop::<TestSignature>::new(ExecutionLimits::new(10, 10, 0));

            let result = loop_runner
                .run(
                    || panic!("timeout-triggered fallback should not request a step"),
                    |_prompt, trigger| {
                        assert_eq!(trigger, FallbackTrigger::Timeout);
                        Ok("{\"answer\":\"timeout\",\"_confidence\":0.7}".to_string())
                    },
                )
                .unwrap();

            match result {
                ExecutionResult::Extracted { trigger_reason, .. } => {
                    assert_eq!(trigger_reason, FallbackTrigger::Timeout);
                }
                other => panic!("expected extracted fallback result, got {:?}", other),
            }
        }

        #[test]
        fn test_submit_validation_error_is_terminal_without_fallback() {
            let loop_runner =
                FallbackLoop::<TestSignature>::new(ExecutionLimits::new(10, 10, 60_000));
            let mut steps = VecDeque::from(vec![FallbackLoopStep::new("SUBMIT({})")
                .with_submit_result(SubmitResult::validation_error(vec![
                    SubmitError::NoSignatureRegistered,
                ]))]);

            let mut fallback_called = false;
            let result = loop_runner
                .run(
                    || Ok(steps.pop_front()),
                    |_prompt, _trigger| {
                        fallback_called = true;
                        Ok("{\"answer\":\"unexpected\",\"_confidence\":0.2}".to_string())
                    },
                )
                .unwrap();

            assert!(result.is_failed());
            assert!(!fallback_called);
        }
    }
}
