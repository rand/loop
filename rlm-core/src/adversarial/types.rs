//! Core types for adversarial validation.
//!
//! Defines the structures used for adversarial review of LLM outputs,
//! including validation context, issues, and results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a validation session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidationId(pub Uuid);

impl ValidationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ValidationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ValidationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context for adversarial validation.
///
/// Contains all the information needed for the adversarial model to
/// evaluate the primary model's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationContext {
    /// Unique validation session ID
    pub id: ValidationId,
    /// The original user request/task
    pub request: String,
    /// The primary model's response being validated
    pub response: String,
    /// Relevant code files with their contents
    pub code_context: Vec<CodeFile>,
    /// Tool outputs referenced in the response
    pub tool_outputs: Vec<ToolOutput>,
    /// Previous validation iterations (for multi-round validation)
    pub prior_iterations: Vec<ValidationIteration>,
    /// Spec IDs that should be traced
    pub relevant_specs: Vec<String>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ValidationContext {
    /// Create a new validation context.
    pub fn new(request: impl Into<String>, response: impl Into<String>) -> Self {
        Self {
            id: ValidationId::new(),
            request: request.into(),
            response: response.into(),
            code_context: Vec::new(),
            tool_outputs: Vec::new(),
            prior_iterations: Vec::new(),
            relevant_specs: Vec::new(),
            metadata: None,
        }
    }

    /// Add code context.
    pub fn with_code_file(mut self, file: CodeFile) -> Self {
        self.code_context.push(file);
        self
    }

    /// Add tool output.
    pub fn with_tool_output(mut self, output: ToolOutput) -> Self {
        self.tool_outputs.push(output);
        self
    }

    /// Add prior iteration.
    pub fn with_prior_iteration(mut self, iteration: ValidationIteration) -> Self {
        self.prior_iterations.push(iteration);
        self
    }

    /// Add relevant spec ID.
    pub fn with_spec(mut self, spec_id: impl Into<String>) -> Self {
        self.relevant_specs.push(spec_id.into());
        self
    }

    /// Get the current iteration number.
    pub fn iteration_number(&self) -> usize {
        self.prior_iterations.len() + 1
    }
}

/// A code file in the validation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFile {
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Language (for syntax highlighting hints)
    pub language: Option<String>,
    /// Whether this is a new file being created
    pub is_new: bool,
    /// Original content (for diffs)
    pub original_content: Option<String>,
}

impl CodeFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            language: None,
            is_new: false,
            original_content: None,
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn as_new_file(mut self) -> Self {
        self.is_new = true;
        self
    }

    pub fn with_original(mut self, content: impl Into<String>) -> Self {
        self.original_content = Some(content.into());
        self
    }
}

/// A tool output referenced in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Tool name
    pub tool: String,
    /// Tool input/arguments
    pub input: String,
    /// Tool output
    pub output: String,
    /// Whether the tool succeeded
    pub success: bool,
}

impl ToolOutput {
    pub fn new(tool: impl Into<String>, input: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            input: input.into(),
            output: output.into(),
            success: true,
        }
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }
}

/// A single validation iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIteration {
    /// Iteration number (1-indexed)
    pub iteration: usize,
    /// Issues found in this iteration
    pub issues: Vec<Issue>,
    /// Primary model's response to the issues
    pub response: Option<String>,
    /// Whether issues were resolved
    pub resolved: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// An issue found during adversarial validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique identifier
    pub id: String,
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Category of the issue
    pub category: IssueCategory,
    /// Issue title (brief description)
    pub title: String,
    /// Detailed description of the issue
    pub description: String,
    /// Location in the response or code
    pub location: Option<IssueLocation>,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Confidence in this being a real issue (0.0-1.0)
    pub confidence: f64,
    /// Whether this is a blocking issue
    pub blocking: bool,
}

impl Issue {
    /// Create a new issue.
    pub fn new(
        severity: IssueSeverity,
        category: IssueCategory,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            severity,
            category,
            title: title.into(),
            description: description.into(),
            location: None,
            suggestion: None,
            confidence: 0.8,
            blocking: severity == IssueSeverity::Critical || severity == IssueSeverity::High,
        }
    }

    pub fn with_location(mut self, location: IssueLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn as_blocking(mut self) -> Self {
        self.blocking = true;
        self
    }

    pub fn as_non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

/// Severity level of an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Critical issue - must be fixed
    Critical,
    /// High severity - should be fixed
    High,
    /// Medium severity - consider fixing
    Medium,
    /// Low severity - minor issue
    Low,
    /// Informational - not necessarily a problem
    Info,
}

