//! Adversarial validator trait and implementations.
//!
//! Defines the core validation interface and provides implementations
//! for different validation backends.

use async_trait::async_trait;
use tracing::{debug, info, instrument, warn};

use super::strategies::{CriticStrategy, EdgeCaseStrategy, SecurityStrategy, ValidationStrategy};
use super::types::{
    AdversarialConfig, Issue, ValidationContext, ValidationIteration,
    ValidationResult, ValidationStats, ValidationVerdict,
};
use crate::error::Result;
use crate::llm::{ChatMessage, ClientConfig, CompletionRequest, GoogleClient, LLMClient};

/// Trait for adversarial validation.
///
/// Implementations of this trait review LLM outputs and identify
/// potential issues using an adversarial model.
#[async_trait]
pub trait AdversarialValidator: Send + Sync {
    /// Validate a response in the given context.
    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult>;

    /// Validate with multiple iterations until convergence or max iterations.
    async fn validate_iterative(
        &self,
        context: &mut ValidationContext,
        max_iterations: usize,
    ) -> Result<ValidationResult>;

    /// Get the validator's configuration.
    fn config(&self) -> &AdversarialConfig;

    /// Check if validation should be triggered based on the config.
    fn should_validate(&self, trigger: &str) -> bool {
        let config = self.config();
        if !config.enabled {
            return false;
        }

        match trigger {
            "review" => matches!(
                config.trigger,
                super::types::AdversarialTrigger::OnReview
                    | super::types::AdversarialTrigger::Always
            ),
            "commit" => matches!(
                config.trigger,
                super::types::AdversarialTrigger::OnCommit
                    | super::types::AdversarialTrigger::Always
            ),
            "manual" => true,
            _ => matches!(config.trigger, super::types::AdversarialTrigger::Always),
        }
    }
}

/// Adversarial validator using Google's Gemini models.
///
/// This is the primary implementation that uses a different provider
/// (Google) than the primary model (Anthropic) for true adversarial review.
pub struct GeminiValidator {
    client: GoogleClient,
    config: AdversarialConfig,
    strategies: Vec<Box<dyn ValidationStrategy>>,
}

impl GeminiValidator {
    /// Create a new Gemini validator.
    pub fn new(api_key: &str, config: AdversarialConfig) -> Result<Self> {
        let client_config = ClientConfig::new(api_key)
            .with_default_model(&config.model)
            .with_timeout(120);

        let client = GoogleClient::new(client_config);

        // Initialize strategies based on config
        let strategies = Self::init_strategies(&config);

        Ok(Self {
            client,
            config,
            strategies,
        })
    }

    /// Initialize validation strategies from config.
    fn init_strategies(config: &AdversarialConfig) -> Vec<Box<dyn ValidationStrategy>> {
        let mut strategies: Vec<Box<dyn ValidationStrategy>> = Vec::new();

        for name in &config.strategies {
            match name.as_str() {
                "critic" => strategies.push(Box::new(CriticStrategy::new())),
                "edge_case" => strategies.push(Box::new(EdgeCaseStrategy::new())),
                "security" => strategies.push(Box::new(SecurityStrategy::new())),
                _ => {
                    warn!("Unknown validation strategy: {}", name);
                }
            }
        }

        if strategies.is_empty() {
            // Default to critic strategy
            strategies.push(Box::new(CriticStrategy::new()));
        }

        strategies
    }

