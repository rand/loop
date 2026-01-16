//! Type definitions for DP Integration.
//!
//! This module defines the core types for tracking spec coverage and proof status
//! in the Disciplined Process workflow.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A SPEC-XX.YY identifier.
///
/// Format: SPEC-{major}.{minor} where major and minor are zero-padded numbers.
/// Examples: SPEC-01.01, SPEC-02.03, SPEC-10.15
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecId {
    /// Major version (XX in SPEC-XX.YY).
    pub major: u32,
    /// Minor version (YY in SPEC-XX.YY).
    pub minor: u32,
}

impl SpecId {
    /// Create a new SpecId.
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Parse a SPEC-XX.YY string.
    ///
    /// # Examples
    ///
    /// ```
    /// use rlm_core::dp_integration::SpecId;
    ///
    /// let spec = SpecId::parse("SPEC-01.02").unwrap();
    /// assert_eq!(spec.major, 1);
    /// assert_eq!(spec.minor, 2);
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Handle both "SPEC-XX.YY" and "[SPEC-XX.YY]" formats
        let spec_str = s
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(s);

        let rest = spec_str.strip_prefix("SPEC-")?;
        let (major_str, minor_str) = rest.split_once('.')?;

        let major = major_str.parse().ok()?;
        let minor = minor_str.parse().ok()?;

        Some(Self { major, minor })
    }

    /// Format as canonical string (SPEC-XX.YY).
    pub fn to_string_canonical(&self) -> String {
        format!("SPEC-{:02}.{:02}", self.major, self.minor)
    }

    /// Format as bracketed string ([SPEC-XX.YY]).
    pub fn to_string_bracketed(&self) -> String {
        format!("[{}]", self.to_string_canonical())
    }
}

impl std::fmt::Display for SpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_canonical())
    }
}

impl std::str::FromStr for SpecId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid SPEC-XX.YY format: {}", s))
    }
}

/// Status of a proof in a Lean file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    /// No Lean theorem exists for this spec.
    NotFormalized,
    /// Theorem is declared but has no proof body (uses sorry or is axiom).
    Stated,
    /// Proof exists but contains `sorry` placeholders.
    HasSorry,
    /// Proof is complete (no sorry, compiles successfully).
    Complete,
    /// Proof attempts have failed (type errors, etc.).
    Failed,
}

impl ProofStatus {
    /// Check if this status represents a complete proof.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Check if this status needs work.
    pub fn needs_work(&self) -> bool {
        matches!(self, Self::NotFormalized | Self::Stated | Self::HasSorry | Self::Failed)
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::NotFormalized => "Not formalized",
            Self::Stated => "Stated (no proof)",
            Self::HasSorry => "Has sorry",
            Self::Complete => "Complete",
            Self::Failed => "Failed",
        }
    }

    /// Get an emoji indicator for CLI output.
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::NotFormalized => "[ ]",
            Self::Stated => "[~]",
            Self::HasSorry => "[!]",
            Self::Complete => "[x]",
            Self::Failed => "[X]",
        }
    }
}

impl Default for ProofStatus {
    fn default() -> Self {
        Self::NotFormalized
    }
}

impl std::fmt::Display for ProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Information about a Lean theorem linked to a spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoremInfo {
    /// Theorem name (e.g., "auth_flow_correct").
    pub name: String,
    /// File containing the theorem.
    pub file: PathBuf,
    /// Line number in the file.
    pub line: u32,
    /// Namespace if any.
    pub namespace: Option<String>,
    /// The theorem statement.
    pub statement: Option<String>,
    /// Proof status.
    pub status: ProofStatus,
    /// Number of sorry placeholders (if HasSorry).
    pub sorry_count: u32,
    /// Error message if Failed.
    pub error_message: Option<String>,
}

