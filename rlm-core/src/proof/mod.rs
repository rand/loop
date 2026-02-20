//! Proof automation pipeline for Lean 4.
//!
//! This module provides a tiered proof automation system that attempts
//! to prove goals automatically using increasingly sophisticated methods:
//!
//! 1. **Decidable tactics** (Tier 1): Guaranteed to terminate - `decide`, `omega`, `simp`, `rfl`
//! 2. **Automation tactics** (Tier 2): Search-based - `aesop`, `linarith`, `ring`
//! 3. **AI-assisted** (Tier 3): LLM-generated tactic sequences
//! 4. **Human loop** (Tier 4): `sorry` fallback marker for manual completion
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::proof::{ProofAutomation, ProofAutomationBuilder};
//! use rlm_core::lean::{LeanRepl, LeanReplConfig, Goal};
//!
//! // Create proof automation engine
//! let mut automation = ProofAutomationBuilder::new()
//!     .max_tactics_per_tier(10)
//!     .enable_ai(false)  // Disable AI for faster local testing
//!     .build();
//!
//! // Create a goal to prove
//! let goal = Goal::from_string("x + 0 = x")
//!     .with_hypothesis("x", "Nat");
//!
//! // Spawn Lean REPL and attempt proof
//! let config = LeanReplConfig::default();
//! let mut repl = LeanRepl::spawn(config)?;
//!
//! let result = automation.prove(&mut repl, &goal)?;
//! println!("Proof attempt: {}", result.summary());
//! ```
//!
//! ## Architecture
//!
//! The proof automation pipeline consists of:
//!
//! - **Types** (`types.rs`): Core data structures for proof attempts and strategies
//! - **Tactics** (`tactics.rs`): Tactic constants and selection functions
//! - **Engine** (`engine.rs`): Main proof automation orchestration
//! - **AI Assistant** (`ai_assistant.rs`): LLM-powered tactic suggestion
//!
//! ## Domain-Specific Strategies
//!
//! The system recognizes different proof domains and applies appropriate tactics:
//!
//! - **Arithmetic**: `omega`, `linarith`, `ring`, `norm_num`
//! - **Set Theory**: `ext`, `simp`, `aesop`
//! - **Logic**: `decide`, `tauto`, `constructor`, `cases`
//! - **Type Theory**: `rfl`, `congr`, `subst`
//!
//! ## Learning from Success
//!
//! The engine can learn from successful proofs:
//!
//! ```rust,ignore
//! // Record a successful proof pattern
//! automation.record_success(&goal, "omega", SpecDomain::Arithmetic);
//!
//! // The strategy for Arithmetic domain is updated
//! let strategies = automation.strategies_for_domain(SpecDomain::Arithmetic);
//! ```

pub mod ai_assistant;
pub mod engine;
pub mod session;
pub mod tactics;
pub mod types;

// Re-export main types
pub use ai_assistant::{AIAssistantConfig, AIProofAssistant};
pub use engine::{ProofAutomation, ProofAutomationBuilder, ProofAutomationConfig};
pub use session::{
    select_target, HelperLemma, HelperProofStatus, LimitReason, ProofSession, ProtocolConfig,
    ProtocolEnforcer, ProtocolError, ProofSessionStatus, SorryLocation, TacticAttempt,
    TacticOutcome,
};
pub use tactics::{
    domain_specific_tactics, tactics_for_goal, tactics_for_tier, AUTOMATION_TACTICS,
    DECIDABLE_TACTICS,
};
pub use types::{
    AutomationTier, DomainStats, ProofAttempt, ProofContext, ProofStats, ProofStrategy,
    SpecDomain, TacticResult, TierStats,
};
