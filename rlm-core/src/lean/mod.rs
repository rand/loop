//! Lean 4 REPL integration for formal verification.
//!
//! This module provides integration with the Lean 4 proof assistant via
//! the leanprover-community/repl JSON protocol.
//!
//! ## Components
//!
//! - **types**: Core types for Lean commands, responses, and proof states
//! - **repl**: REPL subprocess management and ReplEnvironment implementation

pub mod repl;
pub mod types;

pub use repl::{LeanRepl, LeanReplConfig, LeanReplPool};
pub use types::{
    Goal, LeanCommand, LeanEventMetadata, LeanMessage, LeanResponse, MessageSeverity, ProofState,
    ProofStep, Sorry, TacticSuggestion,
};
