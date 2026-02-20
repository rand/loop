//! Trajectory event types for observable RLM execution.
//!
//! The trajectory system provides a stream of events that can be rendered
//! differently depending on the deployment context:
//! - Claude Code: Streaming text output
//! - TUI: Bubble Tea panel updates
//! - Analysis: JSON export for replay
//!
//! ## Features
//! - Event streaming with verbosity levels
//! - Cost tracking with budget management
//! - Model pricing (Jan 2026)
//! - Burn rate tracking and alerts

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Types of trajectory events emitted during RLM execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrajectoryEventType {
    /// RLM orchestration started
    RlmStart,
    /// Analysis phase (complexity assessment, strategy selection)
    Analyze,
    /// REPL code execution started
    ReplExec,
    /// REPL execution result
    ReplResult,
    /// Reasoning step in the trace
    Reason,
    /// Recursive sub-call started
    RecurseStart,
    /// Recursive sub-call completed
    RecurseEnd,
    /// Final answer/synthesis
    Final,
    /// Error occurred
    Error,
    /// Tool use (external tool, not REPL)
    ToolUse,
    /// Cost report for the operation
    CostReport,
    /// Beginning verification of response/trace (Strawberry)
    VerifyStart,
    /// Atomic claim identified during verification
    ClaimExtracted,
    /// Claim verified against evidence
    EvidenceChecked,
    /// Budget metrics computed (p0, p1, gap)
    BudgetComputed,
    /// Claim flagged as potentially hallucinated
    HallucinationFlag,
    /// Verification report complete
    VerifyComplete,
    /// Memory operation (store, query, evolve)
    Memory,
    /// Context externalization complete
    Externalize,
    /// Decomposition/partitioning step
    Decompose,
    /// Synthesis step
    Synthesize,
    // Adversarial validation events
    /// Adversarial validation started
    AdversarialStart,
    /// Adversarial critic invoked (each strategy/iteration)
    CriticInvoked,
    /// Issue found during adversarial review
    IssueFound,
    /// Adversarial validation complete
    AdversarialComplete,
}

impl std::fmt::Display for TrajectoryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::RlmStart => "RLM_START",
            Self::Analyze => "ANALYZE",
            Self::ReplExec => "REPL_EXEC",
            Self::ReplResult => "REPL_RESULT",
            Self::Reason => "REASON",
            Self::RecurseStart => "RECURSE_START",
            Self::RecurseEnd => "RECURSE_END",
            Self::Final => "FINAL",
            Self::Error => "ERROR",
            Self::ToolUse => "TOOL_USE",
            Self::CostReport => "COST_REPORT",
            Self::VerifyStart => "VERIFY_START",
            Self::ClaimExtracted => "CLAIM_EXTRACTED",
            Self::EvidenceChecked => "EVIDENCE_CHECKED",
            Self::BudgetComputed => "BUDGET_COMPUTED",
            Self::HallucinationFlag => "HALLUCINATION_FLAG",
            Self::VerifyComplete => "VERIFY_COMPLETE",
            Self::Memory => "MEMORY",
            Self::Externalize => "EXTERNALIZE",
            Self::Decompose => "DECOMPOSE",
            Self::Synthesize => "SYNTHESIZE",
            Self::AdversarialStart => "ADVERSARIAL_START",
            Self::CriticInvoked => "CRITIC_INVOKED",
            Self::IssueFound => "ISSUE_FOUND",
            Self::AdversarialComplete => "ADVERSARIAL_COMPLETE",
        };
        write!(f, "{}", s)
    }
}

/// A trajectory event emitted during RLM execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    /// Type of the event
    pub event_type: TrajectoryEventType,
    /// Current recursion depth (0 = top level)
    pub depth: u32,
    /// Human-readable content describing the event
    pub content: String,
    /// Event-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

impl TrajectoryEvent {
    /// Create a new trajectory event.
    pub fn new(event_type: TrajectoryEventType, depth: u32, content: impl Into<String>) -> Self {
        Self {
            event_type,
            depth,
            content: content.into(),
            metadata: None,
            timestamp: Utc::now(),
        }
    }

