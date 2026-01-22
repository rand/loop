//! Python bindings for adversarial validation types.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

use crate::adversarial::{
    AdversarialConfig, AdversarialTrigger, CodeFile, Issue, IssueCategory, IssueLocation,
    IssueSeverity, ToolOutput, ValidationContext, ValidationResult, ValidationStats,
    ValidationVerdict,
};

/// Python enum for IssueSeverity.
#[pyclass(name = "IssueSeverity", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyIssueSeverity {
    Critical = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Info = 4,
}

impl From<IssueSeverity> for PyIssueSeverity {
    fn from(s: IssueSeverity) -> Self {
        match s {
            IssueSeverity::Critical => PyIssueSeverity::Critical,
            IssueSeverity::High => PyIssueSeverity::High,
            IssueSeverity::Medium => PyIssueSeverity::Medium,
            IssueSeverity::Low => PyIssueSeverity::Low,
            IssueSeverity::Info => PyIssueSeverity::Info,
        }
    }
}

impl From<PyIssueSeverity> for IssueSeverity {
    fn from(s: PyIssueSeverity) -> Self {
        match s {
            PyIssueSeverity::Critical => IssueSeverity::Critical,
            PyIssueSeverity::High => IssueSeverity::High,
            PyIssueSeverity::Medium => IssueSeverity::Medium,
            PyIssueSeverity::Low => IssueSeverity::Low,
            PyIssueSeverity::Info => IssueSeverity::Info,
        }
    }
}

#[pymethods]
impl PyIssueSeverity {
    fn __repr__(&self) -> &'static str {
        match self {
            PyIssueSeverity::Critical => "IssueSeverity.Critical",
            PyIssueSeverity::High => "IssueSeverity.High",
            PyIssueSeverity::Medium => "IssueSeverity.Medium",
            PyIssueSeverity::Low => "IssueSeverity.Low",
            PyIssueSeverity::Info => "IssueSeverity.Info",
        }
    }

    /// Check if this severity is blocking by default.
    fn is_blocking_default(&self) -> bool {
        IssueSeverity::from(*self).is_blocking_default()
    }
}

/// Python enum for IssueCategory.
#[pyclass(name = "IssueCategory", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyIssueCategory {
    LogicError = 0,
    Security = 1,
    ErrorHandling = 2,
    Testing = 3,
    Performance = 4,
    ApiMisuse = 5,
    Traceability = 6,
    Consistency = 7,
    EdgeCase = 8,
    Architecture = 9,
    Documentation = 10,
    Other = 11,
}

impl From<IssueCategory> for PyIssueCategory {
    fn from(c: IssueCategory) -> Self {
        match c {
            IssueCategory::LogicError => PyIssueCategory::LogicError,
            IssueCategory::Security => PyIssueCategory::Security,
            IssueCategory::ErrorHandling => PyIssueCategory::ErrorHandling,
            IssueCategory::Testing => PyIssueCategory::Testing,
            IssueCategory::Performance => PyIssueCategory::Performance,
            IssueCategory::ApiMisuse => PyIssueCategory::ApiMisuse,
            IssueCategory::Traceability => PyIssueCategory::Traceability,
            IssueCategory::Consistency => PyIssueCategory::Consistency,
            IssueCategory::EdgeCase => PyIssueCategory::EdgeCase,
            IssueCategory::Architecture => PyIssueCategory::Architecture,
            IssueCategory::Documentation => PyIssueCategory::Documentation,
            IssueCategory::Other => PyIssueCategory::Other,
        }
    }
}

impl From<PyIssueCategory> for IssueCategory {
    fn from(c: PyIssueCategory) -> Self {
        match c {
            PyIssueCategory::LogicError => IssueCategory::LogicError,
            PyIssueCategory::Security => IssueCategory::Security,
            PyIssueCategory::ErrorHandling => IssueCategory::ErrorHandling,
            PyIssueCategory::Testing => IssueCategory::Testing,
            PyIssueCategory::Performance => IssueCategory::Performance,
            PyIssueCategory::ApiMisuse => IssueCategory::ApiMisuse,
            PyIssueCategory::Traceability => IssueCategory::Traceability,
            PyIssueCategory::Consistency => IssueCategory::Consistency,
            PyIssueCategory::EdgeCase => IssueCategory::EdgeCase,
            PyIssueCategory::Architecture => IssueCategory::Architecture,
            PyIssueCategory::Documentation => IssueCategory::Documentation,
            PyIssueCategory::Other => IssueCategory::Other,
        }
    }
}

