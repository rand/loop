//! Review checks for formalization coverage.
//!
//! This module provides review checks that can be run as part of the DP
//! review phase to ensure adequate formalization coverage.

use serde::{Deserialize, Serialize};

use super::types::{CoverageReport, CoverageSummary, ProofStatus, SpecCoverage, SpecId};

/// Result of a review check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// Whether the review passed.
    pub passed: bool,
    /// Check name.
    pub check_name: String,
    /// Description of what was checked.
    pub description: String,
    /// Issues found (empty if passed).
    pub issues: Vec<ReviewIssue>,
    /// Warnings (non-blocking).
    pub warnings: Vec<String>,
    /// Summary message.
    pub summary: String,
}

impl ReviewResult {
    /// Create a passing result.
    pub fn pass(check_name: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            passed: true,
            check_name: check_name.into(),
            description: String::new(),
            issues: Vec::new(),
            warnings: Vec::new(),
            summary: summary.into(),
        }
    }

    /// Create a failing result.
    pub fn fail(
        check_name: impl Into<String>,
        summary: impl Into<String>,
        issues: Vec<ReviewIssue>,
    ) -> Self {
        Self {
            passed: false,
            check_name: check_name.into(),
            description: String::new(),
            issues,
            warnings: Vec::new(),
            summary: summary.into(),
        }
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// An issue found during review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Spec ID if applicable.
    pub spec_id: Option<SpecId>,
    /// Issue description.
    pub message: String,
    /// Suggested fix.
    pub suggestion: Option<String>,
    /// Whether this is blocking.
    pub blocking: bool,
}

impl ReviewIssue {
    /// Create a new blocking issue.
    pub fn blocking(message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Error,
            spec_id: None,
            message: message.into(),
            suggestion: None,
            blocking: true,
        }
    }

    /// Create a new non-blocking issue.
    pub fn non_blocking(message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            spec_id: None,
            message: message.into(),
            suggestion: None,
            blocking: false,
        }
    }

    /// Set spec ID.
    pub fn for_spec(mut self, spec_id: SpecId) -> Self {
        self.spec_id = Some(spec_id);
        self
    }

    /// Set suggestion.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: IssueSeverity) -> Self {
        self.severity = severity;
        self.blocking = matches!(severity, IssueSeverity::Error);
        self
    }
}

/// Severity of a review issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Informational only.
    Info,
    /// Warning - should be addressed but not blocking.
    Warning,
    /// Error - must be addressed, blocking.
    Error,
}

/// A review check that can be run.
#[derive(Debug, Clone)]
pub struct ReviewCheck {
    /// Check name.
    pub name: String,
    /// Check description.
    pub description: String,
    /// Whether this check is enabled.
    pub enabled: bool,
    /// Check configuration.
    pub config: ReviewCheckConfig,
}

/// Configuration for a review check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewCheckConfig {
    /// Minimum formalization percentage required.
    pub min_formalization_pct: Option<f64>,
    /// Minimum completion percentage required.
    pub min_completion_pct: Option<f64>,
    /// Maximum allowed sorry count.
    pub max_sorry_count: Option<u32>,
    /// Require all critical specs to be formalized.
    pub require_critical_formalized: bool,
    /// Critical spec IDs (must be formalized if require_critical_formalized).
    pub critical_specs: Vec<SpecId>,
    /// Block on any sorry placeholders.
    pub block_on_sorry: bool,
    /// Block on any failed proofs.
    pub block_on_failed: bool,
}

impl Default for ReviewCheckConfig {
    fn default() -> Self {
        Self {
            min_formalization_pct: Some(50.0),
            min_completion_pct: None,
            max_sorry_count: None,
            require_critical_formalized: true,
            critical_specs: Vec::new(),
            block_on_sorry: false,
            block_on_failed: true,
        }
    }
}

impl ReviewCheckConfig {
    /// Strict configuration - requires high coverage and no sorries.
    pub fn strict() -> Self {
        Self {
            min_formalization_pct: Some(80.0),
            min_completion_pct: Some(70.0),
            max_sorry_count: Some(0),
            require_critical_formalized: true,
            critical_specs: Vec::new(),
            block_on_sorry: true,
            block_on_failed: true,
        }
    }

    /// Lenient configuration - just checks for basic formalization.
    pub fn lenient() -> Self {
        Self {
            min_formalization_pct: Some(25.0),
            min_completion_pct: None,
            max_sorry_count: None,
            require_critical_formalized: false,
            critical_specs: Vec::new(),
            block_on_sorry: false,
            block_on_failed: false,
        }
    }
}

/// Runs formalization review checks.
pub struct FormalizationReview {
    /// Check configuration.
    config: ReviewCheckConfig,
}