    /// Add metadata to the event.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Add multiple metadata entries.
    pub fn with_metadata_map(mut self, map: HashMap<String, Value>) -> Self {
        self.metadata = Some(map);
        self
    }

    /// Get a metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.as_ref()?.get(key)
    }

    // Convenience constructors for common event types

    /// Create an RLM start event.
    pub fn rlm_start(query: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RlmStart, 0, query)
    }

    /// Create an analyze event.
    pub fn analyze(depth: u32, analysis: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Analyze, depth, analysis)
    }

    /// Create a REPL execution event.
    pub fn repl_exec(depth: u32, code: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::ReplExec, depth, code)
    }

    /// Create a REPL result event.
    pub fn repl_result(depth: u32, result: impl Into<String>, success: bool) -> Self {
        Self::new(TrajectoryEventType::ReplResult, depth, result).with_metadata("success", success)
    }

    /// Create a reasoning event.
    pub fn reason(depth: u32, reasoning: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Reason, depth, reasoning)
    }

    /// Create a recursive call start event.
    pub fn recurse_start(depth: u32, query: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RecurseStart, depth, query)
    }

    /// Create a recursive call end event.
    pub fn recurse_end(depth: u32, result: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RecurseEnd, depth, result)
    }

    /// Create a final answer event.
    pub fn final_answer(depth: u32, answer: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Final, depth, answer)
    }

    /// Create an error event.
    pub fn error(depth: u32, error: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Error, depth, error)
    }

    /// Create a cost report event.
    pub fn cost_report(cost: &CostSummary) -> Self {
        Self::new(TrajectoryEventType::CostReport, 0, cost.to_string())
            .with_metadata("input_tokens", cost.input_tokens as i64)
            .with_metadata("output_tokens", cost.output_tokens as i64)
            .with_metadata("total_cost_usd", cost.total_cost_usd)
    }

    /// Create a hallucination flag event.
    pub fn hallucination_flag(
        depth: u32,
        claim: impl Into<String>,
        budget_gap: f64,
        status: impl Into<String>,
    ) -> Self {
        Self::new(TrajectoryEventType::HallucinationFlag, depth, claim)
            .with_metadata("budget_gap", budget_gap)
            .with_metadata("status", status.into())
    }

    /// Create an adversarial validation start event.
    pub fn adversarial_start(
        depth: u32,
        validation_id: impl Into<String>,
        strategies: &[String],
    ) -> Self {
        Self::new(
            TrajectoryEventType::AdversarialStart,
            depth,
            format!("Starting adversarial validation {}", validation_id.into()),
        )
        .with_metadata("strategies", serde_json::json!(strategies))
    }

    /// Create a critic invoked event.
    pub fn critic_invoked(depth: u32, strategy: impl Into<String>, iteration: usize) -> Self {
        let strategy = strategy.into();
        Self::new(
            TrajectoryEventType::CriticInvoked,
            depth,
            format!("Invoking {} critic (iteration {})", strategy, iteration),
        )
        .with_metadata("strategy", strategy)
        .with_metadata("iteration", iteration as i64)
    }

    /// Create an issue found event.
    pub fn issue_found(
        depth: u32,
        severity: impl Into<String>,
        category: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        let severity = severity.into();
        let category = category.into();
        let title = title.into();
        Self::new(
            TrajectoryEventType::IssueFound,
            depth,
            format!("[{}] {}: {}", severity, category, title),
        )
        .with_metadata("severity", severity)
        .with_metadata("category", category)
        .with_metadata("title", title)
    }

    /// Create an adversarial validation complete event.
    pub fn adversarial_complete(
        depth: u32,
        verdict: impl Into<String>,
        issue_count: usize,
        cost_usd: f64,
    ) -> Self {
        let verdict = verdict.into();
        Self::new(
            TrajectoryEventType::AdversarialComplete,
            depth,
            format!(
                "Adversarial validation complete: {} ({} issues, ${:.4})",
                verdict, issue_count, cost_usd
            ),
        )
        .with_metadata("verdict", verdict)
        .with_metadata("issue_count", issue_count as i64)
        .with_metadata("cost_usd", cost_usd)
    }

    /// Check if this is an error event.
    pub fn is_error(&self) -> bool {
        self.event_type == TrajectoryEventType::Error
    }

    /// Check if this is a final answer event.
    pub fn is_final(&self) -> bool {
        self.event_type == TrajectoryEventType::Final
    }

    /// Format as a single-line log entry.
    pub fn as_log_line(&self) -> String {
        let indent = "  ".repeat(self.depth as usize);
        format!(
            "[{}] {}{}: {}",
            self.timestamp.format("%H:%M:%S%.3f"),
            indent,
            self.event_type,
            self.content.lines().next().unwrap_or("")
        )
    }
}

