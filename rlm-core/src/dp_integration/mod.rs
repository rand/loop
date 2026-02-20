//! DP Integration module for formal specifications.
//!
//! This module integrates formal specifications (Lean) with the Disciplined Process (DP)
//! workflow, enabling:
//!
//! - **Spec coverage tracking**: Which SPEC-XX.YY requirements are formalized in Lean
//! - **Proof status tracking**: Complete, sorry, failed status for theorems
//! - **CLI command support**: `/dp:spec coverage --with-lean`, `/dp:spec verify --lean`
//! - **Evidence gathering**: For proof completion
//! - **Review checks**: For formalization coverage
//!
//! ## SPEC-XX.YY Integration
//!
//! Lean theorems reference SPEC-XX.YY via comments:
//!
//! ```lean
//! /--
//! SPEC-01.02: Session timeout validation
//! Sessions must timeout after inactivity period.
//! -/
//! theorem session_timeout_correct : ... := by
//!   ...
//! ```
//!
//! Tests can @trace to SPEC-XX.YY:
//!
//! ```rust,ignore
//! #[test]
//! fn test_session_timeout() {
//!     // @trace SPEC-01.02
//!     assert!(session.is_expired_after(timeout));
//! }
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::dp_integration::{DPIntegration, CoverageReport, ProofStatus};
//!
//! // Scan Lean files and specs
//! let mut dp = DPIntegration::new("/path/to/project");
//! dp.scan().await?;
//!
//! // Generate coverage report
//! let report = dp.coverage_report();
//! println!("Coverage: {}/{} ({:.0}%)",
//!     report.formalized_count,
//!     report.total_specs,
//!     report.coverage_percentage());
//!
//! // Check specific spec
//! let coverage = dp.get_spec_coverage("SPEC-01.02");
//! match coverage.proof_status {
//!     ProofStatus::Complete => println!("Fully proven!"),
//!     ProofStatus::HasSorry => println!("Has sorry - incomplete"),
//!     _ => println!("Status: {:?}", coverage.proof_status),
//! }
//! ```

pub mod commands;
pub mod coverage;
pub mod proof_status;
pub mod review;
pub mod types;

// Re-exports for convenience
pub use commands::{DPCommand, DPCommandHandler, DPCommandResult};
pub use coverage::{CoverageScanner, SpecCoverageTracker};
pub use proof_status::{LeanProofScanner, ProofEvidence};
pub use review::{FormalizationReview, ReviewCheck, ReviewResult};
pub use types::{CoverageReport, CoverageSummary, ProofStatus, SpecCoverage, SpecId, TheoremInfo};
