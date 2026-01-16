//! Panel data structures for TUI rendering.
//!
//! These structures represent the data needed to render each panel
//! in the Bubble Tea TUI. All types are serializable for FFI transport.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::memory::{Node, Tier};
use crate::repl::ExecuteResult;
use crate::trajectory::{TrajectoryEvent, TrajectoryEventType};

// =============================================================================
// Trace Panel
// =============================================================================

/// Data for the RLM trace panel.
///
/// Displays the execution trace of RLM operations including events,
/// current recursion depth, and execution status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePanelData {
    /// Recent trace events for display
    pub events: Vec<TraceEventView>,
    /// Current recursion depth
    pub current_depth: u32,
    /// Maximum depth reached in this execution
    pub max_depth_reached: u32,
    /// Current execution status
    pub status: super::ExecutionStatus,
    /// Elapsed time since execution started (milliseconds)
    pub elapsed_ms: u64,
    /// Total events emitted
    pub total_events: u64,
}

impl TracePanelData {
    /// Create empty trace panel data.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            current_depth: 0,
            max_depth_reached: 0,
            status: super::ExecutionStatus::Idle,
            elapsed_ms: 0,
            total_events: 0,
        }
    }

    /// Add an event to the trace panel.
    pub fn push_event(&mut self, event: TraceEventView) {
        if event.depth > self.max_depth_reached {
            self.max_depth_reached = event.depth;
        }
        self.current_depth = event.depth;
        self.total_events += 1;
        self.events.push(event);
    }

    /// Keep only the most recent N events.
    pub fn truncate_to(&mut self, max_events: usize) {
        if self.events.len() > max_events {
            let start = self.events.len() - max_events;
            self.events = self.events[start..].to_vec();
        }
    }
}

impl Default for TracePanelData {
    fn default() -> Self {
        Self::new()
    }
}

/// A single event in the trace panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEventView {
    /// Type of the event (for filtering/styling)
    pub event_type: String,
    /// Human-readable content
    pub content: String,
    /// Recursion depth (for indentation)
    pub depth: u32,
    /// Timestamp (ISO 8601 format)
    pub timestamp: String,
    /// Style hint for rendering
    pub style: EventStyle,
}

impl TraceEventView {
    /// Create a new trace event view.
    pub fn new(
        event_type: impl Into<String>,
        content: impl Into<String>,
        depth: u32,
        style: EventStyle,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            content: content.into(),
            depth,
            timestamp: Utc::now().to_rfc3339(),
            style,
        }
    }

    /// Create from a TrajectoryEvent.
    pub fn from_trajectory_event(event: &TrajectoryEvent) -> Self {
        let style = EventStyle::from_event_type(event.event_type);
        Self {
            event_type: event.event_type.to_string(),
            content: event.content.clone(),
            depth: event.depth,
            timestamp: event.timestamp.to_rfc3339(),
            style,
        }
    }

    /// Truncate content to a maximum length.
    pub fn with_max_content_length(mut self, max_len: usize) -> Self {
        if self.content.len() > max_len {
            self.content = format!("{}...", &self.content[..max_len - 3]);
        }
        self
    }
}

/// Style hint for event rendering in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStyle {
    /// Normal/default styling
    Normal,
    /// Success (green)
    Success,
    /// Error (red)
    Error,
    /// Warning (yellow)
    Warning,
    /// Debug/verbose (dim/gray)
    Debug,
    /// Information (blue)
    Info,
    /// Highlight/important (bold)
    Highlight,
}

impl EventStyle {
    /// Determine style based on trajectory event type.
    pub fn from_event_type(event_type: TrajectoryEventType) -> Self {
        match event_type {
            TrajectoryEventType::Error => Self::Error,
            TrajectoryEventType::Final => Self::Success,
            TrajectoryEventType::HallucinationFlag => Self::Warning,
            TrajectoryEventType::RlmStart => Self::Highlight,
            TrajectoryEventType::ReplExec | TrajectoryEventType::ReplResult => Self::Info,
            TrajectoryEventType::Memory | TrajectoryEventType::Externalize => Self::Debug,
            TrajectoryEventType::ClaimExtracted
            | TrajectoryEventType::EvidenceChecked
            | TrajectoryEventType::BudgetComputed => Self::Debug,
            _ => Self::Normal,
        }
    }
}