impl TheoremInfo {
    /// Create a new TheoremInfo.
    pub fn new(name: impl Into<String>, file: impl Into<PathBuf>, line: u32) -> Self {
        Self {
            name: name.into(),
            file: file.into(),
            line,
            namespace: None,
            statement: None,
            status: ProofStatus::Stated,
            sorry_count: 0,
            error_message: None,
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the statement.
    pub fn with_statement(mut self, statement: impl Into<String>) -> Self {
        self.statement = Some(statement.into());
        self
    }

    /// Set the status.
    pub fn with_status(mut self, status: ProofStatus) -> Self {
        self.status = status;
        self
    }

    /// Set sorry count and update status accordingly.
    pub fn with_sorry_count(mut self, count: u32) -> Self {
        self.sorry_count = count;
        if count > 0 {
            self.status = ProofStatus::HasSorry;
        }
        self
    }

    /// Set error message and mark as failed.
    pub fn with_error(mut self, message: impl Into<String>) -> Self {
        self.error_message = Some(message.into());
        self.status = ProofStatus::Failed;
        self
    }

    /// Get the fully qualified name.
    pub fn qualified_name(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}.{}", ns, self.name),
            None => self.name.clone(),
        }
    }

    /// Get a formatted location string.
    pub fn location(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

/// Coverage information for a single spec requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecCoverage {
    /// The spec identifier (e.g., "SPEC-01.02").
    pub spec_id: SpecId,
    /// Human-readable requirement text.
    pub requirement_text: String,
    /// Lean theorem(s) that formalize this spec (may be multiple).
    pub theorems: Vec<TheoremInfo>,
    /// Overall proof status (worst status among all theorems).
    pub proof_status: ProofStatus,
    /// Source of the spec (e.g., "docs/spec/auth.md").
    pub spec_source: Option<PathBuf>,
    /// Line in spec source.
    pub spec_line: Option<u32>,
    /// Test files that trace to this spec.
    pub test_traces: Vec<TestTrace>,
}

impl SpecCoverage {
    /// Create a new SpecCoverage for an unformalized spec.
    pub fn new(spec_id: SpecId, requirement_text: impl Into<String>) -> Self {
        Self {
            spec_id,
            requirement_text: requirement_text.into(),
            theorems: Vec::new(),
            proof_status: ProofStatus::NotFormalized,
            spec_source: None,
            spec_line: None,
            test_traces: Vec::new(),
        }
    }

    /// Add a theorem that formalizes this spec.
    pub fn add_theorem(&mut self, theorem: TheoremInfo) {
        self.theorems.push(theorem);
        self.update_proof_status();
    }

    /// Add a test trace.
    pub fn add_test_trace(&mut self, trace: TestTrace) {
        self.test_traces.push(trace);
    }

    /// Update the overall proof status based on theorems.
    fn update_proof_status(&mut self) {
        if self.theorems.is_empty() {
            self.proof_status = ProofStatus::NotFormalized;
            return;
        }

        // Status priority: Failed > HasSorry > Stated > Complete
        let mut has_failed = false;
        let mut has_sorry = false;
        let mut has_stated = false;
        let mut all_complete = true;

        for theorem in &self.theorems {
            match theorem.status {
                ProofStatus::Failed => has_failed = true,
                ProofStatus::HasSorry => has_sorry = true,
                ProofStatus::Stated => has_stated = true,
                ProofStatus::Complete => {}
                ProofStatus::NotFormalized => all_complete = false,
            }
            if !matches!(theorem.status, ProofStatus::Complete) {
                all_complete = false;
            }
        }

        self.proof_status = if has_failed {
            ProofStatus::Failed
        } else if has_sorry {
            ProofStatus::HasSorry
        } else if has_stated {
            ProofStatus::Stated
        } else if all_complete {
            ProofStatus::Complete
        } else {
            ProofStatus::Stated
        };
    }

    /// Check if this spec is formalized (has at least one theorem).
    pub fn is_formalized(&self) -> bool {
        !self.theorems.is_empty()
    }

    /// Check if all proofs are complete.
    pub fn is_complete(&self) -> bool {
        self.proof_status == ProofStatus::Complete
    }

    /// Get the primary theorem (first one).
    pub fn primary_theorem(&self) -> Option<&TheoremInfo> {
        self.theorems.first()
    }
}

/// A test that traces to a spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTrace {
    /// Test function name.
    pub test_name: String,
    /// File containing the test.
    pub file: PathBuf,
    /// Line number.
    pub line: u32,
    /// Whether the test passes.
    pub passes: Option<bool>,
}

impl TestTrace {
    /// Create a new TestTrace.
    pub fn new(test_name: impl Into<String>, file: impl Into<PathBuf>, line: u32) -> Self {
        Self {
            test_name: test_name.into(),
            file: file.into(),
            line,
            passes: None,
        }
    }
}

