//! Command handlers for DP integration.
//!
//! This module provides CLI command support for the DP workflow integration,
//! including `/dp:spec coverage --with-lean` and `/dp:spec verify --lean`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::coverage::CoverageScanner;
use super::proof_status::LeanProofScanner;
use super::review::{FormalizationReview, ReviewCheckConfig};
use super::types::{CoverageReport, ProofStatus, SpecId};

/// DP command types for spec integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DPCommand {
    /// Show spec coverage report.
    /// `/dp:spec coverage [--with-lean] [--format=text|json]`
    Coverage {
        /// Include Lean formalization status.
        with_lean: bool,
        /// Output format.
        format: OutputFormat,
        /// Filter by status.
        status_filter: Option<ProofStatus>,
    },

    /// Verify specs against Lean formalizations.
    /// `/dp:spec verify --lean [--spec=SPEC-XX.YY]`
    Verify {
        /// Verify with Lean REPL.
        lean: bool,
        /// Specific spec to verify.
        spec_id: Option<SpecId>,
        /// Run all review checks.
        review: bool,
    },

    /// List specs by status.
    /// `/dp:spec list [--uncovered|--incomplete|--sorry]`
    List {
        /// Show uncovered (unformalized) specs.
        uncovered: bool,
        /// Show incomplete (not complete) specs.
        incomplete: bool,
        /// Show specs with sorry.
        sorry: bool,
    },

    /// Show details for a specific spec.
    /// `/dp:spec show SPEC-XX.YY`
    Show {
        /// Spec ID to show.
        spec_id: SpecId,
    },

    /// Run formalization review checks.
    /// `/dp:review --lean`
    Review {
        /// Use strict review configuration.
        strict: bool,
        /// File issues as tasks for non-blocking problems.
        file_issues: bool,
    },
}

/// Output format for commands.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// JSON output.
    Json,
    /// Markdown output.
    Markdown,
}

/// Result of executing a DP command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DPCommandResult {
    /// Whether the command succeeded.
    pub success: bool,
    /// Command output (formatted).
    pub output: String,
    /// Structured data (for JSON output).
    pub data: Option<serde_json::Value>,
    /// Errors encountered.
    pub errors: Vec<String>,
    /// Warnings.
    pub warnings: Vec<String>,
}

impl DPCommandResult {
    /// Create a successful result.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            errors: vec![error.into()],
            data: None,
            warnings: Vec::new(),
        }
    }

    /// Add structured data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Handler for DP commands.
pub struct DPCommandHandler {
    /// Project root directory.
    #[allow(dead_code)] // Reserved for future path resolution
    project_root: PathBuf,
    /// Coverage scanner.
    scanner: CoverageScanner,
    /// Cached coverage report.
    cached_report: Option<CoverageReport>,
}