/// Token usage for an LLM call.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

impl TokenUsage {
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        }
    }

    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Cost tracking for a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostComponent {
    Orchestration,
    Repl,
    Recursion,
    Memory,
    Verification,
    Embedding,
}

/// Summary of costs for an RLM execution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostSummary {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost_usd: f64,
    /// Breakdown by component
    pub by_component: HashMap<String, TokenUsage>,
}

impl CostSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add token usage for a component.
    pub fn add(&mut self, component: CostComponent, usage: TokenUsage, cost_usd: f64) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_tokens += usage.cache_creation_tokens;
        self.cache_read_tokens += usage.cache_read_tokens;
        self.total_cost_usd += cost_usd;

        let key = format!("{:?}", component).to_lowercase();
        self.by_component
            .entry(key)
            .and_modify(|existing| {
                existing.input_tokens += usage.input_tokens;
                existing.output_tokens += usage.output_tokens;
                existing.cache_creation_tokens += usage.cache_creation_tokens;
                existing.cache_read_tokens += usage.cache_read_tokens;
            })
            .or_insert(usage);
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl std::fmt::Display for CostSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tokens: {} in / {} out | Cost: ${:.4}",
            self.input_tokens, self.output_tokens, self.total_cost_usd
        )
    }
}

/// Export format for trajectory data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON Lines format (one event per line)
    JsonLines,
    /// Pretty-printed JSON array
    JsonPretty,
    /// Compact JSON array
    JsonCompact,
    /// Markdown summary
    Markdown,
}

/// Serialize a list of events to the specified format.
pub fn export_events(events: &[TrajectoryEvent], format: ExportFormat) -> String {
    match format {
        ExportFormat::JsonLines => events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n"),
        ExportFormat::JsonPretty => {
            serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".to_string())
        }
        ExportFormat::JsonCompact => {
            serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string())
        }
        ExportFormat::Markdown => events_to_markdown(events),
    }
}

fn events_to_markdown(events: &[TrajectoryEvent]) -> String {
    let mut md = String::from("# RLM Trajectory\n\n");

    for event in events {
        let indent = "  ".repeat(event.depth as usize);
        md.push_str(&format!(
            "{}**{}** `{}`\n",
            indent, event.event_type, event.timestamp
        ));
        if !event.content.is_empty() {
            md.push_str(&format!("{}> {}\n", indent, event.content));
        }
        md.push('\n');
    }

    md
}

// =============================================================================
// Model Pricing (Jan 2026)
// =============================================================================

/// Model pricing per 1M tokens (USD).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per 1M input tokens
    pub input_per_million: f64,
    /// Cost per 1M output tokens
    pub output_per_million: f64,
    /// Cost per 1M cache creation tokens (if applicable)
    pub cache_creation_per_million: f64,
    /// Cost per 1M cache read tokens (if applicable)
    pub cache_read_per_million: f64,
}

