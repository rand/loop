//! Main TUI adapter for Bubble Tea integration.
//!
//! The TUIAdapter provides a unified interface for Go's Bubble Tea TUI
//! to interact with rlm-core systems.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::memory::SqliteMemoryStore;
use crate::trajectory::{
    BroadcastEmitter, BudgetConfig, BudgetManager, BudgetState, TrajectoryEmitter, TrajectoryEvent,
    Verbosity,
};

use super::events::{EventBridge, ExecutionStatus, TUIEvent};
use super::panels::{
    BudgetPanelData, MemoryNodeView, MemoryPanelData, ReplEntry, ReplPanelData,
    TierCounts, TracePanelData, TraceEventView,
};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for the TUI adapter.
#[derive(Debug, Clone)]
pub struct TUIConfig {
    /// Maximum events to keep in trace panel
    pub max_trace_events: usize,
    /// Maximum REPL history entries
    pub max_repl_history: usize,
    /// Maximum memory nodes to display
    pub max_memory_nodes: usize,
    /// Event channel capacity
    pub event_channel_capacity: usize,
    /// Content preview length for memory nodes
    pub memory_preview_length: usize,
    /// Content preview length for trace events
    pub trace_preview_length: usize,
    /// Budget configuration
    pub budget_config: BudgetConfig,
    /// Verbosity level for trace events
    pub verbosity: Verbosity,
}

impl Default for TUIConfig {
    fn default() -> Self {
        Self {
            max_trace_events: 100,
            max_repl_history: 50,
            max_memory_nodes: 20,
            event_channel_capacity: 256,
            memory_preview_length: 100,
            trace_preview_length: 200,
            budget_config: BudgetConfig::default(),
            verbosity: Verbosity::Normal,
        }
    }
}

impl TUIConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum trace events.
    pub fn max_trace_events(mut self, max: usize) -> Self {
        self.max_trace_events = max;
        self
    }

    /// Set maximum REPL history.
    pub fn max_repl_history(mut self, max: usize) -> Self {
        self.max_repl_history = max;
        self
    }

    /// Set maximum memory nodes.
    pub fn max_memory_nodes(mut self, max: usize) -> Self {
        self.max_memory_nodes = max;
        self
    }

    /// Set event channel capacity.
    pub fn event_channel_capacity(mut self, capacity: usize) -> Self {
        self.event_channel_capacity = capacity;
        self
    }

    /// Set budget configuration.
    pub fn budget_config(mut self, config: BudgetConfig) -> Self {
        self.budget_config = config;
        self
    }

    /// Set verbosity level.
    pub fn verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }
}

// =============================================================================
// TUI Adapter
// =============================================================================

/// Main adapter for TUI integration.
///
/// The TUIAdapter wraps rlm-core components and provides:
/// - Panel data for UI rendering
/// - Event streaming for real-time updates
/// - Budget and status management
pub struct TUIAdapter {
    /// Memory store reference
    memory: Option<Arc<SqliteMemoryStore>>,
    /// Budget manager
    budget: Arc<BudgetManager>,
    /// Trajectory emitter for internal events
    emitter: Arc<RwLock<BroadcastEmitter>>,
    /// Event bridge for TUI events
    event_bridge: EventBridge,
    /// Configuration
    config: TUIConfig,
    /// Panel state
    state: Arc<RwLock<AdapterState>>,
    /// Execution start time (for elapsed calculation)
    execution_start: Arc<RwLock<Option<Instant>>>,
}

/// Internal state for panel data.
#[derive(Debug, Default)]
struct AdapterState {
    trace: TracePanelData,
    repl: ReplPanelData,
    memory: MemoryPanelData,
    budget: BudgetPanelData,
    status: ExecutionStatus,
}

