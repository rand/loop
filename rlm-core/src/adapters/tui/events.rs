//! Event types and bridge for TUI consumption.
//!
//! This module provides the event bridge that converts internal rlm-core
//! events to TUI-friendly events suitable for Go channel consumption.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::trajectory::{BudgetAlert, BudgetState, TrajectoryEvent, TrajectoryEventType};

use super::panels::{MemoryNodeView, ReplEntry, TraceEventView};

// =============================================================================
// TUI Events
// =============================================================================

/// Event types for TUI consumption.
///
/// These events are serialized to JSON for transport across the FFI boundary
/// to Go channels. Each variant carries the data needed to update the
/// corresponding TUI panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TUIEvent {
    /// Trace panel event
    Trace(TraceEventView),
    /// REPL execution event
    Repl(ReplEntry),
    /// Memory node added/updated
    Memory(MemoryNodeView),
    /// Budget update
    Budget(BudgetUpdate),
    /// Execution status change
    Status(StatusUpdate),
    /// Error occurred
    Error(String),
    /// Batch of events (for efficiency)
    Batch(Vec<TUIEvent>),
}

impl TUIEvent {
    /// Create a trace event.
    pub fn trace(event: TraceEventView) -> Self {
        Self::Trace(event)
    }

    /// Create a REPL event.
    pub fn repl(entry: ReplEntry) -> Self {
        Self::Repl(entry)
    }

    /// Create a memory event.
    pub fn memory(node: MemoryNodeView) -> Self {
        Self::Memory(node)
    }

    /// Create a budget event.
    pub fn budget(update: BudgetUpdate) -> Self {
        Self::Budget(update)
    }

    /// Create a status event.
    pub fn status(update: StatusUpdate) -> Self {
        Self::Status(update)
    }

    /// Create an error event.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(message.into())
    }

    /// Create a batch event.
    pub fn batch(events: Vec<TUIEvent>) -> Self {
        Self::Batch(events)
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get the event type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Trace(_) => "trace",
            Self::Repl(_) => "repl",
            Self::Memory(_) => "memory",
            Self::Budget(_) => "budget",
            Self::Status(_) => "status",
            Self::Error(_) => "error",
            Self::Batch(_) => "batch",
        }
    }
}

// =============================================================================
// Budget Update
// =============================================================================

/// Budget update data for TUI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetUpdate {
    /// Current cost in USD
    pub cost: f64,
    /// Total tokens used
    pub tokens: u64,
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Burn rate (USD per minute)
    pub burn_rate: f64,
    /// Utilization percentage
    pub utilization_percent: f64,
    /// Alert message (if triggered)
    pub alert: Option<String>,
    /// Alert level (warning, exceeded, custom)
    pub alert_level: Option<AlertLevel>,
}

impl BudgetUpdate {
    /// Create a new budget update.
    pub fn new(cost: f64, tokens: u64) -> Self {
        Self {
            cost,
            tokens,
            input_tokens: 0,
            output_tokens: 0,
            burn_rate: 0.0,
            utilization_percent: 0.0,
            alert: None,
            alert_level: None,
        }
    }

    /// Create from budget state.
    pub fn from_state(state: &BudgetState, limit: Option<f64>) -> Self {
        let utilization = limit
            .map(|l| (state.current_cost_usd / l) * 100.0)
            .unwrap_or(0.0);

        Self {
            cost: state.current_cost_usd,
            tokens: state.current_tokens,
            input_tokens: 0, // Not tracked in BudgetState
            output_tokens: 0,
            burn_rate: state.burn_rate_per_minute(),
            utilization_percent: utilization,
            alert: None,
            alert_level: None,
        }
    }

    /// Set token breakdown.
    pub fn with_tokens(mut self, input: u64, output: u64) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self.tokens = input + output;
        self
    }

    /// Set burn rate.
    pub fn with_burn_rate(mut self, rate: f64) -> Self {
        self.burn_rate = rate;
        self
    }

    /// Set utilization.
    pub fn with_utilization(mut self, percent: f64) -> Self {
        self.utilization_percent = percent;
        self
    }

    /// Set alert.
    pub fn with_alert(mut self, message: impl Into<String>, level: AlertLevel) -> Self {
        self.alert = Some(message.into());
        self.alert_level = Some(level);
        self
    }
}

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    /// Informational alert
    Info,
    /// Warning threshold reached
    Warning,
    /// Budget exceeded
    Exceeded,
    /// Critical alert
    Critical,
}

