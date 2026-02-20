//! Spec Agent module for transforming natural language to formal specifications.
//!
//! The Spec Agent provides a multi-phase workflow for converting natural language
//! requirements into formal specifications in both Topos (.tps) and Lean (.lean) formats.
//!
//! ## Overview
//!
//! The specification workflow consists of four phases:
//!
//! 1. **Intake**: Parse natural language requirements, extract intents and entities
//! 2. **Refine**: Generate clarifying questions, incorporate user answers
//! 3. **Formalize**: Generate Topos and Lean specifications with cross-references
//! 4. **Verify**: Type-check Lean specs, validate Topos specs, attempt proofs
//!
//! ## Formalization Levels
//!
//! The agent supports different levels of formalization:
//!
//! - **Types**: Only type definitions (structures, enums)
//! - **Invariants**: Types plus data constraints
//! - **Contracts**: Types, invariants, plus function pre/post conditions
//! - **FullProofs**: Complete formal proofs of correctness properties
//!
//! Completeness mode is configured independently from formalization level:
//! - **Baseline** (default): emits non-placeholder stubs without `TODO`/`sorry`
//! - **Placeholder**: explicit opt-in for draft specs with `TODO`/`sorry`
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::spec_agent::{SpecAgent, SpecAgentConfig, FormalizationLevel};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create an agent with contract-level formalization
//!     let config = SpecAgentConfig::default()
//!         .with_level(FormalizationLevel::Contracts);
//!     let mut agent = SpecAgent::new(config);
//!
//!     // Run the complete workflow
//!     let result = agent.run_workflow(
//!         "An Order has multiple items and a status. \
//!          Users can create orders and cancel them. \
//!          Each order must have at least one item."
//!     ).await?;
//!
//!     // Access generated specifications
//!     println!("Topos spec:\n{}", result.formalization.topos_content);
//!     println!("Lean spec:\n{}", result.formalization.lean_content);
//!
//!     // Check verification results
//!     if result.success() {
//!         println!("Specification verified successfully!");
//!     } else {
//!         for error in result.errors() {
//!             eprintln!("Error: {}", error);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Interactive Workflow
//!
//! For interactive use with clarifying questions:
//!
//! ```rust,ignore
//! use rlm_core::spec_agent::{SpecAgent, Answer};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut agent = SpecAgent::default_agent();
//!
//!     // Phase 1: Intake
//!     let mut ctx = agent.intake("Users can create and manage orders").await?;
//!
//!     // Phase 2: Refine with Q&A
//!     loop {
//!         let questions = agent.refine(&mut ctx, &[]).await?;
//!         if questions.is_empty() {
//!             break;
//!         }
//!
//!         // Present questions to user and collect answers
//!         let answers: Vec<Answer> = vec![]; // ... get from user
//!         agent.refine(&mut ctx, &answers).await?;
//!     }
//!
//!     // Phase 3: Formalize
//!     let formalization = agent.formalize(&ctx).await?;
//!
//!     // Phase 4: Verify
//!     let verification = agent.verify(&formalization).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Components
//!
//! - [`types`]: Core types (FormalizationLevel, SpecDomain, SpecContext, etc.)
//! - [`parser`]: Natural language parser for requirements extraction
//! - [`generators`]: Topos and Lean specification generators
//! - [`agent`]: The main SpecAgent implementation

pub mod agent;
pub mod generators;
pub mod parser;
pub mod types;

// Re-exports for convenience
pub use agent::{SpecAgent, WorkflowResult};
pub use generators::{
    CrossRefGenerator, GeneratedSpec, LeanGenerator, SpecGenerator, ToposGenerator,
};
pub use parser::{NLParser, ParseResult};
pub use types::{
    Ambiguity, AmbiguitySeverity, Answer, CompletenessMode, CrossReference, ExtractedRequirement,
    FormalizationLevel, FormalizationResult, ProofResult, ProofStrategy, Question,
    QuestionCategory, RequirementType, SpecAgentConfig, SpecContext, SpecDomain, SpecPhase,
    VerificationResult,
};
