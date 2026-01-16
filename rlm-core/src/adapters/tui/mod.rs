//! TUI adapter for Bubble Tea integration.
//!
//! This module provides the Rust side of the TUI adapter for integration
//! with Go's Bubble Tea TUI framework. It exposes:
//!
//! - Panel data structures for rendering UI components
//! - Event bridge for streaming updates to Go channels
//! - Status types for execution state management
//!
//! ## Architecture
//!
//! The TUI adapter sits between rlm-core's internal systems and the Go TUI:
//!
//! ```text
//! rlm-core            TUI Adapter              Go TUI
//! +---------+        +------------+           +--------+
//! |Orchestr.|------->|TUIAdapter  |           |Bubble  |
//! |Memory   |        |EventBridge |---(FFI)-->|Tea     |
//! |Budget   |        |Panel Data  |           |Panels  |
//! +---------+        +------------+           +--------+
//! ```
//!
//! ## Panel Types
//!
//! - `TracePanelData`: RLM execution trace events
//! - `ReplPanelData`: REPL execution history
//! - `MemoryPanelData`: Hypergraph memory inspector
//! - `BudgetPanelData`: Cost and token budget status
//!
//! ## Events
//!
//! Events are streamed via `EventBridge` which converts internal
//! `TrajectoryEvent`s to `TUIEvent`s suitable for Go channel consumption.

mod adapter;
mod events;
mod panels;

pub use adapter::{TUIAdapter, TUIConfig};
pub use events::{BudgetUpdate, EventBridge, ExecutionStatus, StatusUpdate, TUIEvent};
pub use panels::{
    BudgetPanelData, EventStyle, MemoryNodeView, MemoryPanelData, ReplEntry, ReplPanelData,
    ReplStatus, TierCounts, TracePanelData, TraceEventView,
};