impl TUIAdapter {
    /// Create a new TUI adapter with the given configuration.
    pub fn new(config: TUIConfig) -> Self {
        let budget = Arc::new(BudgetManager::new(config.budget_config.clone()));
        let mut emitter = BroadcastEmitter::new(config.event_channel_capacity);
        emitter.set_verbosity(config.verbosity);
        let emitter = Arc::new(RwLock::new(emitter));
        let event_bridge = EventBridge::new(config.event_channel_capacity);

        // Initialize budget panel with limits
        let budget_panel = BudgetPanelData::with_limits(
            config.budget_config.max_cost_usd,
            config.budget_config.max_tokens,
        );

        let state = AdapterState {
            budget: budget_panel,
            ..Default::default()
        };

        Self {
            memory: None,
            budget,
            emitter,
            event_bridge,
            config,
            state: Arc::new(RwLock::new(state)),
            execution_start: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(TUIConfig::default())
    }

    /// Set the memory store.
    pub fn with_memory(mut self, memory: Arc<SqliteMemoryStore>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &TUIConfig {
        &self.config
    }

    // =========================================================================
    // Event Subscription
    // =========================================================================

    /// Subscribe to TUI events.
    ///
    /// Returns a receiver that will receive all TUI events suitable for
    /// forwarding to Go channels.
    pub fn subscribe_events(&self) -> broadcast::Receiver<TUIEvent> {
        self.event_bridge.subscribe()
    }

    /// Subscribe to raw trajectory events.
    ///
    /// Returns a receiver for internal trajectory events.
    pub async fn subscribe_trajectory(&self) -> broadcast::Receiver<TrajectoryEvent> {
        self.emitter.read().await.subscribe()
    }

    /// Get the event bridge for direct event emission.
    pub fn event_bridge(&self) -> &EventBridge {
        &self.event_bridge
    }

    // =========================================================================
    // Panel Data Access
    // =========================================================================

    /// Get current trace panel data.
    pub async fn get_trace_panel(&self) -> TracePanelData {
        let state = self.state.read().await;
        let mut panel = state.trace.clone();

        // Update elapsed time if execution is running
        if let Some(start) = *self.execution_start.read().await {
            panel.elapsed_ms = start.elapsed().as_millis() as u64;
        }

        panel
    }

    /// Get current REPL panel data.
    pub async fn get_repl_panel(&self) -> ReplPanelData {
        self.state.read().await.repl.clone()
    }

    /// Get current memory panel data.
    pub async fn get_memory_panel(&self) -> MemoryPanelData {
        let mut panel = self.state.read().await.memory.clone();

        // Refresh from memory store if available
        if let Some(ref memory) = self.memory {
            if let Ok(stats) = memory.stats() {
                panel.node_count = stats.total_nodes as usize;
                panel.edge_count = stats.total_edges as usize;
                panel.tier_counts = TierCounts {
                    task: *stats.nodes_by_tier.get(&crate::memory::Tier::Task).unwrap_or(&0) as usize,
                    session: *stats.nodes_by_tier.get(&crate::memory::Tier::Session).unwrap_or(&0) as usize,
                    long_term: *stats.nodes_by_tier.get(&crate::memory::Tier::LongTerm).unwrap_or(&0) as usize,
                    archive: *stats.nodes_by_tier.get(&crate::memory::Tier::Archive).unwrap_or(&0) as usize,
                };
            }
        }

        panel
    }

    /// Get current budget panel data.
    pub async fn get_budget_panel(&self) -> BudgetPanelData {
        let mut panel = self.state.read().await.budget.clone();

        // Refresh from budget manager
        let budget_state = self.budget.state();
        panel.cost_usd = budget_state.current_cost_usd;
        panel.tokens_used = budget_state.current_tokens;
        panel.burn_rate = budget_state.burn_rate_per_minute();

        if let Some(limit) = self.config.budget_config.max_cost_usd {
            panel.utilization_percent = (budget_state.current_cost_usd / limit) * 100.0;
        }

        panel
    }

    /// Get current execution status.
    pub async fn get_status(&self) -> ExecutionStatus {
        self.state.read().await.status
    }

    // =========================================================================
    // Event Processing
    // =========================================================================

    /// Process a trajectory event and update panel state.
    pub async fn process_trajectory_event(&self, event: &TrajectoryEvent) {
        let mut state = self.state.write().await;

        // Create view and add to trace panel
        let view = TraceEventView::from_trajectory_event(event)
            .with_max_content_length(self.config.trace_preview_length);
        state.trace.push_event(view.clone());
        state.trace.truncate_to(self.config.max_trace_events);

        // Update status based on event type
        match event.event_type {
            crate::trajectory::TrajectoryEventType::RlmStart => {
                state.status = ExecutionStatus::Running;
                state.trace.status = ExecutionStatus::Running;
            }
            crate::trajectory::TrajectoryEventType::Final => {
                state.status = ExecutionStatus::Complete;
                state.trace.status = ExecutionStatus::Complete;
            }
            crate::trajectory::TrajectoryEventType::Error => {
                state.status = ExecutionStatus::Error;
                state.trace.status = ExecutionStatus::Error;
            }
            _ => {}
        }

        // Forward to event bridge
        self.event_bridge.forward_trajectory(event);
    }

    /// Record a REPL execution.
    pub async fn record_repl_execution(&self, code: &str, result: &crate::repl::ExecuteResult) {
        let entry = ReplEntry::from_execute_result(code, result);

        let mut state = self.state.write().await;
        state.repl.record_result(entry.clone());
        state.repl.truncate_to(self.config.max_repl_history);

        // Forward to event bridge
        self.event_bridge.forward_repl(entry);
    }

    /// Record a memory node addition.
    pub async fn record_memory_node(&self, node: &crate::memory::Node) {
        let view = MemoryNodeView::from_node_with_preview_length(
            node,
            self.config.memory_preview_length,
        );

        let mut state = self.state.write().await;
        state.memory.add_node(view.clone());
        state.memory.truncate_to(self.config.max_memory_nodes);

        // Forward to event bridge
        self.event_bridge.forward_memory(view);
    }

    /// Record cost and update budget panel.
    pub async fn record_cost(&self, cost_usd: f64, input_tokens: u64, output_tokens: u64) {
        let tokens = input_tokens + output_tokens;
        let alerts = self.budget.record_cost(cost_usd, tokens);

        let mut state = self.state.write().await;
        state.budget.update(
            self.budget.state().current_cost_usd,
            input_tokens,
            output_tokens,
        );
        state.budget.set_burn_rate(self.budget.burn_rate());

        // Forward alerts
        let budget_state = self.budget.state();
        for alert in alerts {
            self.event_bridge.forward_budget_alert(
                alert,
                &budget_state,
                self.config.budget_config.max_cost_usd,
            );
            state.budget.add_alert(format!("{:?}", alert));
        }

        // Forward state update
        self.event_bridge.forward_budget_state(
            &budget_state,
            self.config.budget_config.max_cost_usd,
        );
    }

    // =========================================================================
    // Execution Control
    // =========================================================================

    /// Mark execution as started.
    pub async fn start_execution(&self) {
        *self.execution_start.write().await = Some(Instant::now());

        let mut state = self.state.write().await;
        state.status = ExecutionStatus::Running;
        state.trace.status = ExecutionStatus::Running;

        self.event_bridge.forward_status(
            ExecutionStatus::Running,
            Some("Execution started".to_string()),
        );
    }

    /// Mark execution as complete.
    pub async fn complete_execution(&self) {
        let elapsed = self.execution_start.read().await
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);

        *self.execution_start.write().await = None;

        let mut state = self.state.write().await;
        state.status = ExecutionStatus::Complete;
        state.trace.status = ExecutionStatus::Complete;
        state.trace.elapsed_ms = elapsed;

        self.event_bridge.forward_status(
            ExecutionStatus::Complete,
            Some(format!("Execution completed in {}ms", elapsed)),
        );
    }

    /// Mark execution as failed.
    pub async fn fail_execution(&self, error: impl Into<String>) {
        let error_msg = error.into();
        *self.execution_start.write().await = None;

        let mut state = self.state.write().await;
        state.status = ExecutionStatus::Error;
        state.trace.status = ExecutionStatus::Error;

        self.event_bridge.forward_status(
            ExecutionStatus::Error,
            Some(error_msg.clone()),
        );
        self.event_bridge.forward_error(error_msg);
    }

    /// Cancel execution.
    pub async fn cancel_execution(&self) {
        *self.execution_start.write().await = None;

        let mut state = self.state.write().await;
        state.status = ExecutionStatus::Cancelled;
        state.trace.status = ExecutionStatus::Cancelled;

        self.event_bridge.forward_status(
            ExecutionStatus::Cancelled,
            Some("Execution cancelled".to_string()),
        );
    }

    /// Reset all panel state.
    pub async fn reset(&self) {
        *self.execution_start.write().await = None;

        let mut state = self.state.write().await;
        state.trace = TracePanelData::new();
        state.repl = ReplPanelData::new();
        state.memory = MemoryPanelData::new();
        state.budget = BudgetPanelData::with_limits(
            self.config.budget_config.max_cost_usd,
            self.config.budget_config.max_tokens,
        );
        state.status = ExecutionStatus::Idle;

        self.event_bridge.forward_status(
            ExecutionStatus::Idle,
            Some("Adapter reset".to_string()),
        );
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Check if budget is exceeded.
    pub fn is_budget_exceeded(&self) -> bool {
        self.budget.is_exceeded()
    }

    /// Check if execution should stop due to budget.
    pub fn should_stop(&self) -> bool {
        self.budget.should_stop()
    }

    /// Get current budget state.
    pub fn budget_state(&self) -> BudgetState {
        self.budget.state()
    }

    /// Set verbosity level.
    pub async fn set_verbosity(&self, verbosity: Verbosity) {
        self.emitter.write().await.set_verbosity(verbosity);
    }

    /// Get subscriber counts.
    pub async fn subscriber_counts(&self) -> (usize, usize) {
        let trajectory_count = self.emitter.read().await.subscriber_count();
        let tui_count = self.event_bridge.subscriber_count();
        (trajectory_count, tui_count)
    }
}

impl std::fmt::Debug for TUIAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TUIAdapter")
            .field("config", &self.config)
            .field("has_memory", &self.memory.is_some())
            .field("event_bridge", &self.event_bridge)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::TrajectoryEvent;

    #[test]
    fn test_config_builder() {
        let config = TUIConfig::new()
            .max_trace_events(200)
            .max_repl_history(100)
            .event_channel_capacity(512);

        assert_eq!(config.max_trace_events, 200);
        assert_eq!(config.max_repl_history, 100);
        assert_eq!(config.event_channel_capacity, 512);
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        let adapter = TUIAdapter::with_defaults();
        let status = adapter.get_status().await;
        assert_eq!(status, ExecutionStatus::Idle);
    }

    #[tokio::test]
    async fn test_execution_lifecycle() {
        let adapter = TUIAdapter::with_defaults();

        adapter.start_execution().await;
        assert_eq!(adapter.get_status().await, ExecutionStatus::Running);

        adapter.complete_execution().await;
        assert_eq!(adapter.get_status().await, ExecutionStatus::Complete);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let adapter = TUIAdapter::with_defaults();
        let mut rx = adapter.subscribe_events();

        adapter.start_execution().await;

        // Should receive status update
        let event = rx.recv().await.unwrap();
        match event {
            TUIEvent::Status(update) => {
                assert_eq!(update.status, ExecutionStatus::Running);
            }
            _ => panic!("Expected status event"),
        }
    }

    #[tokio::test]
    async fn test_trajectory_processing() {
        let adapter = TUIAdapter::with_defaults();
        let mut rx = adapter.subscribe_events();

        let event = TrajectoryEvent::rlm_start("Test query");
        adapter.process_trajectory_event(&event).await;

        // Should receive trace event
        let tui_event = rx.recv().await.unwrap();
        match tui_event {
            TUIEvent::Trace(view) => {
                assert_eq!(view.event_type, "RLM_START");
                assert_eq!(view.content, "Test query");
            }
            _ => panic!("Expected trace event"),
        }

        // Check panel state
        let panel = adapter.get_trace_panel().await;
        assert_eq!(panel.total_events, 1);
        assert_eq!(panel.status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_budget_recording() {
        let config = TUIConfig::new()
            .budget_config(BudgetConfig::with_cost_limit(1.0));
        let adapter = TUIAdapter::new(config);

        adapter.record_cost(0.5, 30_000, 20_000).await;

        let panel = adapter.get_budget_panel().await;
        assert!((panel.cost_usd - 0.5).abs() < 0.01);
        assert_eq!(panel.tokens_used, 50_000);
        assert!((panel.utilization_percent - 50.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_reset() {
        let adapter = TUIAdapter::with_defaults();

        adapter.start_execution().await;
        let event = TrajectoryEvent::rlm_start("Test");
        adapter.process_trajectory_event(&event).await;

        adapter.reset().await;

        let trace = adapter.get_trace_panel().await;
        assert_eq!(trace.total_events, 0);
        assert_eq!(adapter.get_status().await, ExecutionStatus::Idle);
    }
}