impl From<BudgetAlert> for AlertLevel {
    fn from(alert: BudgetAlert) -> Self {
        match alert {
            BudgetAlert::Warning => Self::Warning,
            BudgetAlert::Exceeded => Self::Exceeded,
            BudgetAlert::Custom(p) if p >= 100 => Self::Exceeded,
            BudgetAlert::Custom(p) if p >= 80 => Self::Warning,
            BudgetAlert::Custom(_) => Self::Info,
        }
    }
}

// =============================================================================
// Status Update
// =============================================================================

/// Execution status update for TUI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    /// Current execution status
    pub status: ExecutionStatus,
    /// Human-readable message
    pub message: Option<String>,
    /// Current recursion depth
    pub depth: u32,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
}

impl StatusUpdate {
    /// Create a new status update.
    pub fn new(status: ExecutionStatus) -> Self {
        Self {
            status,
            message: None,
            depth: 0,
            elapsed_ms: 0,
        }
    }

    /// Set message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set depth.
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Set elapsed time.
    pub fn with_elapsed(mut self, ms: u64) -> Self {
        self.elapsed_ms = ms;
        self
    }
}

/// Execution status for TUI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// No execution in progress
    Idle,
    /// Execution is running
    Running,
    /// Execution is paused (e.g., waiting for user input)
    Paused,
    /// Execution completed successfully
    Complete,
    /// Execution failed with error
    Error,
    /// Execution was cancelled
    Cancelled,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Complete => write!(f, "complete"),
            Self::Error => write!(f, "error"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// =============================================================================
// Event Bridge
// =============================================================================

/// Bridge to convert internal events to TUI events.
///
/// The EventBridge subscribes to internal rlm-core events and converts them
/// to TUIEvents that can be consumed by Go channels via FFI.
pub struct EventBridge {
    /// Sender for TUI events
    sender: broadcast::Sender<TUIEvent>,
    /// Channel capacity
    capacity: usize,
}

impl EventBridge {
    /// Create a new event bridge with the specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender, capacity }
    }

    /// Subscribe to TUI events.
    ///
    /// Returns a receiver that will receive all TUI events.
    pub fn subscribe(&self) -> broadcast::Receiver<TUIEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the channel capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Forward a trajectory event as a TUI trace event.
    pub fn forward_trajectory(&self, event: &TrajectoryEvent) {
        let view = TraceEventView::from_trajectory_event(event);
        let tui_event = TUIEvent::Trace(view);
        let _ = self.sender.send(tui_event);
    }

    /// Forward a budget alert.
    pub fn forward_budget_alert(&self, alert: BudgetAlert, state: &BudgetState, limit: Option<f64>) {
        let message = match alert {
            BudgetAlert::Warning => format!(
                "Budget warning: ${:.4} spent ({:.1}% of limit)",
                state.current_cost_usd,
                limit.map(|l| (state.current_cost_usd / l) * 100.0).unwrap_or(0.0)
            ),
            BudgetAlert::Exceeded => format!("Budget exceeded: ${:.4} spent", state.current_cost_usd),
            BudgetAlert::Custom(p) => format!(
                "Budget alert ({}%): ${:.4} spent",
                p, state.current_cost_usd
            ),
        };

        let update = BudgetUpdate::from_state(state, limit)
            .with_alert(message, AlertLevel::from(alert));

        let _ = self.sender.send(TUIEvent::Budget(update));
    }

    /// Forward a budget state update (no alert).
    pub fn forward_budget_state(&self, state: &BudgetState, limit: Option<f64>) {
        let update = BudgetUpdate::from_state(state, limit);
        let _ = self.sender.send(TUIEvent::Budget(update));
    }

    /// Forward a status update.
    pub fn forward_status(&self, status: ExecutionStatus, message: Option<String>) {
        let update = StatusUpdate::new(status);
        let update = if let Some(msg) = message {
            update.with_message(msg)
        } else {
            update
        };
        let _ = self.sender.send(TUIEvent::Status(update));
    }

    /// Forward a REPL entry.
    pub fn forward_repl(&self, entry: ReplEntry) {
        let _ = self.sender.send(TUIEvent::Repl(entry));
    }

    /// Forward a memory node.
    pub fn forward_memory(&self, node: MemoryNodeView) {
        let _ = self.sender.send(TUIEvent::Memory(node));
    }

    /// Forward an error.
    pub fn forward_error(&self, error: impl Into<String>) {
        let _ = self.sender.send(TUIEvent::Error(error.into()));
    }

    /// Emit a raw TUI event.
    pub fn emit(&self, event: TUIEvent) -> Result<usize, broadcast::error::SendError<TUIEvent>> {
        self.sender.send(event)
    }

    /// Emit a batch of events.
    pub fn emit_batch(&self, events: Vec<TUIEvent>) -> Result<usize, broadcast::error::SendError<TUIEvent>> {
        self.sender.send(TUIEvent::Batch(events))
    }
}