#[pymethods]
impl PyIssueCategory {
    fn __repr__(&self) -> &'static str {
        match self {
            PyIssueCategory::LogicError => "IssueCategory.LogicError",
            PyIssueCategory::Security => "IssueCategory.Security",
            PyIssueCategory::ErrorHandling => "IssueCategory.ErrorHandling",
            PyIssueCategory::Testing => "IssueCategory.Testing",
            PyIssueCategory::Performance => "IssueCategory.Performance",
            PyIssueCategory::ApiMisuse => "IssueCategory.ApiMisuse",
            PyIssueCategory::Traceability => "IssueCategory.Traceability",
            PyIssueCategory::Consistency => "IssueCategory.Consistency",
            PyIssueCategory::EdgeCase => "IssueCategory.EdgeCase",
            PyIssueCategory::Architecture => "IssueCategory.Architecture",
            PyIssueCategory::Documentation => "IssueCategory.Documentation",
            PyIssueCategory::Other => "IssueCategory.Other",
        }
    }
}

/// Python enum for ValidationVerdict.
#[pyclass(name = "ValidationVerdict", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyValidationVerdict {
    Pending = 0,
    Approved = 1,
    ApprovedWithComments = 2,
    Rejected = 3,
    Error = 4,
}

impl From<ValidationVerdict> for PyValidationVerdict {
    fn from(v: ValidationVerdict) -> Self {
        match v {
            ValidationVerdict::Pending => PyValidationVerdict::Pending,
            ValidationVerdict::Approved => PyValidationVerdict::Approved,
            ValidationVerdict::ApprovedWithComments => PyValidationVerdict::ApprovedWithComments,
            ValidationVerdict::Rejected => PyValidationVerdict::Rejected,
            ValidationVerdict::Error => PyValidationVerdict::Error,
        }
    }
}

impl From<PyValidationVerdict> for ValidationVerdict {
    fn from(v: PyValidationVerdict) -> Self {
        match v {
            PyValidationVerdict::Pending => ValidationVerdict::Pending,
            PyValidationVerdict::Approved => ValidationVerdict::Approved,
            PyValidationVerdict::ApprovedWithComments => ValidationVerdict::ApprovedWithComments,
            PyValidationVerdict::Rejected => ValidationVerdict::Rejected,
            PyValidationVerdict::Error => ValidationVerdict::Error,
        }
    }
}

#[pymethods]
impl PyValidationVerdict {
    fn __repr__(&self) -> &'static str {
        match self {
            PyValidationVerdict::Pending => "ValidationVerdict.Pending",
            PyValidationVerdict::Approved => "ValidationVerdict.Approved",
            PyValidationVerdict::ApprovedWithComments => "ValidationVerdict.ApprovedWithComments",
            PyValidationVerdict::Rejected => "ValidationVerdict.Rejected",
            PyValidationVerdict::Error => "ValidationVerdict.Error",
        }
    }
}

/// Python enum for AdversarialTrigger.
#[pyclass(name = "AdversarialTrigger", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyAdversarialTrigger {
    OnReview = 0,
    OnCommit = 1,
    Manual = 2,
    Always = 3,
}

impl From<AdversarialTrigger> for PyAdversarialTrigger {
    fn from(t: AdversarialTrigger) -> Self {
        match t {
            AdversarialTrigger::OnReview => PyAdversarialTrigger::OnReview,
            AdversarialTrigger::OnCommit => PyAdversarialTrigger::OnCommit,
            AdversarialTrigger::Manual => PyAdversarialTrigger::Manual,
            AdversarialTrigger::Always => PyAdversarialTrigger::Always,
        }
    }
}

impl From<PyAdversarialTrigger> for AdversarialTrigger {
    fn from(t: PyAdversarialTrigger) -> Self {
        match t {
            PyAdversarialTrigger::OnReview => AdversarialTrigger::OnReview,
            PyAdversarialTrigger::OnCommit => AdversarialTrigger::OnCommit,
            PyAdversarialTrigger::Manual => AdversarialTrigger::Manual,
            PyAdversarialTrigger::Always => AdversarialTrigger::Always,
        }
    }
}

#[pymethods]
impl PyAdversarialTrigger {
    fn __repr__(&self) -> &'static str {
        match self {
            PyAdversarialTrigger::OnReview => "AdversarialTrigger.OnReview",
            PyAdversarialTrigger::OnCommit => "AdversarialTrigger.OnCommit",
            PyAdversarialTrigger::Manual => "AdversarialTrigger.Manual",
            PyAdversarialTrigger::Always => "AdversarialTrigger.Always",
        }
    }
}