/// Summary statistics for spec coverage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageSummary {
    /// Total number of specs.
    pub total_specs: usize,
    /// Number of specs with Lean formalization.
    pub formalized_count: usize,
    /// Number of specs with complete proofs.
    pub complete_count: usize,
    /// Number of specs with sorry placeholders.
    pub has_sorry_count: usize,
    /// Number of specs with failed proofs.
    pub failed_count: usize,
    /// Number of specs with only stated theorems.
    pub stated_count: usize,
    /// Number of specs with test traces.
    pub tested_count: usize,
}

impl CoverageSummary {
    /// Calculate the formalization percentage.
    pub fn formalization_percentage(&self) -> f64 {
        if self.total_specs == 0 {
            0.0
        } else {
            (self.formalized_count as f64 / self.total_specs as f64) * 100.0
        }
    }

    /// Calculate the completion percentage (complete proofs).
    pub fn completion_percentage(&self) -> f64 {
        if self.total_specs == 0 {
            0.0
        } else {
            (self.complete_count as f64 / self.total_specs as f64) * 100.0
        }
    }

    /// Calculate the proof completion percentage (among formalized).
    pub fn proof_completion_percentage(&self) -> f64 {
        if self.formalized_count == 0 {
            0.0
        } else {
            (self.complete_count as f64 / self.formalized_count as f64) * 100.0
        }
    }
}

/// Full coverage report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Coverage for each spec.
    pub specs: Vec<SpecCoverage>,
    /// Summary statistics.
    pub summary: CoverageSummary,
    /// When the report was generated.
    pub generated_at: String,
    /// Project root path.
    pub project_root: PathBuf,
    /// Lean files scanned.
    pub lean_files_scanned: Vec<PathBuf>,
    /// Spec files scanned.
    pub spec_files_scanned: Vec<PathBuf>,
}

impl CoverageReport {
    /// Create a new empty report.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            specs: Vec::new(),
            summary: CoverageSummary::default(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_root: project_root.into(),
            lean_files_scanned: Vec::new(),
            spec_files_scanned: Vec::new(),
        }
    }

    /// Add a spec coverage entry.
    pub fn add_spec(&mut self, coverage: SpecCoverage) {
        self.specs.push(coverage);
        self.update_summary();
    }

    /// Update summary statistics.
    pub fn update_summary(&mut self) {
        let mut summary = CoverageSummary::default();
        summary.total_specs = self.specs.len();

        for spec in &self.specs {
            if spec.is_formalized() {
                summary.formalized_count += 1;
            }
            if !spec.test_traces.is_empty() {
                summary.tested_count += 1;
            }

            match spec.proof_status {
                ProofStatus::Complete => summary.complete_count += 1,
                ProofStatus::HasSorry => summary.has_sorry_count += 1,
                ProofStatus::Failed => summary.failed_count += 1,
                ProofStatus::Stated => summary.stated_count += 1,
                ProofStatus::NotFormalized => {}
            }
        }

        self.summary = summary;
    }

    /// Get spec coverage by ID.
    pub fn get_spec(&self, spec_id: &SpecId) -> Option<&SpecCoverage> {
        self.specs.iter().find(|s| s.spec_id == *spec_id)
    }

    /// Get specs that need work (not complete).
    pub fn incomplete_specs(&self) -> Vec<&SpecCoverage> {
        self.specs.iter().filter(|s| !s.is_complete()).collect()
    }

    /// Get specs that are not formalized.
    pub fn unformalized_specs(&self) -> Vec<&SpecCoverage> {
        self.specs.iter().filter(|s| !s.is_formalized()).collect()
    }

    /// Get specs with sorry placeholders.
    pub fn specs_with_sorry(&self) -> Vec<&SpecCoverage> {
        self.specs
            .iter()
            .filter(|s| s.proof_status == ProofStatus::HasSorry)
            .collect()
    }

    /// Format the report as human-readable text.
    pub fn format_text(&self) -> String {
        let mut output = String::new();

        output.push_str("Spec Coverage Report\n");
        output.push_str("====================\n\n");

        for spec in &self.specs {
            output.push_str(&format!(
                "{}: {}\n",
                spec.spec_id,
                truncate_text(&spec.requirement_text, 50)
            ));
            output.push_str(&format!("  Status: {}\n", spec.proof_status));

            if let Some(theorem) = spec.primary_theorem() {
                output.push_str(&format!(
                    "  Theorem: {} ({})\n",
                    theorem.qualified_name(),
                    theorem.location()
                ));
            }

            if !spec.test_traces.is_empty() {
                output.push_str(&format!("  Tests: {} trace(s)\n", spec.test_traces.len()));
            }

            output.push('\n');
        }

        output.push_str(&format!(
            "Coverage: {}/{} ({:.0}%) formalized, {}/{} ({:.0}%) complete\n",
            self.summary.formalized_count,
            self.summary.total_specs,
            self.summary.formalization_percentage(),
            self.summary.complete_count,
            self.summary.total_specs,
            self.summary.completion_percentage()
        ));

        output
    }
}

