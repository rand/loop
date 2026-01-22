//! Adversarial validation for LLM outputs.
//!
//! This module provides adversarial review capabilities using a different
//! model provider (Google/Gemini) to validate outputs from the primary
//! model (Anthropic/Claude). This cross-provider validation helps catch
//! issues that might be missed due to shared biases.
//!
//! ## Architecture
//!
//! The adversarial validation system consists of:
//!
//! 1. **ValidationContext**: Captures all information needed for review
//!    - Original request and response
//!    - Code context (files, diffs)
//!    - Tool outputs
//!    - Prior iterations for multi-round validation
//!
//! 2. **AdversarialValidator**: Core validation trait
//!    - Single-pass validation
//!    - Iterative validation until convergence
//!
//! 3. **ValidationStrategy**: Focused issue detection
//!    - CriticStrategy: General code review
//!    - SecurityStrategy: Security vulnerabilities
//!    - EdgeCaseStrategy: Missing boundary handling
//!    - TestingStrategy: Test coverage gaps
//!    - TraceabilityStrategy: Spec linkage verification
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::adversarial::{
//!     AdversarialConfig, GeminiValidator, ValidationContext, AdversarialValidator,
//! };
//!
//! // Create validator with Gemini
//! let config = AdversarialConfig {
//!     enabled: true,
//!     model: "gemini-2.0-flash".to_string(),
//!     max_iterations: 3,
//!     ..Default::default()
//! };
//!
//! let validator = GeminiValidator::new(&api_key, config)?;
//!
//! // Build validation context
//! let ctx = ValidationContext::new(
//!     "Fix the authentication bug",
//!     "I fixed the bug by removing the password check..."
//! )
//! .with_code_file(CodeFile::new("src/auth.rs", "fn login(...) { ... }"));
//!
//! // Validate
//! let result = validator.validate(&ctx).await?;
//!
//! if result.has_blocking_issues() {
//!     for issue in result.blocking_issues() {
//!         println!("[{}] {}: {}", issue.severity, issue.title, issue.description);
//!     }
//! }
//! ```
//!
//! ## Integration with Disciplined Process
//!
//! The adversarial module integrates with the disciplined process workflow:
//!
//! - **On Review**: Triggered by `/dp:review` command
//! - **On Commit**: Triggered before git commit (if configured)
//! - **Traceability**: Validates `@trace SPEC-XX.YY` markers
//!
//! ```rust,ignore
//! // In the review hook
//! if validator.should_validate("review") {
//!     let ctx = ValidationContext::new(request, response)
//!         .with_spec("SPEC-01.02");
//!
//!     let result = validator.validate(&ctx).await?;
//!
//!     if result.verdict == ValidationVerdict::Rejected {
//!         return Err("Blocking issues found".into());
//!     }
//! }
//! ```
//!
//! ## Fresh Context Invocation
//!
//! For true adversarial review, the validator uses "fresh context" - it doesn't
//! share conversation history with the primary model. This prevents the adversary
//! from being influenced by the primary's reasoning.
//!
//! ## Cost Considerations
//!
//! Adversarial validation adds latency and cost:
//!
//! - Gemini 2.0 Flash: ~$0.075/1M input, $0.30/1M output
//! - Typical review: ~5K input, ~2K output = ~$0.001 per review
//! - Multi-iteration: Up to 3x cost for max_iterations=3
//!
//! Configure triggers carefully to balance coverage vs. cost.

pub mod invoker;
pub mod strategies;
pub mod types;
pub mod validator;

// Re-exports
pub use invoker::{
    FreshContextInvoker, FreshInvokerBuilder, GeminiFreshInvoker, InvocationStats,
    PooledFreshInvoker,
};
pub use strategies::{
    CriticStrategy, EdgeCaseStrategy, PerformanceStrategy, SecurityStrategy, StrategyFactory,
    TestingStrategy, TraceabilityStrategy, ValidationStrategy,
};
pub use types::{
    AdversarialConfig, AdversarialTrigger, CodeFile, Issue, IssueCategory, IssueLocation,
    IssueSeverity, ToolOutput, ValidationContext, ValidationId, ValidationIteration,
    ValidationResult, ValidationStats, ValidationVerdict,
};
pub use validator::{AdversarialValidator, GeminiValidator};

#[cfg(test)]
pub use validator::MockValidator;