/// Python wrapper for CodeFile.
#[pyclass(name = "CodeFile")]
#[derive(Clone)]
pub struct PyCodeFile {
    pub(crate) inner: CodeFile,
}

#[pymethods]
impl PyCodeFile {
    #[new]
    #[pyo3(signature = (path, content, language=None, is_new=false))]
    fn new(path: String, content: String, language: Option<String>, is_new: bool) -> Self {
        let mut file = CodeFile::new(path, content);
        if let Some(lang) = language {
            file = file.with_language(lang);
        }
        if is_new {
            file = file.as_new_file();
        }
        Self { inner: file }
    }

    /// Set the original content for diff.
    fn with_original(&mut self, content: String) -> Self {
        self.inner = self.inner.clone().with_original(content);
        self.clone()
    }

    #[getter]
    fn path(&self) -> String {
        self.inner.path.clone()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn language(&self) -> Option<String> {
        self.inner.language.clone()
    }

    #[getter]
    fn is_new(&self) -> bool {
        self.inner.is_new
    }

    #[getter]
    fn original_content(&self) -> Option<String> {
        self.inner.original_content.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "CodeFile(path={:?}, len={}, is_new={})",
            self.inner.path,
            self.inner.content.len(),
            self.inner.is_new
        )
    }
}

/// Python wrapper for ToolOutput.
#[pyclass(name = "ToolOutput")]
#[derive(Clone)]
pub struct PyToolOutput {
    pub(crate) inner: ToolOutput,
}

#[pymethods]
impl PyToolOutput {
    #[new]
    #[pyo3(signature = (tool, input, output, success=true))]
    fn new(tool: String, input: String, output: String, success: bool) -> Self {
        Self {
            inner: ToolOutput::new(tool, input, output).with_success(success),
        }
    }

    #[getter]
    fn tool(&self) -> String {
        self.inner.tool.clone()
    }

    #[getter]
    fn input(&self) -> String {
        self.inner.input.clone()
    }

    #[getter]
    fn output(&self) -> String {
        self.inner.output.clone()
    }

    #[getter]
    fn success(&self) -> bool {
        self.inner.success
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolOutput(tool={:?}, success={})",
            self.inner.tool, self.inner.success
        )
    }
}

/// Python wrapper for IssueLocation.
#[pyclass(name = "IssueLocation")]
#[derive(Clone)]
pub struct PyIssueLocation {
    pub(crate) inner: IssueLocation,
}

#[pymethods]
impl PyIssueLocation {
    /// Create a location in a file.
    #[staticmethod]
    fn in_file(file: String, line: u32) -> Self {
        Self {
            inner: IssueLocation::in_file(file, line),
        }
    }

    /// Create a location in the response.
    #[staticmethod]
    fn in_response(start: usize, end: usize) -> Self {
        Self {
            inner: IssueLocation::in_response(start, end),
        }
    }

    /// Add column info.
    fn with_column(&mut self, column: u32) -> Self {
        self.inner = self.inner.clone().with_column(column);
        self.clone()
    }

    /// Add code snippet.
    fn with_snippet(&mut self, snippet: String) -> Self {
        self.inner = self.inner.clone().with_snippet(snippet);
        self.clone()
    }

    #[getter]
    fn file(&self) -> Option<String> {
        self.inner.file.clone()
    }

    #[getter]
    fn line(&self) -> Option<u32> {
        self.inner.line
    }

    #[getter]
    fn column(&self) -> Option<u32> {
        self.inner.column
    }

    #[getter]
    fn response_span(&self) -> Option<(usize, usize)> {
        self.inner.response_span
    }

    #[getter]
    fn snippet(&self) -> Option<String> {
        self.inner.snippet.clone()
    }

    fn __repr__(&self) -> String {
        if let Some(ref file) = self.inner.file {
            format!(
                "IssueLocation(file={:?}, line={:?})",
                file, self.inner.line
            )
        } else {
            format!("IssueLocation(response_span={:?})", self.inner.response_span)
        }
    }
}

/// Python wrapper for Issue.
#[pyclass(name = "Issue")]
#[derive(Clone)]
pub struct PyIssue {
    pub(crate) inner: Issue,
}