impl FormalizationReview {
    /// Create a new review with default configuration.
    pub fn new() -> Self {
        Self {
            config: ReviewCheckConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: ReviewCheckConfig) -> Self {
        Self { config }
    }

    /// Set minimum formalization percentage.
    pub fn min_formalization(mut self, pct: f64) -> Self {
        self.config.min_formalization_pct = Some(pct);
        self
    }

    /// Set minimum completion percentage.
    pub fn min_completion(mut self, pct: f64) -> Self {
        self.config.min_completion_pct = Some(pct);
        self
    }

    /// Add a critical spec that must be formalized.
    pub fn require_spec(mut self, spec_id: SpecId) -> Self {
        self.config.critical_specs.push(spec_id);
        self.config.require_critical_formalized = true;
        self
    }

    /// Block on any sorry placeholders.
    pub fn block_on_sorry(mut self) -> Self {
        self.config.block_on_sorry = true;
        self
    }

    /// Run all review checks on a coverage report.
    pub fn review(&self, report: &CoverageReport) -> Vec<ReviewResult> {
        let mut results = Vec::new();

        // Check formalization coverage
        results.push(self.check_formalization_coverage(&report.summary));

        // Check proof completion
        results.push(self.check_proof_completion(&report.summary));

        // Check critical specs
        if self.config.require_critical_formalized && !self.config.critical_specs.is_empty() {
            results.push(self.check_critical_specs(&report.specs));
        }

        // Check for sorry placeholders
        if self.config.block_on_sorry || self.config.max_sorry_count.is_some() {
            results.push(self.check_sorry_status(&report.specs));
        }

        // Check for failed proofs
        if self.config.block_on_failed {
            results.push(self.check_failed_proofs(&report.specs));
        }

        results
    }

    /// Check formalization coverage percentage.
    fn check_formalization_coverage(&self, summary: &CoverageSummary) -> ReviewResult {
        let pct = summary.formalization_percentage();

        if let Some(min_pct) = self.config.min_formalization_pct {
            if pct < min_pct {
                let issue = ReviewIssue::blocking(format!(
                    "Formalization coverage {:.1}% is below minimum {:.1}%",
                    pct, min_pct
                ))
                .with_suggestion(format!(
                    "Add Lean theorems for {} more specs",
                    ((min_pct - pct) / 100.0 * summary.total_specs as f64).ceil() as u32
                ));

                return ReviewResult::fail(
                    "formalization_coverage",
                    format!(
                        "Formalization coverage {:.1}% < {:.1}% required",
                        pct, min_pct
                    ),
                    vec![issue],
                )
                .with_description("Checks that enough specs have Lean formalizations");
            }
        }

        ReviewResult::pass(
            "formalization_coverage",
            format!(
                "Formalization coverage {:.1}% ({}/{})",
                pct, summary.formalized_count, summary.total_specs
            ),
        )
        .with_description("Checks that enough specs have Lean formalizations")
    }

    /// Check proof completion percentage.
    fn check_proof_completion(&self, summary: &CoverageSummary) -> ReviewResult {
        let pct = summary.completion_percentage();

        if let Some(min_pct) = self.config.min_completion_pct {
            if pct < min_pct {
                let issue = ReviewIssue::blocking(format!(
                    "Proof completion {:.1}% is below minimum {:.1}%",
                    pct, min_pct
                ))
                .with_suggestion("Complete proofs for theorems with sorry placeholders");

                return ReviewResult::fail(
                    "proof_completion",
                    format!("Proof completion {:.1}% < {:.1}% required", pct, min_pct),
                    vec![issue],
                )
                .with_description("Checks that enough proofs are complete (no sorry)");
            }
        }

        ReviewResult::pass(
            "proof_completion",
            format!(
                "Proof completion {:.1}% ({}/{})",
                pct, summary.complete_count, summary.total_specs
            ),
        )
        .with_description("Checks that enough proofs are complete (no sorry)")
    }

    /// Check that critical specs are formalized.
    fn check_critical_specs(&self, specs: &[SpecCoverage]) -> ReviewResult {
        let mut issues = Vec::new();

        for critical_id in &self.config.critical_specs {
            let spec = specs.iter().find(|s| s.spec_id == *critical_id);

            match spec {
                None => {
                    issues.push(
                        ReviewIssue::blocking(format!(
                            "Critical spec {} not found in coverage report",
                            critical_id
                        ))
                        .for_spec(critical_id.clone())
                        .with_suggestion("Add the spec to your specification documents"),
                    );
                }
                Some(s) if !s.is_formalized() => {
                    issues.push(
                        ReviewIssue::blocking(format!(
                            "Critical spec {} is not formalized",
                            critical_id
                        ))
                        .for_spec(critical_id.clone())
                        .with_suggestion(format!(
                            "Add a Lean theorem for: {}",
                            truncate(&s.requirement_text, 50)
                        )),
                    );
                }
                _ => {}
            }
        }

        if issues.is_empty() {
            ReviewResult::pass(
                "critical_specs",
                format!(
                    "All {} critical specs are formalized",
                    self.config.critical_specs.len()
                ),
            )
            .with_description("Checks that designated critical specs have Lean formalizations")
        } else {
            ReviewResult::fail(
                "critical_specs",
                format!("{} critical spec(s) need attention", issues.len()),
                issues,
            )
            .with_description("Checks that designated critical specs have Lean formalizations")
        }
    }

    /// Check sorry placeholder status.
    fn check_sorry_status(&self, specs: &[SpecCoverage]) -> ReviewResult {
        let specs_with_sorry: Vec<_> = specs
            .iter()
            .filter(|s| s.proof_status == ProofStatus::HasSorry)
            .collect();

        let total_sorries: u32 = specs_with_sorry
            .iter()
            .flat_map(|s| &s.theorems)
            .map(|t| t.sorry_count)
            .sum();

        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check max sorry count
        if let Some(max) = self.config.max_sorry_count {
            if total_sorries > max {
                issues.push(
                    ReviewIssue::blocking(format!(
                        "Total sorry count {} exceeds maximum {}",
                        total_sorries, max
                    ))
                    .with_suggestion("Complete the proofs or remove unnecessary theorems"),
                );
            }
        }

        // Block on any sorry if configured
        if self.config.block_on_sorry && !specs_with_sorry.is_empty() {
            for spec in &specs_with_sorry {
                issues.push(
                    ReviewIssue::blocking(format!("Spec {} has incomplete proofs", spec.spec_id))
                        .for_spec(spec.spec_id.clone())
                        .with_suggestion("Complete all sorry placeholders before merging"),
                );
            }
        } else if !specs_with_sorry.is_empty() {
            for spec in &specs_with_sorry {
                warnings.push(format!(
                    "{}: {} sorry placeholder(s)",
                    spec.spec_id,
                    spec.theorems.iter().map(|t| t.sorry_count).sum::<u32>()
                ));
            }
        }

        if issues.is_empty() {
            let mut result = ReviewResult::pass(
                "sorry_status",
                if total_sorries == 0 {
                    "No sorry placeholders found".to_string()
                } else {
                    format!(
                        "{} sorry placeholder(s) in {} spec(s) (non-blocking)",
                        total_sorries,
                        specs_with_sorry.len()
                    )
                },
            )
            .with_description("Checks for sorry placeholders in proofs");

            for warning in warnings {
                result = result.with_warning(warning);
            }

            result
        } else {
            ReviewResult::fail(
                "sorry_status",
                format!("{} sorry-related issue(s)", issues.len()),
                issues,
            )
            .with_description("Checks for sorry placeholders in proofs")
        }
    }

    /// Check for failed proofs.
    fn check_failed_proofs(&self, specs: &[SpecCoverage]) -> ReviewResult {
        let failed_specs: Vec<_> = specs
            .iter()
            .filter(|s| s.proof_status == ProofStatus::Failed)
            .collect();

        if failed_specs.is_empty() {
            return ReviewResult::pass("failed_proofs", "No failed proofs")
                .with_description("Checks that no proofs have type errors");
        }

        let issues: Vec<_> = failed_specs
            .iter()
            .map(|spec| {
                let error_msg = spec
                    .theorems
                    .iter()
                    .filter_map(|t| t.error_message.as_ref())
                    .next()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default();

                ReviewIssue::blocking(format!(
                    "Spec {} has failed proof{}",
                    spec.spec_id, error_msg
                ))
                .for_spec(spec.spec_id.clone())
                .with_suggestion("Fix the type errors in the proof")
            })
            .collect();

        ReviewResult::fail(
            "failed_proofs",
            format!("{} spec(s) have failed proofs", failed_specs.len()),
            issues,
        )
        .with_description("Checks that no proofs have type errors")
    }

    /// Run review and return overall pass/fail with summary.
    pub fn run(&self, report: &CoverageReport) -> (bool, String) {
        let results = self.review(report);

        let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();

        if failed.is_empty() {
            (true, "All formalization checks passed".to_string())
        } else {
            let summary = failed
                .iter()
                .map(|r| format!("- {}: {}", r.check_name, r.summary))
                .collect::<Vec<_>>()
                .join("\n");

            (false, format!("Formalization review failed:\n{}", summary))
        }
    }
}

impl Default for FormalizationReview {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate a string for display.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dp_integration::types::TheoremInfo;
    use std::path::PathBuf;

