//! Validation strategies for adversarial review.
//!
//! Each strategy focuses on a specific type of issue detection.

use super::types::{Issue, IssueCategory, IssueSeverity, ValidationContext};

/// Trait for validation strategies.
///
/// Strategies provide focused issue detection for specific categories
/// of problems. They can be combined for comprehensive review.
pub trait ValidationStrategy: Send + Sync {
    /// Name of the strategy.
    fn name(&self) -> &str;

    /// Description of what this strategy looks for.
    fn description(&self) -> &str;

    /// Categories this strategy focuses on.
    fn categories(&self) -> Vec<IssueCategory>;

    /// Build strategy-specific prompt additions.
    fn prompt_additions(&self, context: &ValidationContext) -> String;

    /// Post-process issues to add strategy-specific context.
    fn post_process(&self, issues: &mut [Issue]) {
        // Default: no post-processing
        let _ = issues;
    }
}

/// General code critic strategy.
///
/// Looks for logic errors, missing error handling, and general code quality issues.
pub struct CriticStrategy {
    focus_areas: Vec<String>,
}

impl CriticStrategy {
    pub fn new() -> Self {
        Self {
            focus_areas: vec![
                "Logic errors and bugs".to_string(),
                "Missing error handling".to_string(),
                "Incorrect assumptions".to_string(),
                "Off-by-one errors".to_string(),
                "Race conditions".to_string(),
                "Resource leaks".to_string(),
            ],
        }
    }

    pub fn with_focus(mut self, area: impl Into<String>) -> Self {
        self.focus_areas.push(area.into());
        self
    }
}

impl Default for CriticStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for CriticStrategy {
    fn name(&self) -> &str {
        "critic"
    }

    fn description(&self) -> &str {
        "General code critic looking for logic errors, bugs, and code quality issues"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![
            IssueCategory::LogicError,
            IssueCategory::ErrorHandling,
            IssueCategory::Consistency,
        ]
    }

    fn prompt_additions(&self, _context: &ValidationContext) -> String {
        let mut prompt = String::from("### Critic Review Focus\n");
        prompt.push_str("Look carefully for:\n");
        for area in &self.focus_areas {
            prompt.push_str(&format!("- {}\n", area));
        }
        prompt.push_str(
            "\nQuestion every assumption. If the code assumes something, verify it's true.\n",
        );
        prompt
    }
}

/// Edge case detection strategy.
///
/// Specifically looks for missing edge case handling.
pub struct EdgeCaseStrategy {
    common_edge_cases: Vec<String>,
}

impl EdgeCaseStrategy {
    pub fn new() -> Self {
        Self {
            common_edge_cases: vec![
                "Empty collections (empty array, empty string, empty map)".to_string(),
                "Null/None/nil values".to_string(),
                "Boundary values (0, -1, MAX_INT, MIN_INT)".to_string(),
                "Unicode and special characters".to_string(),
                "Very large inputs".to_string(),
                "Concurrent access".to_string(),
                "Network failures and timeouts".to_string(),
                "Disk full / permission denied".to_string(),
                "Malformed input data".to_string(),
            ],
        }
    }

    pub fn with_edge_case(mut self, case: impl Into<String>) -> Self {
        self.common_edge_cases.push(case.into());
        self
    }
}

impl Default for EdgeCaseStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for EdgeCaseStrategy {
    fn name(&self) -> &str {
        "edge_case"
    }

    fn description(&self) -> &str {
        "Edge case hunter looking for missing boundary condition handling"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![IssueCategory::EdgeCase, IssueCategory::ErrorHandling]
    }

    fn prompt_additions(&self, _context: &ValidationContext) -> String {
        let mut prompt = String::from("### Edge Case Analysis\n");
        prompt.push_str("Identify any unhandled edge cases:\n");
        for case in &self.common_edge_cases {
            prompt.push_str(&format!("- {}\n", case));
        }
        prompt.push_str("\nFor each function/method, ask: what happens if the input is empty, null, very large, or malformed?\n");
        prompt
    }
}

/// Security-focused strategy.
///
/// Looks for security vulnerabilities and unsafe practices.
pub struct SecurityStrategy {
    vulnerability_classes: Vec<String>,
}