#[pymethods]
impl PyIssue {
    #[new]
    fn new(
        severity: PyIssueSeverity,
        category: PyIssueCategory,
        title: String,
        description: String,
    ) -> Self {
        Self {
            inner: Issue::new(severity.into(), category.into(), title, description),
        }
    }

    /// Add location.
    fn with_location(&mut self, location: &PyIssueLocation) -> Self {
        self.inner = self.inner.clone().with_location(location.inner.clone());
        self.clone()
    }

    /// Add suggestion.
    fn with_suggestion(&mut self, suggestion: String) -> Self {
        self.inner = self.inner.clone().with_suggestion(suggestion);
        self.clone()
    }

    /// Set confidence.
    fn with_confidence(&mut self, confidence: f64) -> Self {
        self.inner = self.inner.clone().with_confidence(confidence);
        self.clone()
    }

    /// Mark as blocking.
    fn as_blocking(&mut self) -> Self {
        self.inner = self.inner.clone().as_blocking();
        self.clone()
    }

    /// Mark as non-blocking.
    fn as_non_blocking(&mut self) -> Self {
        self.inner = self.inner.clone().as_non_blocking();
        self.clone()
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn severity(&self) -> PyIssueSeverity {
        self.inner.severity.into()
    }

    #[getter]
    fn category(&self) -> PyIssueCategory {
        self.inner.category.into()
    }

    #[getter]
    fn title(&self) -> String {
        self.inner.title.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn location(&self) -> Option<PyIssueLocation> {
        self.inner.location.clone().map(|l| PyIssueLocation { inner: l })
    }

    #[getter]
    fn suggestion(&self) -> Option<String> {
        self.inner.suggestion.clone()
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    #[getter]
    fn blocking(&self) -> bool {
        self.inner.blocking
    }

    fn __repr__(&self) -> String {
        format!(
            "Issue(severity={:?}, category={:?}, title={:?}, blocking={})",
            self.inner.severity, self.inner.category, self.inner.title, self.inner.blocking
        )
    }
}

/// Python wrapper for ValidationContext.
#[pyclass(name = "ValidationContext")]
#[derive(Clone)]
pub struct PyValidationContext {
    pub(crate) inner: ValidationContext,
}

#[pymethods]
impl PyValidationContext {
    #[new]
    fn new(request: String, response: String) -> Self {
        Self {
            inner: ValidationContext::new(request, response),
        }
    }

    /// Add a code file.
    fn with_code_file(&mut self, file: &PyCodeFile) -> Self {
        self.inner = self.inner.clone().with_code_file(file.inner.clone());
        self.clone()
    }

    /// Add a tool output.
    fn with_tool_output(&mut self, output: &PyToolOutput) -> Self {
        self.inner = self.inner.clone().with_tool_output(output.inner.clone());
        self.clone()
    }

    /// Add a spec reference.
    fn with_spec(&mut self, spec_id: String) -> Self {
        self.inner = self.inner.clone().with_spec(spec_id);
        self.clone()
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn request(&self) -> String {
        self.inner.request.clone()
    }

    #[getter]
    fn response(&self) -> String {
        self.inner.response.clone()
    }

    #[getter]
    fn code_context(&self) -> Vec<PyCodeFile> {
        self.inner
            .code_context
            .iter()
            .map(|f| PyCodeFile { inner: f.clone() })
            .collect()
    }

    #[getter]
    fn tool_outputs(&self) -> Vec<PyToolOutput> {
        self.inner
            .tool_outputs
            .iter()
            .map(|o| PyToolOutput { inner: o.clone() })
            .collect()
    }

    #[getter]
    fn relevant_specs(&self) -> Vec<String> {
        self.inner.relevant_specs.clone()
    }

    #[getter]
    fn iteration_number(&self) -> usize {
        self.inner.iteration_number()
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationContext(id={}, files={}, tools={})",
            self.inner.id,
            self.inner.code_context.len(),
            self.inner.tool_outputs.len()
        )
    }
}

/// Python wrapper for ValidationStats.
#[pyclass(name = "ValidationStats")]
#[derive(Clone)]
pub struct PyValidationStats {
    pub(crate) inner: ValidationStats,
}

#[pymethods]
impl PyValidationStats {
    #[getter]
    fn total_issues(&self) -> u32 {
        self.inner.total_issues
    }

    #[getter]
    fn critical_issues(&self) -> u32 {
        self.inner.critical_issues
    }

    #[getter]
    fn high_issues(&self) -> u32 {
        self.inner.high_issues
    }

    #[getter]
    fn medium_issues(&self) -> u32 {
        self.inner.medium_issues
    }

    #[getter]
    fn low_issues(&self) -> u32 {
        self.inner.low_issues
    }

    #[getter]
    fn info_issues(&self) -> u32 {
        self.inner.info_issues
    }

    #[getter]
    fn by_category(&self) -> HashMap<String, u32> {
        self.inner.by_category.clone()
    }

    #[getter]
    fn tokens_used(&self) -> u32 {
        self.inner.tokens_used
    }

    #[getter]
    fn latency_ms(&self) -> u64 {
        self.inner.latency_ms
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationStats(total={}, critical={}, high={})",
            self.inner.total_issues, self.inner.critical_issues, self.inner.high_issues
        )
    }
}

/// Python wrapper for ValidationResult.
#[pyclass(name = "ValidationResult")]
#[derive(Clone)]
pub struct PyValidationResult {
    pub(crate) inner: ValidationResult,
}

#[pymethods]
impl PyValidationResult {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn verdict(&self) -> PyValidationVerdict {
        self.inner.verdict.into()
    }

    #[getter]
    fn issues(&self) -> Vec<PyIssue> {
        self.inner
            .issues
            .iter()
            .map(|i| PyIssue { inner: i.clone() })
            .collect()
    }

    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }

    #[getter]
    fn stats(&self) -> PyValidationStats {
        PyValidationStats {
            inner: self.inner.stats.clone(),
        }
    }

    #[getter]
    fn started_at(&self) -> String {
        self.inner.started_at.to_rfc3339()
    }

    #[getter]
    fn completed_at(&self) -> String {
        self.inner.completed_at.to_rfc3339()
    }

    #[getter]
    fn cost_usd(&self) -> f64 {
        self.inner.cost_usd
    }

    /// Check if there are any blocking issues.
    fn has_blocking_issues(&self) -> bool {
        self.inner.has_blocking_issues()
    }

    /// Get only blocking issues.
    fn blocking_issues(&self) -> Vec<PyIssue> {
        self.inner
            .blocking_issues()
            .iter()
            .map(|i| PyIssue { inner: (*i).clone() })
            .collect()
    }

    /// Export to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationResult(verdict={:?}, issues={}, cost=${:.4})",
            self.inner.verdict,
            self.inner.issues.len(),
            self.inner.cost_usd
        )
    }
}