    /// Build the validation prompt.
    fn build_prompt(&self, context: &ValidationContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are an adversarial code reviewer. Your job is to find issues, bugs, and potential problems in the following code change.\n\n");
        prompt.push_str("## Original Request\n");
        prompt.push_str(&context.request);
        prompt.push_str("\n\n## Response Being Reviewed\n");
        prompt.push_str(&context.response);
        prompt.push_str("\n\n");

        // Add code context
        if !context.code_context.is_empty() {
            prompt.push_str("## Code Context\n");
            for file in &context.code_context {
                prompt.push_str(&format!("### {}\n```", file.path));
                if let Some(ref lang) = file.language {
                    prompt.push_str(lang);
                }
                prompt.push_str("\n");
                prompt.push_str(&file.content);
                prompt.push_str("\n```\n\n");
            }
        }

        // Add tool outputs
        if !context.tool_outputs.is_empty() {
            prompt.push_str("## Tool Outputs\n");
            for output in &context.tool_outputs {
                prompt.push_str(&format!(
                    "### {} ({})\nInput: {}\nOutput: {}\n\n",
                    output.tool,
                    if output.success { "success" } else { "failed" },
                    output.input,
                    output.output
                ));
            }
        }

        // Add prior iterations
        if !context.prior_iterations.is_empty() {
            prompt.push_str("## Prior Review Iterations\n");
            for iter in &context.prior_iterations {
                prompt.push_str(&format!("### Iteration {}\n", iter.iteration));
                prompt.push_str("Issues found:\n");
                for issue in &iter.issues {
                    prompt.push_str(&format!(
                        "- [{:?}] {}: {}\n",
                        issue.severity, issue.title, issue.description
                    ));
                }
                if let Some(ref response) = iter.response {
                    prompt.push_str(&format!("Response: {}\n", response));
                }
                prompt.push('\n');
            }
        }

        // Add strategy-specific instructions
        prompt.push_str("## Review Focus Areas\n");
        for strategy in &self.strategies {
            prompt.push_str(&format!("- {}\n", strategy.description()));
        }

        prompt.push_str("\n## Output Format\n");
        prompt.push_str("For each issue found, output in this exact format:\n");
        prompt.push_str("```\nISSUE: [severity] [category] - Title\nDESCRIPTION: Detailed description\nLOCATION: file:line (or \"response\" if in the response text)\nSUGGESTION: How to fix it\nCONFIDENCE: 0.0-1.0\n```\n\n");
        prompt.push_str("Severities: critical, high, medium, low, info\n");
        prompt.push_str("Categories: logic_error, security, error_handling, testing, performance, api_misuse, traceability, consistency, edge_case, architecture, documentation, other\n\n");
        prompt.push_str("If no issues are found, respond with: NO_ISSUES_FOUND\n");

        prompt
    }

    /// Parse issues from the response.
    fn parse_issues(&self, response: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        if response.contains("NO_ISSUES_FOUND") {
            return issues;
        }

        // Parse ISSUE blocks
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("ISSUE:") {
                let issue_line = lines[i].trim_start_matches("ISSUE:").trim();

                // Parse severity and category from "[severity] [category] - Title"
                if let Some((severity, category, title)) = Self::parse_issue_header(issue_line) {
                    let mut description = String::new();
                    let mut location = None;
                    let mut suggestion = None;
                    let mut confidence = 0.8;

                    // Read subsequent lines
                    i += 1;
                    while i < lines.len() && !lines[i].starts_with("ISSUE:") {
                        let line = lines[i];
                        if line.starts_with("DESCRIPTION:") {
                            description = line.trim_start_matches("DESCRIPTION:").trim().to_string();
                        } else if line.starts_with("LOCATION:") {
                            let loc = line.trim_start_matches("LOCATION:").trim();
                            location = Self::parse_location(loc);
                        } else if line.starts_with("SUGGESTION:") {
                            suggestion = Some(line.trim_start_matches("SUGGESTION:").trim().to_string());
                        } else if line.starts_with("CONFIDENCE:") {
                            if let Ok(c) = line.trim_start_matches("CONFIDENCE:").trim().parse::<f64>() {
                                confidence = c.clamp(0.0, 1.0);
                            }
                        }
                        i += 1;
                    }

                    // Only include issues above minimum confidence
                    if confidence >= self.config.min_confidence {
                        let mut issue = Issue::new(severity, category, title, description)
                            .with_confidence(confidence);

                        if let Some(loc) = location {
                            issue = issue.with_location(loc);
                        }
                        if let Some(sug) = suggestion {
                            issue = issue.with_suggestion(sug);
                        }

                        issues.push(issue);
                    }

                    continue;
                }
            }
            i += 1;
        }