impl SecurityStrategy {
    pub fn new() -> Self {
        Self {
            vulnerability_classes: vec![
                "Injection (SQL, command, XSS)".to_string(),
                "Authentication/authorization bypass".to_string(),
                "Sensitive data exposure".to_string(),
                "Insecure deserialization".to_string(),
                "Path traversal".to_string(),
                "Cryptographic weaknesses".to_string(),
                "SSRF (Server-Side Request Forgery)".to_string(),
                "Insecure direct object references".to_string(),
                "Missing input validation".to_string(),
                "Hardcoded secrets".to_string(),
            ],
        }
    }

    pub fn with_vulnerability_class(mut self, class: impl Into<String>) -> Self {
        self.vulnerability_classes.push(class.into());
        self
    }
}

impl Default for SecurityStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for SecurityStrategy {
    fn name(&self) -> &str {
        "security"
    }

    fn description(&self) -> &str {
        "Security auditor looking for vulnerabilities and unsafe practices"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![IssueCategory::Security]
    }

    fn prompt_additions(&self, _context: &ValidationContext) -> String {
        let mut prompt = String::from("### Security Audit\n");
        prompt.push_str("Check for these vulnerability classes:\n");
        for class in &self.vulnerability_classes {
            prompt.push_str(&format!("- {}\n", class));
        }
        prompt
            .push_str("\nAny user-controlled input must be validated and sanitized before use.\n");
        prompt.push_str("Any sensitive operation must have proper authorization checks.\n");
        prompt
    }

    fn post_process(&self, issues: &mut [Issue]) {
        // Elevate security issues to at least High severity
        for issue in issues {
            if issue.category == IssueCategory::Security {
                if matches!(issue.severity, IssueSeverity::Low | IssueSeverity::Info) {
                    issue.severity = IssueSeverity::Medium;
                }
                // Security issues are always blocking
                issue.blocking = true;
            }
        }
    }
}

/// Performance-focused strategy.
pub struct PerformanceStrategy {
    concerns: Vec<String>,
}

impl PerformanceStrategy {
    pub fn new() -> Self {
        Self {
            concerns: vec![
                "N+1 query patterns".to_string(),
                "Unnecessary allocations".to_string(),
                "Blocking operations in async code".to_string(),
                "Missing caching opportunities".to_string(),
                "Inefficient algorithms (O(n²) when O(n) possible)".to_string(),
                "Large memory allocations".to_string(),
                "Unnecessary string copies".to_string(),
            ],
        }
    }
}

impl Default for PerformanceStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for PerformanceStrategy {
    fn name(&self) -> &str {
        "performance"
    }

    fn description(&self) -> &str {
        "Performance analyst looking for inefficiencies and scalability issues"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![IssueCategory::Performance]
    }

    fn prompt_additions(&self, _context: &ValidationContext) -> String {
        let mut prompt = String::from("### Performance Analysis\n");
        prompt.push_str("Look for performance issues:\n");
        for concern in &self.concerns {
            prompt.push_str(&format!("- {}\n", concern));
        }
        prompt.push_str("\nConsider what happens when data grows 10x or 100x.\n");
        prompt
    }
}

/// Testing coverage strategy.
pub struct TestingStrategy {
    test_types: Vec<String>,
}

impl TestingStrategy {
    pub fn new() -> Self {
        Self {
            test_types: vec![
                "Unit tests for new functions".to_string(),
                "Integration tests for API changes".to_string(),
                "Edge case tests".to_string(),
                "Error path tests".to_string(),
                "Regression tests for bug fixes".to_string(),
            ],
        }
    }
}

impl Default for TestingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for TestingStrategy {
    fn name(&self) -> &str {
        "testing"
    }

    fn description(&self) -> &str {
        "Test coverage analyst looking for missing or inadequate tests"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![IssueCategory::Testing]
    }

    fn prompt_additions(&self, context: &ValidationContext) -> String {
        let mut prompt = String::from("### Test Coverage Analysis\n");
        prompt.push_str("Check for missing tests:\n");
        for test_type in &self.test_types {
            prompt.push_str(&format!("- {}\n", test_type));
        }

        // Add spec traceability check if specs are present
        if !context.relevant_specs.is_empty() {
            prompt.push_str("\nVerify test traceability:\n");
            for spec in &context.relevant_specs {
                prompt.push_str(&format!("- Tests should have @trace {} marker\n", spec));
            }
        }

        prompt
    }
}

/// Spec traceability strategy.
pub struct TraceabilityStrategy;

