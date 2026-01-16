//! # rlm-core
//!
//! A unified RLM (Recursive Language Model) orchestration library supporting both
//! Claude Code plugins and agentic TUIs.
//!
//! ## Core Components
//!
//! - **Context**: Session context, messages, and tool outputs
//! - **Trajectory**: Observable execution events for streaming
//! - **Complexity**: Task complexity signals and activation decisions
//! - **Orchestrator**: Core RLM orchestration loop
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::{Orchestrator, SessionContext, PatternClassifier};
//!
//! let classifier = PatternClassifier::default();
//! let ctx = SessionContext::new();
//!
//! let decision = classifier.should_activate("Analyze the auth system", &ctx);
//! if decision.should_activate {
//!     println!("RLM activated: {}", decision.reason);
//! }
//! ```

pub mod complexity;
pub mod context;
pub mod error;
pub mod ffi;
pub mod lean;
pub mod llm;
pub mod memory;
pub mod orchestrator;
pub mod proof;
#[cfg(feature = "python")]
pub mod pybind;
pub mod repl;
pub mod spec_agent;
pub mod sync;
pub mod topos;
pub mod trajectory;

// Re-exports for convenience
pub use complexity::{ActivationDecision, PatternClassifier, TaskComplexitySignals};
pub use context::{Message, Role, SessionContext, ToolOutput};
pub use error::{Error, Result};
pub use llm::{
    AnthropicClient, ClientConfig, CompletionRequest, CompletionResponse, LLMClient, ModelSpec,
    ModelTier, Provider, QueryType, RoutingContext, SmartRouter,
};
pub use memory::{Node, NodeId, NodeType, SqliteMemoryStore, Tier};
pub use orchestrator::Orchestrator;
pub use repl::{ExecuteResult, ReplConfig, ReplHandle, ReplPool};
pub use topos::{
    IndexBuilder, LeanRef, Link, LinkIndex, LinkType, ToposClient, ToposClientConfig, ToposRef,
};
pub use proof::{
    AIAssistantConfig, AIProofAssistant, AutomationTier, ProofAttempt, ProofAutomation,
    ProofAutomationBuilder, ProofContext, ProofStats, ProofStrategy, SpecDomain, TacticResult,
};
pub use sync::{
    DriftReport, DriftType, DualTrackSync, FormalizationLevel, SyncDirection, SyncResult,
};
pub use trajectory::{TrajectoryEvent, TrajectoryEventType};