impl ModelPricing {
    /// Create new pricing.
    pub const fn new(input: f64, output: f64) -> Self {
        Self {
            input_per_million: input,
            output_per_million: output,
            cache_creation_per_million: input * 1.25, // 25% premium for cache creation
            cache_read_per_million: input * 0.1,      // 90% discount for cache reads
        }
    }

    /// Calculate cost for token usage.
    pub fn calculate(&self, usage: &TokenUsage) -> f64 {
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        let cache_create_cost =
            (usage.cache_creation_tokens as f64 / 1_000_000.0) * self.cache_creation_per_million;
        let cache_read_cost =
            (usage.cache_read_tokens as f64 / 1_000_000.0) * self.cache_read_per_million;

        input_cost + output_cost + cache_create_cost + cache_read_cost
    }
}

/// Known model pricing table (Jan 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Model {
    ClaudeOpus4,
    ClaudeSonnet4,
    ClaudeHaiku35,
    Gpt4o,
    Gpt4oMini,
    O1,
    O1Mini,
    DeepseekV3,
}

impl Model {
    /// Get pricing for this model.
    pub fn pricing(&self) -> ModelPricing {
        match self {
            // Anthropic (Jan 2026 pricing)
            Self::ClaudeOpus4 => ModelPricing::new(15.0, 75.0),
            Self::ClaudeSonnet4 => ModelPricing::new(3.0, 15.0),
            Self::ClaudeHaiku35 => ModelPricing::new(0.80, 4.0),
            // OpenAI
            Self::Gpt4o => ModelPricing::new(2.50, 10.0),
            Self::Gpt4oMini => ModelPricing::new(0.15, 0.60),
            Self::O1 => ModelPricing::new(15.0, 60.0),
            Self::O1Mini => ModelPricing::new(3.0, 12.0),
            // Open source
            Self::DeepseekV3 => ModelPricing::new(0.27, 1.10),
        }
    }

    /// Calculate cost for this model.
    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        self.pricing().calculate(usage)
    }
}

// =============================================================================
// Budget Management
// =============================================================================

/// Budget alert threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetAlert {
    /// 80% of budget consumed
    Warning,
    /// 100% of budget consumed
    Exceeded,
    /// Custom threshold percentage
    Custom(u8),
}

impl BudgetAlert {
    /// Get threshold as percentage (0-100).
    pub fn threshold_percent(&self) -> u8 {
        match self {
            Self::Warning => 80,
            Self::Exceeded => 100,
            Self::Custom(p) => *p,
        }
    }
}

/// Budget configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum cost in USD (None = unlimited)
    pub max_cost_usd: Option<f64>,
    /// Maximum tokens (None = unlimited)
    pub max_tokens: Option<u64>,
    /// Maximum recursion depth (None = unlimited)
    pub max_depth: Option<u32>,
    /// Alert thresholds to trigger
    pub alert_thresholds: Vec<u8>,
    /// Whether to hard-stop on budget exceeded
    pub hard_limit: bool,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_cost_usd: Some(1.0),   // $1 default
            max_tokens: Some(100_000), // 100k tokens
            max_depth: Some(5),        // 5 recursion levels
            alert_thresholds: vec![80, 100],
            hard_limit: true,
        }
    }
}

impl BudgetConfig {
    /// Unlimited budget (useful for testing).
    pub fn unlimited() -> Self {
        Self {
            max_cost_usd: None,
            max_tokens: None,
            max_depth: None,
            alert_thresholds: vec![],
            hard_limit: false,
        }
    }

    /// Create budget with specific cost limit.
    pub fn with_cost_limit(max_usd: f64) -> Self {
        Self {
            max_cost_usd: Some(max_usd),
            ..Self::default()
        }
    }
}

/// Budget tracking state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetState {
    /// Current accumulated cost
    pub current_cost_usd: f64,
    /// Current accumulated tokens
    pub current_tokens: u64,
    /// Current recursion depth
    pub current_depth: u32,
    /// Alerts that have been triggered
    pub triggered_alerts: Vec<BudgetAlert>,
    /// Start time for burn rate calculation
    pub started_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl Default for BudgetState {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            current_cost_usd: 0.0,
            current_tokens: 0,
            current_depth: 0,
            triggered_alerts: vec![],
            started_at: now,
            updated_at: now,
        }
    }
}