impl TraceabilityStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TraceabilityStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationStrategy for TraceabilityStrategy {
    fn name(&self) -> &str {
        "traceability"
    }

    fn description(&self) -> &str {
        "Traceability checker ensuring code links to specifications"
    }

    fn categories(&self) -> Vec<IssueCategory> {
        vec![IssueCategory::Traceability, IssueCategory::Documentation]
    }

    fn prompt_additions(&self, context: &ValidationContext) -> String {
        let mut prompt = String::from("### Traceability Check\n");

        if context.relevant_specs.is_empty() {
            prompt.push_str(
                "Check if new code should have @trace markers linking to specifications.\n",
            );
        } else {
            prompt.push_str("Verify these specifications are traced in the code:\n");
            for spec in &context.relevant_specs {
                prompt.push_str(&format!(
                    "- {} should have corresponding @trace marker\n",
                    spec
                ));
            }
        }

        prompt.push_str("\nEvery significant function should trace to a specification.\n");
        prompt
    }
}

/// Factory for creating strategy combinations.
pub struct StrategyFactory;

impl StrategyFactory {
    /// Create strategies from names.
    pub fn from_names(names: &[String]) -> Vec<Box<dyn ValidationStrategy>> {
        let mut strategies: Vec<Box<dyn ValidationStrategy>> = Vec::new();

        for name in names {
            match name.as_str() {
                "critic" => strategies.push(Box::new(CriticStrategy::new())),
                "edge_case" => strategies.push(Box::new(EdgeCaseStrategy::new())),
                "security" => strategies.push(Box::new(SecurityStrategy::new())),
                "performance" => strategies.push(Box::new(PerformanceStrategy::new())),
                "testing" => strategies.push(Box::new(TestingStrategy::new())),
                "traceability" => strategies.push(Box::new(TraceabilityStrategy::new())),
                _ => {}
            }
        }

        strategies
    }

    /// Create a comprehensive strategy set.
    pub fn comprehensive() -> Vec<Box<dyn ValidationStrategy>> {
        vec![
            Box::new(CriticStrategy::new()),
            Box::new(EdgeCaseStrategy::new()),
            Box::new(SecurityStrategy::new()),
            Box::new(TestingStrategy::new()),
        ]
    }

    /// Create a quick strategy set for fast validation.
    pub fn quick() -> Vec<Box<dyn ValidationStrategy>> {
        vec![
            Box::new(CriticStrategy::new()),
            Box::new(SecurityStrategy::new()),
        ]
    }

    /// Create a security-focused strategy set.
    pub fn security_focused() -> Vec<Box<dyn ValidationStrategy>> {
        vec![
            Box::new(SecurityStrategy::new()),
            Box::new(EdgeCaseStrategy::new()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critic_strategy() {
        let strategy = CriticStrategy::new();
        assert_eq!(strategy.name(), "critic");
        assert!(strategy.categories().contains(&IssueCategory::LogicError));

        let ctx = ValidationContext::new("request", "response");
        let prompt = strategy.prompt_additions(&ctx);
        assert!(prompt.contains("Logic errors"));
    }

    #[test]
    fn test_security_strategy_post_process() {
        let strategy = SecurityStrategy::new();
        let mut issues =
            vec![
                Issue::new(IssueSeverity::Low, IssueCategory::Security, "Test", "Desc")
                    .as_non_blocking(),
            ];

        strategy.post_process(&mut issues);

        // Should be elevated and made blocking
        assert_eq!(issues[0].severity, IssueSeverity::Medium);
        assert!(issues[0].blocking);
    }

    #[test]
    fn test_strategy_factory() {
        let strategies =
            StrategyFactory::from_names(&["critic".to_string(), "security".to_string()]);

        assert_eq!(strategies.len(), 2);
        assert_eq!(strategies[0].name(), "critic");
        assert_eq!(strategies[1].name(), "security");
    }

    #[test]
    fn test_traceability_with_specs() {
        let strategy = TraceabilityStrategy::new();
        let ctx = ValidationContext::new("request", "response")
            .with_spec("SPEC-01.02")
            .with_spec("SPEC-03.04");

        let prompt = strategy.prompt_additions(&ctx);
        assert!(prompt.contains("SPEC-01.02"));
        assert!(prompt.contains("SPEC-03.04"));
    }
}
