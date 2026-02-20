//! Deciduous-style reasoning traces for provenance tracking.
//!
//! This module provides a system for recording and querying AI decision-making
//! processes as structured decision trees. Based on the Deciduous format for
//! representing reasoning traces.
//!
//! ## Core Concepts
//!
//! - **ReasoningTrace**: A complete decision tree capturing the reasoning process
//! - **DecisionNode**: Individual nodes (goals, decisions, options, actions, outcomes)
//! - **TraceEdge**: Relationships between nodes (spawns, chooses, rejects, etc.)
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::reasoning::{ReasoningTrace, TraceAnalyzer};
//!
//! // Create a new reasoning trace
//! let mut trace = ReasoningTrace::new("Implement user authentication", "session-123");
//! let root = trace.root_goal.clone();
//!
//! // Log a decision with options
//! let chosen = trace.log_decision(
//!     &root,
//!     "Choose authentication strategy",
//!     &["JWT tokens", "Session cookies", "OAuth2 only"],
//!     0,  // Choose JWT tokens
//!     "Stateless, works well with API clients",
//! );
//!
//! // Log the implementation action and outcome
//! trace.log_action(
//!     &chosen,
//!     "Implement JWT middleware in auth.rs",
//!     "Tests passing, integration verified",
//! );
//!
//! // Analyze the trace
//! let analyzer = TraceAnalyzer::new(&trace);
//! println!("Confidence: {:.0}%", analyzer.overall_confidence() * 100.0);
//! println!("{}", analyzer.narrative());
//!
//! // Export as Mermaid diagram
//! println!("{}", trace.to_mermaid());
//! ```
//!
//! ## Storage
//!
//! Traces are stored as hypergraph subgraphs in the memory system:
//!
//! ```rust,ignore
//! use rlm_core::reasoning::{ReasoningTraceStore, TraceQuery};
//!
//! let store = ReasoningTraceStore::in_memory()?;
//!
//! // Save a trace
//! store.save_trace(&trace)?;
//!
//! // Query traces
//! let traces = TraceQuery::new()
//!     .session("session-123")
//!     .goal_contains("auth")
//!     .execute(&store)?;
//!
//! // Find traces linked to a git commit
//! let commit_traces = store.find_by_commit("abc123")?;
//! ```
//!
//! ## Git Integration
//!
//! Traces can be linked to git commits for provenance tracking:
//!
//! ```rust,ignore
//! let trace = ReasoningTrace::new("Feature implementation", "session-456")
//!     .with_git_commit("abc123def")
//!     .with_git_branch("feature/auth");
//! ```

mod query;
mod store;
mod trace;
mod types;
mod visualize;

// Re-export main types
pub use query::{compare_traces, DecisionPath, TraceAnalyzer, TraceComparison, TraceQuery};
pub use store::{ReasoningTraceStore, TraceStoreStats};
pub use trace::{DecisionTree, ReasoningTrace, TraceStats};
pub use types::{
    DecisionNode, DecisionNodeId, DecisionNodeType, DecisionPoint, OptionStatus, TraceEdge,
    TraceEdgeLabel, TraceId,
};
pub use visualize::{
    DotConfig, HtmlConfig, HtmlTheme, NetworkXGraph, NetworkXGraphAttrs, NetworkXLink, NetworkXNode,
};