impl DPCommandHandler {
    /// Create a new command handler.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let project_root = project_root.into();
        let scanner = CoverageScanner::new(&project_root);
        Self {
            project_root,
            scanner,
            cached_report: None,
        }
    }

    /// Execute a DP command.
    pub fn execute(&mut self, command: DPCommand) -> Result<DPCommandResult> {
        match command {
            DPCommand::Coverage {
                with_lean,
                format,
                status_filter,
            } => self.cmd_coverage(with_lean, format, status_filter),

            DPCommand::Verify {
                lean,
                spec_id,
                review,
            } => self.cmd_verify(lean, spec_id, review),

            DPCommand::List {
                uncovered,
                incomplete,
                sorry,
            } => self.cmd_list(uncovered, incomplete, sorry),

            DPCommand::Show { spec_id } => self.cmd_show(spec_id),

            DPCommand::Review {
                strict,
                file_issues,
            } => self.cmd_review(strict, file_issues),
        }
    }

    /// Get or refresh the coverage report.
    fn get_report(&mut self) -> Result<&CoverageReport> {
        if self.cached_report.is_none() {
            self.cached_report = Some(self.scanner.scan()?);
        }
        Ok(self.cached_report.as_ref().unwrap())
    }

    /// Force refresh the coverage report.
    pub fn refresh(&mut self) -> Result<()> {
        self.cached_report = Some(self.scanner.scan()?);
        Ok(())
    }

    /// Handle coverage command.
    fn cmd_coverage(
        &mut self,
        with_lean: bool,
        format: OutputFormat,
        status_filter: Option<ProofStatus>,
    ) -> Result<DPCommandResult> {
        let report = self.get_report()?;

        let filtered_specs: Vec<_> = if let Some(status) = status_filter {
            report
                .specs
                .iter()
                .filter(|s| s.proof_status == status)
                .collect()
        } else {
            report.specs.iter().collect()
        };

        let output = match format {
            OutputFormat::Text => {
                let mut out = String::new();
                out.push_str("Spec Coverage Report\n");
                out.push_str("====================\n\n");

                for spec in &filtered_specs {
                    out.push_str(&format!(
                        "{} {}: {}\n",
                        spec.proof_status.indicator(),
                        spec.spec_id,
                        truncate(&spec.requirement_text, 50)
                    ));

                    if with_lean {
                        out.push_str(&format!("  Status: {}\n", spec.proof_status));

                        if let Some(theorem) = spec.primary_theorem() {
                            out.push_str(&format!(
                                "  Theorem: {} ({})\n",
                                theorem.qualified_name(),
                                theorem.location()
                            ));
                        }

                        if !spec.test_traces.is_empty() {
                            out.push_str(&format!("  Tests: {} trace(s)\n", spec.test_traces.len()));
                        }
                    }
                    out.push('\n');
                }

                out.push_str(&format!(
                    "\nCoverage: {}/{} ({:.0}%) formalized, {}/{} ({:.0}%) complete\n",
                    report.summary.formalized_count,
                    report.summary.total_specs,
                    report.summary.formalization_percentage(),
                    report.summary.complete_count,
                    report.summary.total_specs,
                    report.summary.completion_percentage()
                ));

                out
            }

            OutputFormat::Json => serde_json::to_string_pretty(&report)
                .map_err(|e| Error::Internal(e.to_string()))?,

            OutputFormat::Markdown => {
                let mut out = String::new();
                out.push_str("# Spec Coverage Report\n\n");

                out.push_str("| Spec ID | Status | Requirement | Theorem |\n");
                out.push_str("|---------|--------|-------------|----------|\n");

                for spec in &filtered_specs {
                    let theorem_info = spec
                        .primary_theorem()
                        .map(|t| format!("`{}`", t.qualified_name()))
                        .unwrap_or_else(|| "-".to_string());

                    out.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        spec.spec_id,
                        spec.proof_status.indicator(),
                        truncate(&spec.requirement_text, 40),
                        theorem_info
                    ));
                }

                out.push_str(&format!(
                    "\n**Coverage**: {}/{} ({:.0}%) formalized, {}/{} ({:.0}%) complete\n",
                    report.summary.formalized_count,
                    report.summary.total_specs,
                    report.summary.formalization_percentage(),
                    report.summary.complete_count,
                    report.summary.total_specs,
                    report.summary.completion_percentage()
                ));

                out
            }
        };

        let data = serde_json::to_value(report).ok();
        Ok(DPCommandResult::success(output).with_data(data.unwrap_or(serde_json::Value::Null)))
    }

    /// Handle verify command.
    fn cmd_verify(
        &mut self,
        lean: bool,
        spec_id: Option<SpecId>,
        review: bool,
    ) -> Result<DPCommandResult> {
        let report = self.get_report()?;

        let mut output = String::new();
        let mut all_passed = true;

        if let Some(id) = spec_id {
            // Verify specific spec
            let spec = report
                .specs
                .iter()
                .find(|s| s.spec_id == id)
                .ok_or_else(|| Error::Internal(format!("Spec {} not found", id)))?;

            output.push_str(&format!("Verifying {}...\n", id));

            if !spec.is_formalized() {
                output.push_str("  Status: Not formalized\n");
                all_passed = false;
            } else if lean {
                // Scan Lean files for detailed status
                let scanner = LeanProofScanner::new();
                for theorem in &spec.theorems {
                    let evidence = scanner.scan_file(&theorem.file)?;
                    let thm_evidence = evidence.iter().find(|e| e.theorem_name == theorem.name);

                    if let Some(ev) = thm_evidence {
                        output.push_str(&format!(
                            "  Theorem: {} - {}\n",
                            theorem.name, ev.status
                        ));
                        if ev.sorry_count > 0 {
                            output.push_str(&format!("    Sorry count: {}\n", ev.sorry_count));
                            all_passed = false;
                        }
                        if !ev.tactics_used.is_empty() {
                            output.push_str(&format!(
                                "    Tactics: {}\n",
                                ev.tactics_used.join(", ")
                            ));
                        }
                    }
                }
            } else {
                output.push_str(&format!("  Status: {}\n", spec.proof_status));
                if !spec.proof_status.is_complete() {
                    all_passed = false;
                }
            }
        } else {
            // Verify all specs
            output.push_str("Verifying all specs...\n\n");

            for spec in &report.specs {
                let status_char = if spec.is_complete() {
                    all_passed = all_passed && true;
                    "[x]"
                } else {
                    all_passed = false;
                    if spec.is_formalized() {
                        "[~]"
                    } else {
                        "[ ]"
                    }
                };

                output.push_str(&format!("{} {} - {}\n", status_char, spec.spec_id, spec.proof_status));
            }
        }

        if review {
            output.push_str("\n--- Review Checks ---\n\n");
            let review_handler = FormalizationReview::new();
            let results = review_handler.review(report);

            for result in &results {
                let status = if result.passed { "PASS" } else { "FAIL" };
                output.push_str(&format!("[{}] {}: {}\n", status, result.check_name, result.summary));

                for issue in &result.issues {
                    output.push_str(&format!("  - {}\n", issue.message));
                    if let Some(ref suggestion) = issue.suggestion {
                        output.push_str(&format!("    Suggestion: {}\n", suggestion));
                    }
                }

                for warning in &result.warnings {
                    output.push_str(&format!("  (warning) {}\n", warning));
                }
            }

            all_passed = all_passed && results.iter().all(|r| r.passed);
        }

        output.push_str(&format!(
            "\nVerification {}\n",
            if all_passed { "PASSED" } else { "FAILED" }
        ));

        let mut result = DPCommandResult::success(output);
        result.success = all_passed;
        Ok(result)
    }

    /// Handle list command.
    fn cmd_list(
        &mut self,
        uncovered: bool,
        incomplete: bool,
        sorry: bool,
    ) -> Result<DPCommandResult> {
        let report = self.get_report()?;

        let filter = |spec: &&super::types::SpecCoverage| -> bool {
            if uncovered && !spec.is_formalized() {
                return true;
            }
            if incomplete && !spec.is_complete() && spec.is_formalized() {
                return true;
            }
            if sorry && spec.proof_status == ProofStatus::HasSorry {
                return true;
            }
            // If no filter specified, show all
            !uncovered && !incomplete && !sorry
        };

        let filtered: Vec<_> = report.specs.iter().filter(filter).collect();

        let mut output = String::new();

        let title = if uncovered {
            "Uncovered Specs (no Lean formalization)"
        } else if incomplete {
            "Incomplete Specs (formalized but not proven)"
        } else if sorry {
            "Specs with Sorry Placeholders"
        } else {
            "All Specs"
        };

        output.push_str(&format!("{}\n{}\n\n", title, "=".repeat(title.len())));

        if filtered.is_empty() {
            output.push_str("(none)\n");
        } else {
            for spec in &filtered {
                output.push_str(&format!(
                    "{} {}: {}\n",
                    spec.proof_status.indicator(),
                    spec.spec_id,
                    truncate(&spec.requirement_text, 60)
                ));
            }
            output.push_str(&format!("\nTotal: {}\n", filtered.len()));
        }

        Ok(DPCommandResult::success(output))
    }

    /// Handle show command.
    fn cmd_show(&mut self, spec_id: SpecId) -> Result<DPCommandResult> {
        let report = self.get_report()?;

        let spec = report
            .specs
            .iter()
            .find(|s| s.spec_id == spec_id)
            .ok_or_else(|| Error::Internal(format!("Spec {} not found", spec_id)))?;

        let mut output = String::new();

        output.push_str(&format!("{}\n", spec.spec_id));
        output.push_str(&format!("{}\n\n", "=".repeat(spec.spec_id.to_string().len())));

        output.push_str(&format!("Requirement: {}\n", spec.requirement_text));
        output.push_str(&format!("Status: {}\n", spec.proof_status));

        if let Some(ref source) = spec.spec_source {
            output.push_str(&format!(
                "Source: {}:{}\n",
                source.display(),
                spec.spec_line.unwrap_or(0)
            ));
        }

        output.push('\n');

        if spec.theorems.is_empty() {
            output.push_str("Theorems: (none - not formalized)\n");
        } else {
            output.push_str("Theorems:\n");
            for theorem in &spec.theorems {
                output.push_str(&format!(
                    "  - {} ({}) - {}\n",
                    theorem.qualified_name(),
                    theorem.location(),
                    theorem.status
                ));
                if theorem.sorry_count > 0 {
                    output.push_str(&format!("    Sorry count: {}\n", theorem.sorry_count));
                }
                if let Some(ref stmt) = theorem.statement {
                    output.push_str(&format!("    Statement: {}\n", truncate(stmt, 60)));
                }
            }
        }

        if !spec.test_traces.is_empty() {
            output.push_str("\nTest Traces:\n");
            for trace in &spec.test_traces {
                output.push_str(&format!(
                    "  - {} ({}:{})\n",
                    trace.test_name,
                    trace.file.display(),
                    trace.line
                ));
            }
        }

        let data = serde_json::to_value(spec).ok();
        Ok(DPCommandResult::success(output).with_data(data.unwrap_or(serde_json::Value::Null)))
    }

    /// Handle review command.
    fn cmd_review(&mut self, strict: bool, file_issues: bool) -> Result<DPCommandResult> {
        let report = self.get_report()?;

        let config = if strict {
            ReviewCheckConfig::strict()
        } else {
            ReviewCheckConfig::default()
        };

        let review = FormalizationReview::with_config(config);
        let results = review.review(report);

        let mut output = String::new();
        output.push_str("Formalization Review\n");
        output.push_str("====================\n\n");

        let mut issues_to_file = Vec::new();

        for result in &results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            output.push_str(&format!("[{}] {}\n", status, result.check_name));
            output.push_str(&format!("     {}\n", result.summary));

            if !result.description.is_empty() {
                output.push_str(&format!("     ({})\n", result.description));
            }

            for issue in &result.issues {
                let severity = match issue.severity {
                    super::review::IssueSeverity::Error => "ERROR",
                    super::review::IssueSeverity::Warning => "WARN",
                    super::review::IssueSeverity::Info => "INFO",
                };

                output.push_str(&format!("     [{:5}] {}\n", severity, issue.message));

                if let Some(ref suggestion) = issue.suggestion {
                    output.push_str(&format!("             -> {}\n", suggestion));
                }

                if file_issues && !issue.blocking {
                    issues_to_file.push(issue.clone());
                }
            }

            for warning in &result.warnings {
                output.push_str(&format!("     [WARN ] {}\n", warning));
            }

            output.push('\n');
        }

        let all_passed = results.iter().all(|r| r.passed);
        output.push_str(&format!(
            "Overall: {}\n",
            if all_passed { "PASSED" } else { "FAILED" }
        ));

        if file_issues && !issues_to_file.is_empty() {
            output.push_str(&format!(
                "\n{} non-blocking issue(s) would be filed as tasks.\n",
                issues_to_file.len()
            ));
            // In a real implementation, this would create DP tasks
        }

        let mut cmd_result = DPCommandResult::success(output);
        cmd_result.success = all_passed;

        let data = serde_json::json!({
            "passed": all_passed,
            "results": results.iter().map(|r| {
                serde_json::json!({
                    "check": r.check_name,
                    "passed": r.passed,
                    "summary": r.summary,
                    "issues_count": r.issues.len(),
                    "warnings_count": r.warnings.len(),
                })
            }).collect::<Vec<_>>(),
        });

        Ok(cmd_result.with_data(data))
    }

    /// Parse a command string into a DPCommand.
    pub fn parse_command(input: &str) -> Result<DPCommand> {
        let input = input.trim();

        // /dp:spec coverage [--with-lean] [--format=text|json|markdown]
        if input.starts_with("/dp:spec coverage") {
            let with_lean = input.contains("--with-lean");
            let format = if input.contains("--format=json") {
                OutputFormat::Json
            } else if input.contains("--format=markdown") || input.contains("--format=md") {
                OutputFormat::Markdown
            } else {
                OutputFormat::Text
            };

            let status_filter = if input.contains("--sorry") {
                Some(ProofStatus::HasSorry)
            } else if input.contains("--complete") {
                Some(ProofStatus::Complete)
            } else if input.contains("--incomplete") {
                Some(ProofStatus::Stated)
            } else {
                None
            };

            return Ok(DPCommand::Coverage {
                with_lean,
                format,
                status_filter,
            });
        }

        // /dp:spec verify [--lean] [--spec=SPEC-XX.YY] [--review]
        if input.starts_with("/dp:spec verify") {
            let lean = input.contains("--lean");
            let review = input.contains("--review");

            let spec_id = extract_spec_arg(input, "--spec=");

            return Ok(DPCommand::Verify {
                lean,
                spec_id,
                review,
            });
        }

        // /dp:spec list [--uncovered|--incomplete|--sorry]
        if input.starts_with("/dp:spec list") {
            return Ok(DPCommand::List {
                uncovered: input.contains("--uncovered"),
                incomplete: input.contains("--incomplete"),
                sorry: input.contains("--sorry"),
            });
        }

        // /dp:spec show SPEC-XX.YY
        if input.starts_with("/dp:spec show") {
            let spec_str = input
                .strip_prefix("/dp:spec show")
                .map(|s| s.trim())
                .unwrap_or("");

            let spec_id = SpecId::parse(spec_str)
                .ok_or_else(|| Error::Internal(format!("Invalid spec ID: {}", spec_str)))?;

            return Ok(DPCommand::Show { spec_id });
        }

        // /dp:review [--lean] [--strict] [--file-issues]
        if input.starts_with("/dp:review") {
            return Ok(DPCommand::Review {
                strict: input.contains("--strict"),
                file_issues: input.contains("--file-issues"),
            });
        }

        Err(Error::Internal(format!("Unknown command: {}", input)))
    }
}