impl Default for EventStyle {
    fn default() -> Self {
        Self::Normal
    }
}

// =============================================================================
// REPL Panel
// =============================================================================

/// Data for the REPL panel.
///
/// Displays REPL execution history and current status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplPanelData {
    /// Execution history (most recent last)
    pub history: Vec<ReplEntry>,
    /// Currently executing code (if any)
    pub current_code: Option<String>,
    /// Current REPL status
    pub status: ReplStatus,
    /// Total executions in this session
    pub total_executions: u64,
    /// Total successful executions
    pub successful_executions: u64,
}

impl ReplPanelData {
    /// Create empty REPL panel data.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_code: None,
            status: ReplStatus::Idle,
            total_executions: 0,
            successful_executions: 0,
        }
    }

    /// Record execution starting.
    pub fn start_execution(&mut self, code: impl Into<String>) {
        self.current_code = Some(code.into());
        self.status = ReplStatus::Executing;
    }

    /// Record execution result.
    pub fn record_result(&mut self, entry: ReplEntry) {
        self.total_executions += 1;
        if entry.success {
            self.successful_executions += 1;
        }
        self.current_code = None;
        self.status = ReplStatus::Idle;
        self.history.push(entry);
    }

    /// Keep only the most recent N entries.
    pub fn truncate_to(&mut self, max_entries: usize) {
        if self.history.len() > max_entries {
            let start = self.history.len() - max_entries;
            self.history = self.history[start..].to_vec();
        }
    }

    /// Success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            100.0
        } else {
            (self.successful_executions as f64 / self.total_executions as f64) * 100.0
        }
    }
}

impl Default for ReplPanelData {
    fn default() -> Self {
        Self::new()
    }
}

/// A single REPL execution entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplEntry {
    /// The code that was executed
    pub code: String,
    /// Combined output (stdout + result)
    pub output: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Timestamp of execution
    pub timestamp: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ReplEntry {
    /// Create a new REPL entry.
    pub fn new(code: impl Into<String>, output: impl Into<String>, success: bool) -> Self {
        Self {
            code: code.into(),
            output: output.into(),
            success,
            execution_time_ms: 0,
            timestamp: Utc::now().to_rfc3339(),
            error: None,
        }
    }

    /// Create from an ExecuteResult.
    pub fn from_execute_result(code: impl Into<String>, result: &ExecuteResult) -> Self {
        let output = if result.stdout.is_empty() {
            result
                .result
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default()
        } else {
            result.stdout.clone()
        };

        Self {
            code: code.into(),
            output,
            success: result.success,
            execution_time_ms: result.execution_time_ms as u64,
            timestamp: Utc::now().to_rfc3339(),
            error: result.error.clone(),
        }
    }

    /// Set execution time.
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    /// Set error message.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// REPL subprocess status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplStatus {
    /// Idle, ready for execution
    Idle,
    /// Currently executing code
    Executing,
    /// Waiting for deferred operation resolution
    WaitingForResolution,
    /// REPL subprocess is not running
    Stopped,
    /// REPL encountered an error
    Error,
}

impl Default for ReplStatus {
    fn default() -> Self {
        Self::Idle
    }
}

// =============================================================================
// Memory Panel
// =============================================================================

/// Data for the memory inspector panel.
///
/// Displays hypergraph memory statistics and recent nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPanelData {
    /// Total number of nodes in memory
    pub node_count: usize,
    /// Recent/relevant nodes for display
    pub recent_nodes: Vec<MemoryNodeView>,
    /// Node counts by tier
    pub tier_counts: TierCounts,
    /// Total edge count
    pub edge_count: usize,
    /// Last update timestamp
    pub updated_at: String,
}