impl BudgetState {
    /// Calculate burn rate in USD per minute.
    pub fn burn_rate_per_minute(&self) -> f64 {
        let elapsed = self.updated_at - self.started_at;
        let minutes = elapsed.num_milliseconds() as f64 / 60_000.0;
        if minutes > 0.0 {
            self.current_cost_usd / minutes
        } else {
            0.0
        }
    }

    /// Estimate time to budget exhaustion.
    pub fn estimated_exhaustion(&self, config: &BudgetConfig) -> Option<Duration> {
        let rate = self.burn_rate_per_minute();
        if rate <= 0.0 {
            return None;
        }

        let remaining = config.max_cost_usd? - self.current_cost_usd;
        if remaining <= 0.0 {
            return Some(Duration::zero());
        }

        let minutes_remaining = remaining / rate;
        Some(Duration::milliseconds(
            (minutes_remaining * 60_000.0) as i64,
        ))
    }

    /// Check budget utilization percentage.
    pub fn utilization_percent(&self, config: &BudgetConfig) -> Option<f64> {
        config
            .max_cost_usd
            .map(|max| (self.current_cost_usd / max) * 100.0)
    }
}

/// Budget manager for tracking and enforcing limits.
#[derive(Debug)]
pub struct BudgetManager {
    config: BudgetConfig,
    state: Arc<RwLock<BudgetState>>,
}

impl BudgetManager {
    /// Create new budget manager.
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(BudgetState::default())),
        }
    }

    /// Record cost and check for alerts.
    pub fn record_cost(&self, cost_usd: f64, tokens: u64) -> Vec<BudgetAlert> {
        let mut state = self.state.write().unwrap();
        state.current_cost_usd += cost_usd;
        state.current_tokens += tokens;
        state.updated_at = Utc::now();

        // Check for new alerts
        let mut new_alerts = vec![];

        if let Some(max) = self.config.max_cost_usd {
            let utilization = (state.current_cost_usd / max) * 100.0;

            for threshold in &self.config.alert_thresholds {
                let alert = if *threshold == 80 {
                    BudgetAlert::Warning
                } else if *threshold >= 100 {
                    BudgetAlert::Exceeded
                } else {
                    BudgetAlert::Custom(*threshold)
                };

                if utilization >= *threshold as f64 && !state.triggered_alerts.contains(&alert) {
                    state.triggered_alerts.push(alert);
                    new_alerts.push(alert);
                }
            }
        }

        new_alerts
    }

    /// Check if budget is exceeded.
    pub fn is_exceeded(&self) -> bool {
        let state = self.state.read().unwrap();

        if let Some(max) = self.config.max_cost_usd {
            if state.current_cost_usd >= max {
                return true;
            }
        }

        if let Some(max) = self.config.max_tokens {
            if state.current_tokens >= max {
                return true;
            }
        }

        false
    }

    /// Check if hard limit should stop execution.
    pub fn should_stop(&self) -> bool {
        self.config.hard_limit && self.is_exceeded()
    }

    /// Get current state snapshot.
    pub fn state(&self) -> BudgetState {
        self.state.read().unwrap().clone()
    }

    /// Get burn rate.
    pub fn burn_rate(&self) -> f64 {
        self.state.read().unwrap().burn_rate_per_minute()
    }

    /// Set recursion depth.
    pub fn set_depth(&self, depth: u32) {
        self.state.write().unwrap().current_depth = depth;
    }

    /// Check depth limit.
    pub fn depth_exceeded(&self) -> bool {
        let state = self.state.read().unwrap();
        self.config
            .max_depth
            .map(|max| state.current_depth >= max)
            .unwrap_or(false)
    }
}

// =============================================================================
// Trajectory Streaming
// =============================================================================