    fn make_report(specs: Vec<SpecCoverage>) -> CoverageReport {
        let mut report = CoverageReport::new("/project");
        for spec in specs {
            report.add_spec(spec);
        }
        report
    }

    fn make_complete_spec(major: u32, minor: u32) -> SpecCoverage {
        let mut spec = SpecCoverage::new(SpecId::new(major, minor), "Test requirement");
        spec.add_theorem(
            TheoremInfo::new("theorem", "test.lean", 1).with_status(ProofStatus::Complete),
        );
        spec
    }

    fn make_sorry_spec(major: u32, minor: u32) -> SpecCoverage {
        let mut spec = SpecCoverage::new(SpecId::new(major, minor), "Test requirement");
        spec.add_theorem(
            TheoremInfo::new("theorem", "test.lean", 1)
                .with_status(ProofStatus::HasSorry)
                .with_sorry_count(2),
        );
        spec
    }

    fn make_unformalized_spec(major: u32, minor: u32) -> SpecCoverage {
        SpecCoverage::new(SpecId::new(major, minor), "Test requirement")
    }

    #[test]
    fn test_formalization_coverage_pass() {
        let report = make_report(vec![
            make_complete_spec(1, 1),
            make_complete_spec(1, 2),
            make_unformalized_spec(1, 3),
        ]);

        let review = FormalizationReview::new().min_formalization(50.0);
        let (passed, _) = review.run(&report);

        // 2/3 = 66.7% >= 50%
        assert!(passed);
    }