impl MemoryPanelData {
    /// Create empty memory panel data.
    pub fn new() -> Self {
        Self {
            node_count: 0,
            recent_nodes: Vec::new(),
            tier_counts: TierCounts::default(),
            edge_count: 0,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Add a node view.
    pub fn add_node(&mut self, node: MemoryNodeView) {
        // Update tier counts
        match node.tier.as_str() {
            "task" => self.tier_counts.task += 1,
            "session" => self.tier_counts.session += 1,
            "longterm" => self.tier_counts.long_term += 1,
            "archive" => self.tier_counts.archive += 1,
            _ => {}
        }
        self.node_count += 1;
        self.recent_nodes.push(node);
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// Keep only the most recent N nodes.
    pub fn truncate_to(&mut self, max_nodes: usize) {
        if self.recent_nodes.len() > max_nodes {
            let start = self.recent_nodes.len() - max_nodes;
            self.recent_nodes = self.recent_nodes[start..].to_vec();
        }
    }
}

impl Default for MemoryPanelData {
    fn default() -> Self {
        Self::new()
    }
}

/// View of a memory node for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNodeView {
    /// Node ID (UUID string)
    pub id: String,
    /// Node type (entity, fact, experience, decision, snippet)
    pub node_type: String,
    /// Preview of content (truncated)
    pub content_preview: String,
    /// Memory tier
    pub tier: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Creation timestamp
    pub created_at: String,
    /// Access count
    pub access_count: u64,
}

impl MemoryNodeView {
    /// Create from a Node.
    pub fn from_node(node: &Node) -> Self {
        Self {
            id: node.id.to_string(),
            node_type: node.node_type.to_string(),
            content_preview: truncate_preview(&node.content, 100),
            tier: node.tier.to_string(),
            confidence: node.confidence,
            created_at: node.created_at.to_rfc3339(),
            access_count: node.access_count,
        }
    }

    /// Create with custom content preview length.
    pub fn from_node_with_preview_length(node: &Node, max_len: usize) -> Self {
        Self {
            id: node.id.to_string(),
            node_type: node.node_type.to_string(),
            content_preview: truncate_preview(&node.content, max_len),
            tier: node.tier.to_string(),
            confidence: node.confidence,
            created_at: node.created_at.to_rfc3339(),
            access_count: node.access_count,
        }
    }
}

/// Truncate content for preview display.
fn truncate_preview(content: &str, max_len: usize) -> String {
    let first_line = content.lines().next().unwrap_or(content);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len - 3])
    } else {
        first_line.to_string()
    }
}

/// Node counts by memory tier.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierCounts {
    /// Task tier (working memory)
    pub task: usize,
    /// Session tier (session-accumulated)
    pub session: usize,
    /// Long-term tier (persistent)
    pub long_term: usize,
    /// Archive tier (decayed)
    pub archive: usize,
}

impl TierCounts {
    /// Create new tier counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total count across all tiers.
    pub fn total(&self) -> usize {
        self.task + self.session + self.long_term + self.archive
    }

    /// Increment count for a tier.
    pub fn increment(&mut self, tier: Tier) {
        match tier {
            Tier::Task => self.task += 1,
            Tier::Session => self.session += 1,
            Tier::LongTerm => self.long_term += 1,
            Tier::Archive => self.archive += 1,
        }
    }
}

// =============================================================================
// Budget Panel
// =============================================================================

/// Data for the budget status panel.
///
/// Displays cost tracking, token usage, and budget alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPanelData {
    /// Current accumulated cost in USD
    pub cost_usd: f64,
    /// Total tokens used
    pub tokens_used: u64,
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens used
    pub output_tokens: u64,
    /// Budget limit in USD (None = unlimited)
    pub budget_limit: Option<f64>,
    /// Token limit (None = unlimited)
    pub token_limit: Option<u64>,
    /// Budget utilization percentage (0-100)
    pub utilization_percent: f64,
    /// Current burn rate (USD per minute)
    pub burn_rate: f64,
    /// Estimated time to budget exhaustion (seconds, None = N/A)
    pub estimated_exhaustion_secs: Option<u64>,
    /// Active alerts
    pub alerts: Vec<String>,
    /// Last update timestamp
    pub updated_at: String,
}