/// Verbosity level for trajectory output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    /// Only errors and final results
    Minimal,
    /// Normal operation events
    Normal,
    /// Include reasoning and intermediate steps
    Verbose,
    /// Full debug output
    Debug,
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Normal
    }
}

impl TrajectoryEventType {
    /// Get minimum verbosity level for this event type.
    pub fn min_verbosity(&self) -> Verbosity {
        match self {
            // Always show
            Self::Error | Self::Final | Self::CostReport => Verbosity::Minimal,
            // Normal operation
            Self::RlmStart
            | Self::Analyze
            | Self::RecurseStart
            | Self::RecurseEnd
            | Self::HallucinationFlag => Verbosity::Normal,
            // Verbose details
            Self::ReplExec
            | Self::ReplResult
            | Self::Reason
            | Self::ToolUse
            | Self::VerifyStart
            | Self::VerifyComplete
            | Self::Decompose
            | Self::Synthesize
            | Self::CriticInvoked => Verbosity::Verbose,
            // Debug only
            Self::ClaimExtracted
            | Self::EvidenceChecked
            | Self::BudgetComputed
            | Self::Memory
            | Self::Externalize => Verbosity::Debug,
            // Adversarial events - show at normal verbosity since they're important
            Self::AdversarialStart | Self::IssueFound | Self::AdversarialComplete => {
                Verbosity::Normal
            }
        }
    }

    /// Check if event should be emitted at given verbosity.
    pub fn should_emit(&self, verbosity: Verbosity) -> bool {
        self.min_verbosity() <= verbosity
    }
}

/// Trait for trajectory event emitters.
pub trait TrajectoryEmitter: Send + Sync {
    /// Emit a trajectory event.
    fn emit(&self, event: TrajectoryEvent);

    /// Emit a budget alert.
    fn emit_alert(&self, alert: BudgetAlert, state: &BudgetState);

    /// Get current verbosity level.
    fn verbosity(&self) -> Verbosity;

    /// Set verbosity level.
    fn set_verbosity(&mut self, verbosity: Verbosity);
}

/// Broadcast-based trajectory emitter.
pub struct BroadcastEmitter {
    sender: broadcast::Sender<TrajectoryEvent>,
    verbosity: Verbosity,
}

impl BroadcastEmitter {
    /// Create new broadcast emitter with channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            verbosity: Verbosity::default(),
        }
    }

    /// Subscribe to trajectory events.
    pub fn subscribe(&self) -> broadcast::Receiver<TrajectoryEvent> {
        self.sender.subscribe()
    }

    /// Get number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl TrajectoryEmitter for BroadcastEmitter {
    fn emit(&self, event: TrajectoryEvent) {
        if event.event_type.should_emit(self.verbosity) {
            let _ = self.sender.send(event);
        }
    }

    fn emit_alert(&self, alert: BudgetAlert, state: &BudgetState) {
        let content = match alert {
            BudgetAlert::Warning => format!(
                "Budget warning: ${:.4} spent ({:.1}% of limit)",
                state.current_cost_usd,
                state
                    .utilization_percent(&BudgetConfig::default())
                    .unwrap_or(0.0)
            ),
            BudgetAlert::Exceeded => {
                format!("Budget exceeded: ${:.4} spent", state.current_cost_usd)
            }
            BudgetAlert::Custom(p) => format!(
                "Budget alert ({}%): ${:.4} spent",
                p, state.current_cost_usd
            ),
        };

        let event = TrajectoryEvent::new(TrajectoryEventType::CostReport, 0, content)
            .with_metadata("alert", format!("{:?}", alert))
            .with_metadata("burn_rate", state.burn_rate_per_minute());

        let _ = self.sender.send(event);
    }

    fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }
}

/// Collecting emitter that stores events in a Vec.
#[derive(Debug, Default)]
pub struct CollectingEmitter {
    events: Arc<RwLock<Vec<TrajectoryEvent>>>,
    verbosity: Verbosity,
}