impl IssueSeverity {
    /// Check if this severity is considered blocking by default.
    pub fn is_blocking_default(&self) -> bool {
        matches!(self, Self::Critical | Self::High)
    }
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// Category of an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Logic error in code
    LogicError,
    /// Security vulnerability
    Security,
    /// Missing error handling
    ErrorHandling,
    /// Missing or incorrect tests
    Testing,
    /// Performance issue
    Performance,
    /// API misuse
    ApiMisuse,
    /// Missing spec traceability
    Traceability,
    /// Inconsistency with existing code
    Consistency,
    /// Missing edge case handling
    EdgeCase,
    /// Architectural concern
    Architecture,
    /// Documentation issue
    Documentation,
    /// Other issue
    Other,
}

impl std::fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LogicError => write!(f, "logic_error"),
            Self::Security => write!(f, "security"),
            Self::ErrorHandling => write!(f, "error_handling"),
            Self::Testing => write!(f, "testing"),
            Self::Performance => write!(f, "performance"),
            Self::ApiMisuse => write!(f, "api_misuse"),
            Self::Traceability => write!(f, "traceability"),
            Self::Consistency => write!(f, "consistency"),
            Self::EdgeCase => write!(f, "edge_case"),
            Self::Architecture => write!(f, "architecture"),
            Self::Documentation => write!(f, "documentation"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Location of an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLocation {
    /// File path (if applicable)
    pub file: Option<String>,
    /// Line number (1-indexed)
    pub line: Option<u32>,
    /// Column number (1-indexed)
    pub column: Option<u32>,
    /// Span in the response text
    pub response_span: Option<(usize, usize)>,
    /// Code snippet showing the issue
    pub snippet: Option<String>,
}

impl IssueLocation {
    pub fn in_file(file: impl Into<String>, line: u32) -> Self {
        Self {
            file: Some(file.into()),
            line: Some(line),
            column: None,
            response_span: None,
            snippet: None,
        }
    }

    pub fn in_response(start: usize, end: usize) -> Self {
        Self {
            file: None,
            line: None,
            column: None,
            response_span: Some((start, end)),
            snippet: None,
        }
    }

    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }
}

/// Result of adversarial validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation session ID
    pub id: ValidationId,
    /// Overall verdict
    pub verdict: ValidationVerdict,
    /// All issues found across all iterations
    pub issues: Vec<Issue>,
    /// Number of iterations performed
    pub iterations: usize,
    /// Whether validation converged (no blocking issues)
    pub converged: bool,
    /// Summary statistics
    pub stats: ValidationStats,
    /// When validation started
    pub started_at: DateTime<Utc>,
    /// When validation completed
    pub completed_at: DateTime<Utc>,
    /// Total cost in dollars
    pub cost_usd: f64,
}

impl ValidationResult {
    /// Create a new validation result.
    pub fn new(id: ValidationId) -> Self {
        let now = Utc::now();
        Self {
            id,
            verdict: ValidationVerdict::Pending,
            issues: Vec::new(),
            iterations: 0,
            converged: false,
            stats: ValidationStats::default(),
            started_at: now,
            completed_at: now,
            cost_usd: 0.0,
        }
    }

    /// Check if there are any blocking issues.
    pub fn has_blocking_issues(&self) -> bool {
        self.issues.iter().any(|i| i.blocking)
    }

    /// Get blocking issues only.
    pub fn blocking_issues(&self) -> Vec<&Issue> {
        self.issues.iter().filter(|i| i.blocking).collect()
    }

    /// Get issues by severity.
    pub fn issues_by_severity(&self, severity: IssueSeverity) -> Vec<&Issue> {
        self.issues.iter().filter(|i| i.severity == severity).collect()
    }

    /// Get issues by category.
    pub fn issues_by_category(&self, category: IssueCategory) -> Vec<&Issue> {
        self.issues.iter().filter(|i| i.category == category).collect()
    }

    /// Mark as complete with verdict.
    pub fn complete(mut self, verdict: ValidationVerdict) -> Self {
        self.verdict = verdict;
        self.completed_at = Utc::now();
        self.converged = !self.has_blocking_issues();
        self
    }

    /// Add an issue.
    pub fn with_issue(mut self, issue: Issue) -> Self {
        self.issues.push(issue);
        self
    }

    /// Set the cost.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost_usd = cost;
        self
    }
}

/// Overall validation verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationVerdict {
    /// Validation in progress
    Pending,
    /// No issues found
    Approved,
    /// Issues found but addressed
    ApprovedWithComments,
    /// Blocking issues remain
    Rejected,
    /// Validation failed (error)
    Error,
}