/// Python wrapper for AdversarialConfig.
#[pyclass(name = "AdversarialConfig")]
#[derive(Clone)]
pub struct PyAdversarialConfig {
    pub(crate) inner: AdversarialConfig,
}

#[pymethods]
impl PyAdversarialConfig {
    #[new]
    #[pyo3(signature = (
        enabled=false,
        model="gemini-2.0-flash".to_string(),
        max_iterations=3,
        trigger=PyAdversarialTrigger::OnReview,
        fresh_context=true
    ))]
    fn new(
        enabled: bool,
        model: String,
        max_iterations: usize,
        trigger: PyAdversarialTrigger,
        fresh_context: bool,
    ) -> Self {
        Self {
            inner: AdversarialConfig {
                enabled,
                model,
                max_iterations,
                trigger: trigger.into(),
                fresh_context,
                ..AdversarialConfig::default()
            },
        }
    }

    /// Create default config.
    #[staticmethod]
    fn default() -> Self {
        Self {
            inner: AdversarialConfig::default(),
        }
    }

    /// Set strategies.
    fn with_strategies(&mut self, strategies: Vec<String>) -> Self {
        self.inner.strategies = strategies;
        self.clone()
    }

    /// Set minimum confidence.
    fn with_min_confidence(&mut self, min_confidence: f64) -> Self {
        self.inner.min_confidence = min_confidence;
        self.clone()
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[getter]
    fn model(&self) -> String {
        self.inner.model.clone()
    }

    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[getter]
    fn trigger(&self) -> PyAdversarialTrigger {
        self.inner.trigger.into()
    }

    #[getter]
    fn fresh_context(&self) -> bool {
        self.inner.fresh_context
    }

    #[getter]
    fn strategies(&self) -> Vec<String> {
        self.inner.strategies.clone()
    }

    #[getter]
    fn min_confidence(&self) -> f64 {
        self.inner.min_confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "AdversarialConfig(enabled={}, model={:?}, trigger={:?})",
            self.inner.enabled, self.inner.model, self.inner.trigger
        )
    }
}