impl CollectingEmitter {
    /// Create new collecting emitter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get collected events.
    pub fn events(&self) -> Vec<TrajectoryEvent> {
        self.events.read().unwrap().clone()
    }

    /// Clear collected events.
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
}

impl TrajectoryEmitter for CollectingEmitter {
    fn emit(&self, event: TrajectoryEvent) {
        if event.event_type.should_emit(self.verbosity) {
            self.events.write().unwrap().push(event);
        }
    }

    fn emit_alert(&self, alert: BudgetAlert, state: &BudgetState) {
        let event = TrajectoryEvent::new(
            TrajectoryEventType::CostReport,
            0,
            format!("Budget {:?}: ${:.4}", alert, state.current_cost_usd),
        );
        self.events.write().unwrap().push(event);
    }

    fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }
}

/// Null emitter that discards all events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEmitter;

impl TrajectoryEmitter for NullEmitter {
    fn emit(&self, _event: TrajectoryEvent) {}
    fn emit_alert(&self, _alert: BudgetAlert, _state: &BudgetState) {}
    fn verbosity(&self) -> Verbosity {
        Verbosity::Minimal
    }
    fn set_verbosity(&mut self, _verbosity: Verbosity) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = TrajectoryEvent::rlm_start("What is the auth flow?");
        assert_eq!(event.event_type, TrajectoryEventType::RlmStart);
        assert_eq!(event.depth, 0);
        assert_eq!(event.content, "What is the auth flow?");
    }

    #[test]
    fn test_event_with_metadata() {
        let event = TrajectoryEvent::repl_result(1, "42", true);
        assert_eq!(event.get_metadata("success"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_cost_summary() {
        let mut cost = CostSummary::new();
        cost.add(
            CostComponent::Orchestration,
            TokenUsage::new(1000, 500),
            0.05,
        );
        cost.add(CostComponent::Repl, TokenUsage::new(200, 100), 0.01);

        assert_eq!(cost.input_tokens, 1200);
        assert_eq!(cost.output_tokens, 600);
        assert!((cost.total_cost_usd - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_event_log_line() {
        let event = TrajectoryEvent::analyze(1, "Complexity: high");
        let line = event.as_log_line();
        assert!(line.contains("ANALYZE"));
        assert!(line.contains("  ")); // depth indent
        assert!(line.contains("Complexity: high"));
    }

    #[test]
    fn test_export_json_lines() {
        let events = vec![
            TrajectoryEvent::rlm_start("Test"),
            TrajectoryEvent::final_answer(0, "Done"),
        ];
        let exported = export_events(&events, ExportFormat::JsonLines);
        let lines: Vec<_> = exported.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    // =========================================================================
    // Model Pricing Tests
    // =========================================================================

    #[test]
    fn test_model_pricing() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };

        let cost = Model::ClaudeSonnet4.calculate_cost(&usage);
        // 1M * $3/1M + 0.5M * $15/1M = $3 + $7.50 = $10.50
        assert!((cost - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_pricing_with_cache() {
        let usage = TokenUsage {
            input_tokens: 100_000,
            output_tokens: 50_000,
            cache_creation_tokens: 200_000,
            cache_read_tokens: 500_000,
        };

        let pricing = Model::ClaudeHaiku35.pricing();
        let cost = pricing.calculate(&usage);
        assert!(cost > 0.0);
    }

    // =========================================================================
    // Budget Management Tests
    // =========================================================================

    #[test]
    fn test_budget_config_default() {
        let config = BudgetConfig::default();
        assert_eq!(config.max_cost_usd, Some(1.0));
        assert_eq!(config.max_tokens, Some(100_000));
        assert!(config.hard_limit);
    }

    #[test]
    fn test_budget_manager_alerts() {
        let config = BudgetConfig::with_cost_limit(1.0);
        let manager = BudgetManager::new(config);

        // No alerts initially
        let alerts = manager.record_cost(0.5, 50_000);
        assert!(alerts.is_empty());

        // Warning at 80%
        let alerts = manager.record_cost(0.35, 25_000);
        assert!(alerts.contains(&BudgetAlert::Warning));

        // Exceeded at 100%
        let alerts = manager.record_cost(0.20, 10_000);
        assert!(alerts.contains(&BudgetAlert::Exceeded));
    }

    #[test]
    fn test_budget_should_stop() {
        let config = BudgetConfig::with_cost_limit(0.50);
        let manager = BudgetManager::new(config);

        assert!(!manager.should_stop());

        manager.record_cost(0.60, 50_000);
        assert!(manager.should_stop());
    }

    #[test]
    fn test_budget_unlimited() {
        let config = BudgetConfig::unlimited();
        let manager = BudgetManager::new(config);

        manager.record_cost(100.0, 10_000_000);
        assert!(!manager.is_exceeded());
        assert!(!manager.should_stop());
    }

    #[test]
    fn test_budget_depth_limit() {
        let config = BudgetConfig::default();
        let manager = BudgetManager::new(config);

        manager.set_depth(3);
        assert!(!manager.depth_exceeded());

        manager.set_depth(5);
        assert!(manager.depth_exceeded());
    }

    // =========================================================================
    // Verbosity Tests
    // =========================================================================

    #[test]
    fn test_verbosity_ordering() {
        assert!(Verbosity::Minimal < Verbosity::Normal);
        assert!(Verbosity::Normal < Verbosity::Verbose);
        assert!(Verbosity::Verbose < Verbosity::Debug);
    }

    #[test]
    fn test_event_verbosity_levels() {
        // Minimal events always show
        assert!(TrajectoryEventType::Error.should_emit(Verbosity::Minimal));
        assert!(TrajectoryEventType::Final.should_emit(Verbosity::Minimal));

        // Normal events don't show at minimal
        assert!(!TrajectoryEventType::Analyze.should_emit(Verbosity::Minimal));
        assert!(TrajectoryEventType::Analyze.should_emit(Verbosity::Normal));

        // Debug events only at debug
        assert!(!TrajectoryEventType::Memory.should_emit(Verbosity::Verbose));
        assert!(TrajectoryEventType::Memory.should_emit(Verbosity::Debug));
    }

    // =========================================================================
    // Emitter Tests
    // =========================================================================

    #[test]
    fn test_collecting_emitter() {
        let mut emitter = CollectingEmitter::new();
        emitter.set_verbosity(Verbosity::Debug);

        emitter.emit(TrajectoryEvent::rlm_start("Test"));
        emitter.emit(TrajectoryEvent::error(0, "Error"));
        emitter.emit(TrajectoryEvent::final_answer(0, "Done"));

        let events = emitter.events();
        assert_eq!(events.len(), 3);

        emitter.clear();
        assert!(emitter.events().is_empty());
    }

    #[test]
    fn test_collecting_emitter_filters_by_verbosity() {
        let mut emitter = CollectingEmitter::new();
        emitter.set_verbosity(Verbosity::Minimal);

        // Error should pass (minimal)
        emitter.emit(TrajectoryEvent::error(0, "Error"));
        // Memory should be filtered (debug)
        emitter.emit(TrajectoryEvent::new(
            TrajectoryEventType::Memory,
            0,
            "Store",
        ));

        let events = emitter.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TrajectoryEventType::Error);
    }

    #[test]
    fn test_null_emitter() {
        let mut emitter = NullEmitter;
        emitter.emit(TrajectoryEvent::rlm_start("Test"));
        emitter.set_verbosity(Verbosity::Debug);
        assert_eq!(emitter.verbosity(), Verbosity::Minimal);
    }

    #[test]
    fn test_broadcast_emitter_creation() {
        let emitter = BroadcastEmitter::new(100);
        assert_eq!(emitter.subscriber_count(), 0);

        let _rx = emitter.subscribe();
        assert_eq!(emitter.subscriber_count(), 1);
    }
}