impl std::fmt::Display for ValidationVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::ApprovedWithComments => write!(f, "approved_with_comments"),
            Self::Rejected => write!(f, "rejected"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Statistics from validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total issues found
    pub total_issues: u32,
    /// Critical issues
    pub critical_issues: u32,
    /// High severity issues
    pub high_issues: u32,
    /// Medium severity issues
    pub medium_issues: u32,
    /// Low severity issues
    pub low_issues: u32,
    /// Informational issues
    pub info_issues: u32,
    /// Issues by category
    pub by_category: HashMap<String, u32>,
    /// Total tokens used (input + output)
    pub tokens_used: u32,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

impl ValidationStats {
    /// Update from a list of issues.
    pub fn from_issues(issues: &[Issue]) -> Self {
        let mut stats = Self::default();
        stats.total_issues = issues.len() as u32;

        for issue in issues {
            match issue.severity {
                IssueSeverity::Critical => stats.critical_issues += 1,
                IssueSeverity::High => stats.high_issues += 1,
                IssueSeverity::Medium => stats.medium_issues += 1,
                IssueSeverity::Low => stats.low_issues += 1,
                IssueSeverity::Info => stats.info_issues += 1,
            }

            *stats.by_category.entry(issue.category.to_string()).or_insert(0) += 1;
        }

        stats
    }
}

/// Configuration for adversarial validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialConfig {
    /// Whether adversarial validation is enabled
    pub enabled: bool,
    /// Model to use for adversarial review
    pub model: String,
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// When to trigger adversarial validation
    pub trigger: AdversarialTrigger,
    /// Whether to use fresh context for adversary
    pub fresh_context: bool,
    /// Validation strategies to use
    pub strategies: Vec<String>,
    /// Minimum confidence to report an issue
    pub min_confidence: f64,
    /// Whether to include code context
    pub include_code_context: bool,
    /// Maximum code context size in bytes
    pub max_code_context_bytes: usize,
}

impl Default for AdversarialConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "gemini-2.0-flash".to_string(),
            max_iterations: 3,
            trigger: AdversarialTrigger::OnReview,
            fresh_context: true,
            strategies: vec!["critic".to_string(), "edge_case".to_string()],
            min_confidence: 0.7,
            include_code_context: true,
            max_code_context_bytes: 50_000,
        }
    }
}

/// When to trigger adversarial validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdversarialTrigger {
    /// Trigger on explicit review command
    OnReview,
    /// Trigger before commit
    OnCommit,
    /// Trigger manually only
    Manual,
    /// Trigger on all responses
    Always,
}

impl std::fmt::Display for AdversarialTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OnReview => write!(f, "on_review"),
            Self::OnCommit => write!(f, "on_commit"),
            Self::Manual => write!(f, "manual"),
            Self::Always => write!(f, "always"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_context() {
        let ctx = ValidationContext::new("Fix the bug", "I fixed the bug by...")
            .with_code_file(CodeFile::new("src/main.rs", "fn main() {}"))
            .with_spec("SPEC-01.02");

        assert_eq!(ctx.iteration_number(), 1);
        assert_eq!(ctx.code_context.len(), 1);
        assert_eq!(ctx.relevant_specs.len(), 1);
    }

    #[test]
    fn test_issue_creation() {
        let issue = Issue::new(
            IssueSeverity::High,
            IssueCategory::Security,
            "SQL Injection",
            "User input is not sanitized",
        )
        .with_confidence(0.95)
        .with_location(IssueLocation::in_file("src/db.rs", 42));

        assert!(issue.blocking);
        assert_eq!(issue.confidence, 0.95);
        assert!(issue.location.is_some());
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::new(ValidationId::new())
            .with_issue(Issue::new(
                IssueSeverity::Medium,
                IssueCategory::Testing,
                "Missing test",
                "No test for edge case",
            ).as_non_blocking())
            .with_issue(Issue::new(
                IssueSeverity::Critical,
                IssueCategory::Security,
                "Auth bypass",
                "Authentication can be bypassed",
            ))
            .complete(ValidationVerdict::Rejected);

        assert!(result.has_blocking_issues());
        assert_eq!(result.blocking_issues().len(), 1);
        assert_eq!(result.verdict, ValidationVerdict::Rejected);
    }

    #[test]
    fn test_validation_stats() {
        let issues = vec![
            Issue::new(IssueSeverity::Critical, IssueCategory::Security, "A", "A"),
            Issue::new(IssueSeverity::High, IssueCategory::LogicError, "B", "B"),
            Issue::new(IssueSeverity::Medium, IssueCategory::Security, "C", "C"),
        ];

        let stats = ValidationStats::from_issues(&issues);
        assert_eq!(stats.total_issues, 3);
        assert_eq!(stats.critical_issues, 1);
        assert_eq!(stats.high_issues, 1);
        assert_eq!(stats.medium_issues, 1);
        assert_eq!(*stats.by_category.get("security").unwrap(), 2);
    }
}