impl Clone for EventBridge {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
        }
    }
}

impl std::fmt::Debug for EventBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBridge")
            .field("capacity", &self.capacity)
            .field("subscriber_count", &self.sender.receiver_count())
            .finish()
    }
}

// =============================================================================
// Conversion Helpers
// =============================================================================

/// Convert trajectory event type to execution status (where applicable).
impl From<TrajectoryEventType> for Option<ExecutionStatus> {
    fn from(event_type: TrajectoryEventType) -> Self {
        match event_type {
            TrajectoryEventType::RlmStart => Some(ExecutionStatus::Running),
            TrajectoryEventType::Final => Some(ExecutionStatus::Complete),
            TrajectoryEventType::Error => Some(ExecutionStatus::Error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_event_serialization() {
        let event = TUIEvent::error("Test error");
        let json = event.to_json().unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Test error"));

        let parsed = TUIEvent::from_json(&json).unwrap();
        match parsed {
            TUIEvent::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_budget_update() {
        let update = BudgetUpdate::new(0.5, 50_000)
            .with_tokens(30_000, 20_000)
            .with_burn_rate(0.1)
            .with_utilization(50.0);

        assert_eq!(update.cost, 0.5);
        assert_eq!(update.tokens, 50_000);
        assert_eq!(update.input_tokens, 30_000);
        assert_eq!(update.output_tokens, 20_000);
        assert_eq!(update.burn_rate, 0.1);
    }

    #[test]
    fn test_status_update() {
        let update = StatusUpdate::new(ExecutionStatus::Running)
            .with_message("Processing query")
            .with_depth(2)
            .with_elapsed(1500);

        assert_eq!(update.status, ExecutionStatus::Running);
        assert_eq!(update.message, Some("Processing query".to_string()));
        assert_eq!(update.depth, 2);
        assert_eq!(update.elapsed_ms, 1500);
    }

    #[test]
    fn test_event_bridge_creation() {
        let bridge = EventBridge::new(100);
        assert_eq!(bridge.capacity(), 100);
        assert_eq!(bridge.subscriber_count(), 0);
    }

    #[test]
    fn test_event_bridge_subscribe() {
        let bridge = EventBridge::new(100);
        let _rx1 = bridge.subscribe();
        assert_eq!(bridge.subscriber_count(), 1);

        let _rx2 = bridge.subscribe();
        assert_eq!(bridge.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_event_bridge_emit() {
        let bridge = EventBridge::new(100);
        let mut rx = bridge.subscribe();

        bridge.forward_error("Test error");

        let event = rx.recv().await.unwrap();
        match event {
            TUIEvent::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_alert_level_from_budget_alert() {
        assert_eq!(AlertLevel::from(BudgetAlert::Warning), AlertLevel::Warning);
        assert_eq!(AlertLevel::from(BudgetAlert::Exceeded), AlertLevel::Exceeded);
        assert_eq!(AlertLevel::from(BudgetAlert::Custom(100)), AlertLevel::Exceeded);
        assert_eq!(AlertLevel::from(BudgetAlert::Custom(80)), AlertLevel::Warning);
        assert_eq!(AlertLevel::from(BudgetAlert::Custom(50)), AlertLevel::Info);
    }

    #[test]
    fn test_execution_status_display() {
        assert_eq!(ExecutionStatus::Idle.to_string(), "idle");
        assert_eq!(ExecutionStatus::Running.to_string(), "running");
        assert_eq!(ExecutionStatus::Complete.to_string(), "complete");
    }

    #[test]
    fn test_tui_event_type_name() {
        assert_eq!(TUIEvent::error("test").type_name(), "error");
        assert_eq!(TUIEvent::budget(BudgetUpdate::new(0.0, 0)).type_name(), "budget");
        assert_eq!(TUIEvent::status(StatusUpdate::new(ExecutionStatus::Idle)).type_name(), "status");
    }
}