/// Extract a spec ID from a command argument like --spec=SPEC-01.02.
fn extract_spec_arg(input: &str, prefix: &str) -> Option<SpecId> {
    input.find(prefix).and_then(|start| {
        let rest = &input[start + prefix.len()..];
        let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        SpecId::parse(&rest[..end])
    })
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

    #[test]
    fn test_parse_coverage_command() {
        let cmd = DPCommandHandler::parse_command("/dp:spec coverage").unwrap();
        assert!(matches!(cmd, DPCommand::Coverage { with_lean: false, .. }));

        let cmd = DPCommandHandler::parse_command("/dp:spec coverage --with-lean").unwrap();
        assert!(matches!(cmd, DPCommand::Coverage { with_lean: true, .. }));

        let cmd =
            DPCommandHandler::parse_command("/dp:spec coverage --with-lean --format=json").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::Coverage {
                with_lean: true,
                format: OutputFormat::Json,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_verify_command() {
        let cmd = DPCommandHandler::parse_command("/dp:spec verify --lean").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::Verify {
                lean: true,
                spec_id: None,
                ..
            }
        ));

        let cmd =
            DPCommandHandler::parse_command("/dp:spec verify --lean --spec=SPEC-01.02").unwrap();
        if let DPCommand::Verify { spec_id, .. } = cmd {
            assert_eq!(spec_id, Some(SpecId::new(1, 2)));
        } else {
            panic!("Expected Verify command");
        }
    }

    #[test]
    fn test_parse_list_command() {
        let cmd = DPCommandHandler::parse_command("/dp:spec list --uncovered").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::List {
                uncovered: true,
                incomplete: false,
                sorry: false
            }
        ));

        let cmd = DPCommandHandler::parse_command("/dp:spec list --sorry").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::List {
                sorry: true,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_show_command() {
        let cmd = DPCommandHandler::parse_command("/dp:spec show SPEC-01.02").unwrap();
        if let DPCommand::Show { spec_id } = cmd {
            assert_eq!(spec_id, SpecId::new(1, 2));
        } else {
            panic!("Expected Show command");
        }
    }

    #[test]
    fn test_parse_review_command() {
        let cmd = DPCommandHandler::parse_command("/dp:review --strict").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::Review {
                strict: true,
                file_issues: false
            }
        ));

        let cmd = DPCommandHandler::parse_command("/dp:review --file-issues").unwrap();
        assert!(matches!(
            cmd,
            DPCommand::Review {
                strict: false,
                file_issues: true
            }
        ));
    }

    #[test]
    fn test_command_result() {
        let result = DPCommandResult::success("Test output")
            .with_warning("Warning 1")
            .with_data(serde_json::json!({"key": "value"}));

        assert!(result.success);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.data.is_some());
    }

    #[test]
    fn test_extract_spec_arg() {
        assert_eq!(
            extract_spec_arg("--spec=SPEC-01.02", "--spec="),
            Some(SpecId::new(1, 2))
        );
        assert_eq!(
            extract_spec_arg("--lean --spec=SPEC-10.05 --review", "--spec="),
            Some(SpecId::new(10, 5))
        );
        assert_eq!(extract_spec_arg("--lean", "--spec="), None);
    }
}