    #[test]
    fn test_formalization_coverage_fail() {
        let report = make_report(vec![
            make_complete_spec(1, 1),
            make_unformalized_spec(1, 2),
            make_unformalized_spec(1, 3),
            make_unformalized_spec(1, 4),
        ]);

        let review = FormalizationReview::new().min_formalization(50.0);
        let (passed, _) = review.run(&report);

        // 1/4 = 25% < 50%
        assert!(!passed);
    }

    #[test]
    fn test_critical_specs_pass() {
        let report = make_report(vec![make_complete_spec(1, 1), make_complete_spec(1, 2)]);

        let review = FormalizationReview::new()
            .require_spec(SpecId::new(1, 1))
            .require_spec(SpecId::new(1, 2));

        let results = review.review(&report);
        let critical_result = results.iter().find(|r| r.check_name == "critical_specs");

        assert!(critical_result.map(|r| r.passed).unwrap_or(false));
    }

    #[test]
    fn test_critical_specs_fail() {
        let report = make_report(vec![make_complete_spec(1, 1), make_unformalized_spec(1, 2)]);

        let review = FormalizationReview::new()
            .require_spec(SpecId::new(1, 1))
            .require_spec(SpecId::new(1, 2));

        let results = review.review(&report);
        let critical_result = results.iter().find(|r| r.check_name == "critical_specs");

        assert!(!critical_result.map(|r| r.passed).unwrap_or(true));
    }

    #[test]
    fn test_sorry_blocking() {
        let report = make_report(vec![make_complete_spec(1, 1), make_sorry_spec(1, 2)]);

        let review = FormalizationReview::new().block_on_sorry();

        let (passed, _) = review.run(&report);
        assert!(!passed);
    }

    #[test]
    fn test_sorry_non_blocking() {
        let report = make_report(vec![make_complete_spec(1, 1), make_sorry_spec(1, 2)]);

        // Use a config that checks sorry but doesn't block on it
        let config = ReviewCheckConfig {
            min_formalization_pct: Some(25.0),
            min_completion_pct: None,
            max_sorry_count: Some(100), // Set high limit to trigger check but not fail
            require_critical_formalized: false,
            critical_specs: Vec::new(),
            block_on_sorry: false,
            block_on_failed: false,
        };
        let review = FormalizationReview::with_config(config);

        let results = review.review(&report);
        let sorry_result = results.iter().find(|r| r.check_name == "sorry_status");

        // Should pass but have warnings
        assert!(
            sorry_result.is_some(),
            "sorry_status check should be present"
        );
        assert!(sorry_result.map(|r| r.passed).unwrap_or(false));
        assert!(!sorry_result.map(|r| r.warnings.is_empty()).unwrap_or(true));
    }

    #[test]
    fn test_strict_config() {
        let report = make_report(vec![
            make_complete_spec(1, 1),
            make_complete_spec(1, 2),
            make_sorry_spec(1, 3),
        ]);

        let review = FormalizationReview::with_config(ReviewCheckConfig::strict());
        let (passed, _) = review.run(&report);

        // Should fail due to sorry and possibly coverage
        assert!(!passed);
    }

    #[test]
    fn test_review_issue_builder() {
        let issue = ReviewIssue::blocking("Test issue")
            .for_spec(SpecId::new(1, 1))
            .with_suggestion("Fix it")
            .with_severity(IssueSeverity::Error);

        assert!(issue.blocking);
        assert_eq!(issue.spec_id, Some(SpecId::new(1, 1)));
        assert_eq!(issue.suggestion, Some("Fix it".to_string()));
    }
}