        issues
    }

    /// Parse issue header: "[severity] [category] - Title"
    fn parse_issue_header(header: &str) -> Option<(super::types::IssueSeverity, super::types::IssueCategory, String)> {
        let _parts: Vec<&str> = header.splitn(3, |c| c == '[' || c == ']' || c == '-').collect();

        // Try to find severity and category in brackets
        let header_lower = header.to_lowercase();

        let severity = if header_lower.contains("critical") {
            super::types::IssueSeverity::Critical
        } else if header_lower.contains("high") {
            super::types::IssueSeverity::High
        } else if header_lower.contains("medium") {
            super::types::IssueSeverity::Medium
        } else if header_lower.contains("low") {
            super::types::IssueSeverity::Low
        } else if header_lower.contains("info") {
            super::types::IssueSeverity::Info
        } else {
            super::types::IssueSeverity::Medium
        };

        let category = if header_lower.contains("security") {
            super::types::IssueCategory::Security
        } else if header_lower.contains("logic") {
            super::types::IssueCategory::LogicError
        } else if header_lower.contains("error") || header_lower.contains("handling") {
            super::types::IssueCategory::ErrorHandling
        } else if header_lower.contains("test") {
            super::types::IssueCategory::Testing
        } else if header_lower.contains("perf") {
            super::types::IssueCategory::Performance
        } else if header_lower.contains("api") {
            super::types::IssueCategory::ApiMisuse
        } else if header_lower.contains("trace") {
            super::types::IssueCategory::Traceability
        } else if header_lower.contains("consist") {
            super::types::IssueCategory::Consistency
        } else if header_lower.contains("edge") {
            super::types::IssueCategory::EdgeCase
        } else if header_lower.contains("arch") {
            super::types::IssueCategory::Architecture
        } else if header_lower.contains("doc") {
            super::types::IssueCategory::Documentation
        } else {
            super::types::IssueCategory::Other
        };

        // Extract title (everything after the last '-' or after brackets)
        let title = if let Some(idx) = header.rfind('-') {
            header[idx + 1..].trim().to_string()
        } else {
            header.to_string()
        };

        Some((severity, category, title))
    }

    /// Parse location string.
    fn parse_location(loc: &str) -> Option<super::types::IssueLocation> {
        if loc.eq_ignore_ascii_case("response") {
            return Some(super::types::IssueLocation::in_response(0, 0));
        }

        // Try to parse "file:line"
        if let Some(colon_idx) = loc.rfind(':') {
            let file = loc[..colon_idx].to_string();
            if let Ok(line) = loc[colon_idx + 1..].trim().parse::<u32>() {
                return Some(super::types::IssueLocation::in_file(file, line));
            }
        }

        None
    }
}

#[async_trait]
impl AdversarialValidator for GeminiValidator {
    #[instrument(skip(self, context), fields(validation_id = %context.id))]
    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        info!("Starting adversarial validation");

        let prompt = self.build_prompt(context);
        debug!("Built validation prompt ({} bytes)", prompt.len());

        let request = CompletionRequest {
            model: Some(self.config.model.clone()),
            messages: vec![ChatMessage::user(prompt)],
            max_tokens: Some(8192),
            temperature: Some(0.3),
            system: None,
            stop: None,
            enable_caching: false,
            metadata: None,
        };

        let start = std::time::Instant::now();
        let response = self.client.complete(request).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let issues = self.parse_issues(&response.content);
        info!("Found {} issues", issues.len());

        let stats = ValidationStats::from_issues(&issues);
        let mut stats = stats;
        stats.latency_ms = latency_ms;
        stats.tokens_used = (response.usage.input_tokens + response.usage.output_tokens) as u32;

        let verdict = if issues.is_empty() {
            ValidationVerdict::Approved
        } else if issues.iter().any(|i| i.blocking) {
            ValidationVerdict::Rejected
        } else {
            ValidationVerdict::ApprovedWithComments
        };

        let mut result = ValidationResult::new(context.id.clone());
        result.issues = issues;
        result.stats = stats;
        result.iterations = 1;
        result.cost_usd = response.usage.input_tokens as f64 * 0.000075 / 1000.0
            + response.usage.output_tokens as f64 * 0.0003 / 1000.0;
        result = result.complete(verdict);