/// Truncate text to a maximum length.
fn truncate_text(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_id_parse() {
        let spec = SpecId::parse("SPEC-01.02").unwrap();
        assert_eq!(spec.major, 1);
        assert_eq!(spec.minor, 2);

        let spec = SpecId::parse("[SPEC-10.05]").unwrap();
        assert_eq!(spec.major, 10);
        assert_eq!(spec.minor, 5);

        assert!(SpecId::parse("invalid").is_none());
        assert!(SpecId::parse("SPEC-").is_none());
        assert!(SpecId::parse("SPEC-01").is_none());
    }

    #[test]
    fn test_spec_id_format() {
        let spec = SpecId::new(1, 2);
        assert_eq!(spec.to_string_canonical(), "SPEC-01.02");
        assert_eq!(spec.to_string_bracketed(), "[SPEC-01.02]");

        let spec = SpecId::new(10, 15);
        assert_eq!(spec.to_string_canonical(), "SPEC-10.15");
    }

    #[test]
    fn test_proof_status() {
        assert!(ProofStatus::Complete.is_complete());
        assert!(!ProofStatus::HasSorry.is_complete());

        assert!(ProofStatus::NotFormalized.needs_work());
        assert!(ProofStatus::HasSorry.needs_work());
        assert!(!ProofStatus::Complete.needs_work());
    }

    #[test]
    fn test_theorem_info() {
        let theorem = TheoremInfo::new("auth_correct", "src/Auth.lean", 45)
            .with_namespace("Auth")
            .with_status(ProofStatus::Complete);

        assert_eq!(theorem.qualified_name(), "Auth.auth_correct");
        assert_eq!(theorem.location(), "src/Auth.lean:45");
    }

    #[test]
    fn test_spec_coverage_status() {
        let mut coverage = SpecCoverage::new(SpecId::new(1, 1), "Test requirement");
        assert_eq!(coverage.proof_status, ProofStatus::NotFormalized);

        coverage.add_theorem(
            TheoremInfo::new("theorem1", "test.lean", 10).with_status(ProofStatus::Complete),
        );
        assert_eq!(coverage.proof_status, ProofStatus::Complete);

        coverage.add_theorem(
            TheoremInfo::new("theorem2", "test.lean", 20).with_status(ProofStatus::HasSorry),
        );
        // Worst status wins
        assert_eq!(coverage.proof_status, ProofStatus::HasSorry);
    }

    #[test]
    fn test_coverage_summary() {
        let mut report = CoverageReport::new("/project");

        // Add a complete spec
        let mut spec1 = SpecCoverage::new(SpecId::new(1, 1), "Req 1");
        spec1.add_theorem(
            TheoremInfo::new("t1", "t.lean", 1).with_status(ProofStatus::Complete),
        );
        report.add_spec(spec1);

        // Add an incomplete spec
        let mut spec2 = SpecCoverage::new(SpecId::new(1, 2), "Req 2");
        spec2.add_theorem(
            TheoremInfo::new("t2", "t.lean", 2).with_status(ProofStatus::HasSorry),
        );
        report.add_spec(spec2);

        // Add an unformalized spec
        report.add_spec(SpecCoverage::new(SpecId::new(1, 3), "Req 3"));

        assert_eq!(report.summary.total_specs, 3);
        assert_eq!(report.summary.formalized_count, 2);
        assert_eq!(report.summary.complete_count, 1);
        assert_eq!(report.summary.has_sorry_count, 1);
    }
}