impl BudgetPanelData {
    /// Create empty budget panel data.
    pub fn new() -> Self {
        Self {
            cost_usd: 0.0,
            tokens_used: 0,
            input_tokens: 0,
            output_tokens: 0,
            budget_limit: None,
            token_limit: None,
            utilization_percent: 0.0,
            burn_rate: 0.0,
            estimated_exhaustion_secs: None,
            alerts: Vec::new(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create with budget limits.
    pub fn with_limits(cost_limit: Option<f64>, token_limit: Option<u64>) -> Self {
        Self {
            budget_limit: cost_limit,
            token_limit,
            ..Self::new()
        }
    }

    /// Update with new cost/token data.
    pub fn update(&mut self, cost_usd: f64, input_tokens: u64, output_tokens: u64) {
        self.cost_usd = cost_usd;
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self.tokens_used = input_tokens + output_tokens;
        self.updated_at = Utc::now().to_rfc3339();

        // Recalculate utilization
        if let Some(limit) = self.budget_limit {
            self.utilization_percent = (cost_usd / limit) * 100.0;
        }
    }

    /// Set burn rate and estimated exhaustion.
    pub fn set_burn_rate(&mut self, rate_per_minute: f64) {
        self.burn_rate = rate_per_minute;

        // Calculate estimated exhaustion
        if rate_per_minute > 0.0 {
            if let Some(limit) = self.budget_limit {
                let remaining = limit - self.cost_usd;
                if remaining > 0.0 {
                    let minutes = remaining / rate_per_minute;
                    self.estimated_exhaustion_secs = Some((minutes * 60.0) as u64);
                } else {
                    self.estimated_exhaustion_secs = Some(0);
                }
            }
        }
    }

    /// Add an alert.
    pub fn add_alert(&mut self, alert: impl Into<String>) {
        self.alerts.push(alert.into());
    }

    /// Clear all alerts.
    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    /// Check if budget is exceeded.
    pub fn is_exceeded(&self) -> bool {
        if let Some(limit) = self.budget_limit {
            return self.cost_usd >= limit;
        }
        if let Some(limit) = self.token_limit {
            return self.tokens_used >= limit;
        }
        false
    }

    /// Check if budget warning threshold is reached (80%).
    pub fn is_warning(&self) -> bool {
        self.utilization_percent >= 80.0 && self.utilization_percent < 100.0
    }

    /// Format cost as string.
    pub fn format_cost(&self) -> String {
        format!("${:.4}", self.cost_usd)
    }

    /// Format tokens as string.
    pub fn format_tokens(&self) -> String {
        format!("{} in / {} out", self.input_tokens, self.output_tokens)
    }
}

impl Default for BudgetPanelData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_panel_data() {
        let mut panel = TracePanelData::new();
        assert_eq!(panel.status, super::super::ExecutionStatus::Idle);

        let event = TraceEventView::new("RLM_START", "Starting analysis", 0, EventStyle::Highlight);
        panel.push_event(event);

        assert_eq!(panel.total_events, 1);
        assert_eq!(panel.current_depth, 0);
    }

    #[test]
    fn test_repl_entry_creation() {
        let entry = ReplEntry::new("print('hello')", "hello", true).with_execution_time(100);

        assert!(entry.success);
        assert_eq!(entry.execution_time_ms, 100);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_repl_panel_success_rate() {
        let mut panel = ReplPanelData::new();

        panel.record_result(ReplEntry::new("1+1", "2", true));
        panel.record_result(ReplEntry::new("1/0", "error", false));
        panel.record_result(ReplEntry::new("2+2", "4", true));

        assert!((panel.success_rate() - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_tier_counts() {
        let mut counts = TierCounts::new();
        counts.increment(Tier::Task);
        counts.increment(Tier::Task);
        counts.increment(Tier::Session);

        assert_eq!(counts.task, 2);
        assert_eq!(counts.session, 1);
        assert_eq!(counts.total(), 3);
    }

    #[test]
    fn test_budget_panel_limits() {
        let mut panel = BudgetPanelData::with_limits(Some(1.0), Some(100_000));
        panel.update(0.5, 30_000, 20_000);

        assert_eq!(panel.utilization_percent, 50.0);
        assert!(!panel.is_exceeded());
        assert!(!panel.is_warning());

        panel.update(0.85, 60_000, 40_000);
        assert!(panel.is_warning());
    }

    #[test]
    fn test_event_style_from_type() {
        assert_eq!(
            EventStyle::from_event_type(TrajectoryEventType::Error),
            EventStyle::Error
        );
        assert_eq!(
            EventStyle::from_event_type(TrajectoryEventType::Final),
            EventStyle::Success
        );
        assert_eq!(
            EventStyle::from_event_type(TrajectoryEventType::HallucinationFlag),
            EventStyle::Warning
        );
    }

    #[test]
    fn test_truncate_preview() {
        let short = "Hello world";
        assert_eq!(truncate_preview(short, 100), "Hello world");

        let long = "This is a very long string that should be truncated";
        let truncated = truncate_preview(long, 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 20);
    }
}