        Ok(result)
    }

    async fn validate_iterative(
        &self,
        context: &mut ValidationContext,
        max_iterations: usize,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult::new(context.id.clone());
        let mut total_cost = 0.0;

        for iteration in 1..=max_iterations {
            info!("Validation iteration {}/{}", iteration, max_iterations);

            let iter_result = self.validate(context).await?;
            total_cost += iter_result.cost_usd;

            // Record this iteration
            let iter_record = ValidationIteration {
                iteration,
                issues: iter_result.issues.clone(),
                response: None,
                resolved: iter_result.issues.is_empty() || !iter_result.has_blocking_issues(),
                timestamp: chrono::Utc::now(),
            };

            // Add issues from this iteration
            for issue in &iter_result.issues {
                // Avoid duplicates
                if !result.issues.iter().any(|i| i.title == issue.title) {
                    result.issues.push(issue.clone());
                }
            }

            // Check for convergence
            if iter_record.resolved {
                info!("Validation converged after {} iterations", iteration);
                result.iterations = iteration;
                result.converged = true;
                result.cost_usd = total_cost;
                result.stats = ValidationStats::from_issues(&result.issues);

                let verdict = if result.issues.is_empty() {
                    ValidationVerdict::Approved
                } else {
                    ValidationVerdict::ApprovedWithComments
                };

                return Ok(result.complete(verdict));
            }

            // Add to context for next iteration
            context.prior_iterations.push(iter_record);
        }

        // Did not converge
        result.iterations = max_iterations;
        result.converged = false;
        result.cost_usd = total_cost;
        result.stats = ValidationStats::from_issues(&result.issues);

        Ok(result.complete(ValidationVerdict::Rejected))
    }

    fn config(&self) -> &AdversarialConfig {
        &self.config
    }
}

/// A mock validator for testing.
#[cfg(test)]
pub struct MockValidator {
    config: AdversarialConfig,
    issues_to_return: Vec<Issue>,
}

#[cfg(test)]
impl MockValidator {
    pub fn new() -> Self {
        Self {
            config: AdversarialConfig::default(),
            issues_to_return: Vec::new(),
        }
    }

    pub fn with_issues(mut self, issues: Vec<Issue>) -> Self {
        self.issues_to_return = issues;
        self
    }
}

#[cfg(test)]
#[async_trait]
impl AdversarialValidator for MockValidator {
    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let mut result = ValidationResult::new(context.id.clone());
        result.issues = self.issues_to_return.clone();
        result.iterations = 1;

        let verdict = if result.issues.is_empty() {
            ValidationVerdict::Approved
        } else if result.has_blocking_issues() {
            ValidationVerdict::Rejected
        } else {
            ValidationVerdict::ApprovedWithComments
        };

        Ok(result.complete(verdict))
    }

    async fn validate_iterative(
        &self,
        context: &mut ValidationContext,
        _max_iterations: usize,
    ) -> Result<ValidationResult> {
        self.validate(context).await
    }

    fn config(&self) -> &AdversarialConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_issue_header() {
        let (sev, cat, title) = GeminiValidator::parse_issue_header(
            "[critical] [security] - SQL Injection vulnerability"
        ).unwrap();

        assert_eq!(sev, super::super::types::IssueSeverity::Critical);
        assert_eq!(cat, super::super::types::IssueCategory::Security);
        assert_eq!(title, "SQL Injection vulnerability");
    }

    #[test]
    fn test_parse_location() {
        let loc = GeminiValidator::parse_location("src/main.rs:42").unwrap();
        assert_eq!(loc.file, Some("src/main.rs".to_string()));
        assert_eq!(loc.line, Some(42));

        let loc = GeminiValidator::parse_location("response").unwrap();
        assert!(loc.response_span.is_some());
    }

    #[tokio::test]
    async fn test_mock_validator() {
        let validator = MockValidator::new().with_issues(vec![
            Issue::new(
                super::super::types::IssueSeverity::High,
                super::super::types::IssueCategory::Security,
                "Test issue",
                "Test description",
            )
        ]);

        let ctx = ValidationContext::new("request", "response");
        let result = validator.validate(&ctx).await.unwrap();

        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.verdict, ValidationVerdict::Rejected);
    }
}
