//! Natural language parser for extracting requirements.
//!
//! This module parses natural language requirements and extracts:
//! - Data structures and types
//! - Behaviors and operations
//! - Constraints and invariants
//! - Ambiguities that need clarification

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

use super::types::{
    Ambiguity, AmbiguitySeverity, ExtractedRequirement, Question, QuestionCategory,
    RequirementType, SpecContext, SpecDomain,
};

// ============================================================================
// Regex patterns for NL parsing
// ============================================================================

/// Pattern for identifying data structure definitions.
/// Matches phrases like "a user has", "each order contains", "the system stores".
static DATA_STRUCTURE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(a|an|each|every|the)\s+(\w+)\s+(has|have|contains?|includes?|stores?|holds?)\b")
        .expect("Invalid regex")
});

/// Pattern for identifying behavioral requirements.
/// Matches phrases like "users can", "the system should", "must be able to".
static BEHAVIOR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(can|should|must|shall|will|may)\s+(be able to\s+)?(\w+)",
    )
    .expect("Invalid regex")
});

/// Pattern for identifying constraints.
/// Matches phrases like "must be", "cannot be", "always", "never".
static CONSTRAINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(must|cannot|can't|should not|shouldn't|always|never|at least|at most|exactly|no more than|no less than)\b")
        .expect("Invalid regex")
});

/// Pattern for identifying error cases.
/// Matches phrases like "if fails", "when error", "invalid input".
static ERROR_CASE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(fail|error|invalid|reject|deny|refuse|exception|timeout|not found|unauthorized|forbidden)\b")
        .expect("Invalid regex")
});

/// Pattern for identifying quantities/cardinality.
/// Matches phrases like "multiple items", "one or more", "at most 10".
static QUANTITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(one|single|multiple|many|several|at least \d+|at most \d+|exactly \d+|\d+ or more|\d+ to \d+)\b")
        .expect("Invalid regex")
});

/// Pattern for identifying entity names (capitalized words or quoted terms).
static ENTITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:`([^`]+)`|'([^']+)'|"([^"]+)"|\b([A-Z][a-z]+(?:[A-Z][a-z]+)*)\b)"#)
        .expect("Invalid regex")
});

/// Pattern for ambiguous terms that need clarification.
static AMBIGUOUS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(appropriate|suitable|reasonable|some|various|certain|relevant|proper|correct|valid|good|bad|large|small|fast|slow|soon|later|sometimes|often|usually|etc\.?|and so on|or something)\b")
        .expect("Invalid regex")
});

// ============================================================================
// Domain detection keywords
// ============================================================================

/// Keywords indicating distributed systems domain.
const DISTRIBUTED_KEYWORDS: &[&str] = &[
    "distributed",
    "consensus",
    "replicate",
    "partition",
    "node",
    "cluster",
    "eventual consistency",
    "CAP",
    "raft",
    "paxos",
    "leader",
    "follower",
    "heartbeat",
    "quorum",
];

/// Keywords indicating API domain.
const API_KEYWORDS: &[&str] = &[
    "API",
    "endpoint",
    "REST",
    "GraphQL",
    "request",
    "response",
    "HTTP",
    "GET",
    "POST",
    "PUT",
    "DELETE",
    "status code",
    "JSON",
    "payload",
];

/// Keywords indicating security domain.
const SECURITY_KEYWORDS: &[&str] = &[
    "security",
    "auth",
    "authenticate",
    "authorize",
    "permission",
    "role",
    "access control",
    "encrypt",
    "decrypt",
    "token",
    "JWT",
    "OAuth",
    "password",
    "credential",
    "secret",
];

/// Keywords indicating algorithm domain.
const ALGORITHM_KEYWORDS: &[&str] = &[
    "algorithm",
    "sort",
    "search",
    "traverse",
    "graph",
    "tree",
    "complexity",
    "O(n)",
    "recursive",
    "iterate",
    "optimize",
    "efficient",
];

/// Keywords indicating concurrency domain.
const CONCURRENCY_KEYWORDS: &[&str] = &[
    "concurrent",
    "parallel",
    "thread",
    "async",
    "await",
    "lock",
    "mutex",
    "semaphore",
    "atomic",
    "race condition",
    "deadlock",
    "synchronize",
];

// ============================================================================
// NL Parser
// ============================================================================

/// Parser for natural language requirements.
pub struct NLParser;

impl NLParser {
    /// Parse natural language input and populate the spec context.
    pub fn parse(ctx: &mut SpecContext) -> ParseResult {
        let input = &ctx.nl_input;
        let mut result = ParseResult::new();

        // Split into sentences for processing
        let sentences = Self::split_sentences(input);

        // Extract requirements from each sentence
        for (idx, sentence) in sentences.iter().enumerate() {
            Self::extract_requirements_from_sentence(sentence, idx, &mut result, ctx);
        }

        // Detect domains
        ctx.detected_domains = Self::detect_domains(input);

        // Find ambiguities
        let ambiguities = Self::find_ambiguities(input);
        for ambiguity in ambiguities {
            ctx.add_ambiguity(ambiguity);
        }

        // Add requirements to context
        for req in &result.requirements {
            ctx.add_requirement(req.clone());
        }

        result
    }

    /// Split text into sentences.
    fn split_sentences(text: &str) -> Vec<String> {
        // Simple sentence splitting - could be more sophisticated
        text.split(|c| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Extract requirements from a single sentence.
    fn extract_requirements_from_sentence(
        sentence: &str,
        sentence_idx: usize,
        result: &mut ParseResult,
        _ctx: &SpecContext,
    ) {
        let entities = Self::extract_entities(sentence);

        // Check for data structure requirements
        if DATA_STRUCTURE_PATTERN.is_match(sentence) {
            if let Some(cap) = DATA_STRUCTURE_PATTERN.captures(sentence) {
                let entity = cap.get(2).map(|m| m.as_str()).unwrap_or("Entity");
                result.requirements.push(ExtractedRequirement {
                    id: format!("REQ-DS-{}", sentence_idx),
                    text: sentence.to_string(),
                    req_type: RequirementType::DataStructure,
                    confidence: 0.8,
                    source_span: None,
                    entities: entities.clone(),
                    formal_name: Some(Self::to_pascal_case(entity)),
                });
            }
        }

        // Check for behavioral requirements
        if BEHAVIOR_PATTERN.is_match(sentence) {
            if let Some(cap) = BEHAVIOR_PATTERN.captures(sentence) {
                let verb = cap.get(3).map(|m| m.as_str()).unwrap_or("perform");
                result.requirements.push(ExtractedRequirement {
                    id: format!("REQ-BH-{}", sentence_idx),
                    text: sentence.to_string(),
                    req_type: RequirementType::Behavior,
                    confidence: 0.75,
                    source_span: None,
                    entities: entities.clone(),
                    formal_name: Some(Self::to_snake_case(verb)),
                });
            }
        }

        // Check for constraint requirements
        if CONSTRAINT_PATTERN.is_match(sentence) {
            let has_quantity = QUANTITY_PATTERN.is_match(sentence);
            result.requirements.push(ExtractedRequirement {
                id: format!("REQ-CN-{}", sentence_idx),
                text: sentence.to_string(),
                req_type: RequirementType::Constraint,
                confidence: if has_quantity { 0.85 } else { 0.7 },
                source_span: None,
                entities: entities.clone(),
                formal_name: None,
            });
        }

        // Check for error case requirements
        if ERROR_CASE_PATTERN.is_match(sentence) {
            result.requirements.push(ExtractedRequirement {
                id: format!("REQ-ER-{}", sentence_idx),
                text: sentence.to_string(),
                req_type: RequirementType::ErrorCase,
                confidence: 0.8,
                source_span: None,
                entities,
                formal_name: None,
            });
        }
    }

    /// Extract entity names from text.
    fn extract_entities(text: &str) -> Vec<String> {
        let mut entities = HashSet::new();

        for cap in ENTITY_PATTERN.captures_iter(text) {
            // Get the matched group (could be backtick, quote, or capitalized word)
            let entity = cap
                .get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .or_else(|| cap.get(4))
                .map(|m| m.as_str().to_string());

            if let Some(e) = entity {
                // Filter out common words
                if !Self::is_common_word(&e) {
                    entities.insert(e);
                }
            }
        }

        entities.into_iter().collect()
    }

    /// Check if a word is a common word that shouldn't be treated as an entity.
    fn is_common_word(word: &str) -> bool {
        const COMMON_WORDS: &[&str] = &[
            "The", "This", "That", "These", "Those", "Each", "Every", "All", "Any", "Some", "When",
            "Where", "What", "Which", "Who", "How", "If", "Then", "Else", "And", "Or", "But", "For",
            "With", "From", "Into", "After", "Before", "During", "While",
        ];
        COMMON_WORDS.contains(&word)
    }

    /// Detect domains from the input text.
    fn detect_domains(text: &str) -> Vec<SpecDomain> {
        let text_lower = text.to_lowercase();
        let mut domains = Vec::new();

        if DISTRIBUTED_KEYWORDS
            .iter()
            .any(|k| text_lower.contains(k))
        {
            domains.push(SpecDomain::DistributedSystems);
        }

        if API_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            domains.push(SpecDomain::APIs);
        }

        if SECURITY_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            domains.push(SpecDomain::Security);
        }

        if ALGORITHM_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            domains.push(SpecDomain::Algorithms);
        }

        if CONCURRENCY_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            domains.push(SpecDomain::Concurrency);
        }

        // Default to ApplicationFlow if no specific domain detected
        if domains.is_empty() {
            domains.push(SpecDomain::ApplicationFlow);
        }

        domains
    }

    /// Find ambiguities in the input text.
    fn find_ambiguities(text: &str) -> Vec<Ambiguity> {
        let mut ambiguities = Vec::new();

        for cap in AMBIGUOUS_PATTERN.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                let term = m.as_str().to_string();
                let severity = Self::ambiguity_severity(&term);

                // Get context around the ambiguous term
                let start = m.start().saturating_sub(20);
                let end = (m.end() + 20).min(text.len());
                let context = &text[start..end];

                ambiguities.push(Ambiguity {
                    description: format!("Ambiguous term '{}' needs clarification", term),
                    source_text: context.to_string(),
                    interpretations: Self::suggest_interpretations(&term),
                    severity,
                });
            }
        }

        ambiguities
    }

    /// Determine the severity of an ambiguity.
    fn ambiguity_severity(term: &str) -> AmbiguitySeverity {
        let term_lower = term.to_lowercase();
        match term_lower.as_str() {
            "appropriate" | "suitable" | "reasonable" | "proper" | "correct" => {
                AmbiguitySeverity::High
            }
            "some" | "various" | "certain" | "relevant" => AmbiguitySeverity::Medium,
            _ => AmbiguitySeverity::Low,
        }
    }

    /// Suggest interpretations for an ambiguous term.
    fn suggest_interpretations(term: &str) -> Vec<String> {
        let term_lower = term.to_lowercase();
        match term_lower.as_str() {
            "appropriate" | "suitable" => vec![
                "Meets specific criteria (define criteria)".to_string(),
                "Within acceptable range (define range)".to_string(),
            ],
            "some" | "various" => vec![
                "At least one".to_string(),
                "A specific subset (define which)".to_string(),
                "All that match criteria".to_string(),
            ],
            "reasonable" => vec![
                "Within defined limits".to_string(),
                "Based on specific formula".to_string(),
                "Configurable threshold".to_string(),
            ],
            "valid" | "correct" => vec![
                "Passes validation rules".to_string(),
                "Matches expected format".to_string(),
                "Within acceptable bounds".to_string(),
            ],
            "fast" | "slow" => vec![
                "Under N milliseconds".to_string(),
                "Compared to baseline".to_string(),
                "Meeting SLA requirements".to_string(),
            ],
            _ => vec!["Please provide specific criteria".to_string()],
        }
    }

    /// Generate clarifying questions based on context.
    pub fn generate_questions(ctx: &SpecContext) -> Vec<Question> {
        let mut questions = Vec::new();
        let mut question_id = 0;

        // Questions for high-severity ambiguities
        for ambiguity in &ctx.ambiguities {
            if ambiguity.severity == AmbiguitySeverity::High {
                question_id += 1;
                questions.push(Question {
                    id: format!("Q-AMB-{}", question_id),
                    text: format!(
                        "The phrase \"{}\" is ambiguous. Could you clarify what specific criteria or constraints you mean?",
                        ambiguity.source_text.trim()
                    ),
                    category: QuestionCategory::Scope,
                    rationale: ambiguity.description.clone(),
                    suggestions: ambiguity.interpretations.clone(),
                    required: true,
                });
            }
        }

        // Questions for data structure requirements lacking details
        for req in ctx.requirements.iter().filter(|r| r.req_type == RequirementType::DataStructure) {
            // Check if we have field details
            if !req.text.contains(':') && !req.text.contains("with") {
                question_id += 1;
                questions.push(Question {
                    id: format!("Q-DS-{}", question_id),
                    text: format!(
                        "For the {} data structure, what fields/properties does it have?",
                        req.formal_name.as_ref().unwrap_or(&"entity".to_string())
                    ),
                    category: QuestionCategory::DataTypes,
                    rationale: format!("Need to define the structure of {}", req.formal_name.as_ref().unwrap_or(&"entity".to_string())),
                    suggestions: vec![
                        "id: unique identifier".to_string(),
                        "created_at: timestamp".to_string(),
                        "status: enum of states".to_string(),
                    ],
                    required: true,
                });
            }
        }

        // Questions for constraints lacking specifics
        for req in ctx.requirements.iter().filter(|r| r.req_type == RequirementType::Constraint) {
            if !QUANTITY_PATTERN.is_match(&req.text) {
                question_id += 1;
                questions.push(Question {
                    id: format!("Q-CN-{}", question_id),
                    text: format!(
                        "The constraint \"{}\" - can you provide specific numeric bounds or criteria?",
                        Self::truncate(&req.text, 60)
                    ),
                    category: QuestionCategory::Invariants,
                    rationale: "Numeric constraints enable formal verification".to_string(),
                    suggestions: vec![
                        "Must be at least N".to_string(),
                        "Must be at most N".to_string(),
                        "Must be exactly N".to_string(),
                    ],
                    required: false,
                });
            }
        }

        // Questions about error handling if behaviors exist but no error cases
        let has_behaviors = ctx.requirements.iter().any(|r| r.req_type == RequirementType::Behavior);
        let has_errors = ctx.requirements.iter().any(|r| r.req_type == RequirementType::ErrorCase);

        if has_behaviors && !has_errors {
            question_id += 1;
            questions.push(Question {
                id: format!("Q-ERR-{}", question_id),
                text: "What should happen when an operation fails? (e.g., invalid input, resource not found, permission denied)".to_string(),
                category: QuestionCategory::EdgeCases,
                rationale: "Error handling is important for robust specifications".to_string(),
                suggestions: vec![
                    "Return error code".to_string(),
                    "Throw exception".to_string(),
                    "Return None/null".to_string(),
                    "Log and continue".to_string(),
                ],
                required: false,
            });
        }

        questions
    }

    /// Convert to PascalCase.
    fn to_pascal_case(s: &str) -> String {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    /// Convert to snake_case.
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    }

    /// Truncate string to max length with ellipsis.
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
}

/// Result of parsing natural language input.
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    /// Extracted requirements.
    pub requirements: Vec<ExtractedRequirement>,
    /// Parsing warnings.
    pub warnings: Vec<String>,
    /// Raw entities found.
    pub entities: Vec<String>,
}

impl ParseResult {
    /// Create a new empty parse result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if parsing found any requirements.
    pub fn has_requirements(&self) -> bool {
        !self.requirements.is_empty()
    }

    /// Get requirements by type.
    pub fn requirements_by_type(&self, req_type: RequirementType) -> Vec<&ExtractedRequirement> {
        self.requirements
            .iter()
            .filter(|r| r.req_type == req_type)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_structure() {
        let mut ctx = SpecContext::new("An Order has multiple items and a status");
        let result = NLParser::parse(&mut ctx);

        assert!(result.has_requirements());
        let ds_reqs = result.requirements_by_type(RequirementType::DataStructure);
        assert!(!ds_reqs.is_empty());
        assert!(ds_reqs[0].formal_name.as_ref().unwrap().contains("Order"));
    }

    #[test]
    fn test_parse_behavior() {
        let mut ctx = SpecContext::new("Users can create orders and view their history");
        let result = NLParser::parse(&mut ctx);

        let bh_reqs = result.requirements_by_type(RequirementType::Behavior);
        assert!(!bh_reqs.is_empty());
    }

    #[test]
    fn test_parse_constraint() {
        let mut ctx = SpecContext::new("Each order must have at least one item");
        let result = NLParser::parse(&mut ctx);

        let cn_reqs = result.requirements_by_type(RequirementType::Constraint);
        assert!(!cn_reqs.is_empty());
    }

    #[test]
    fn test_detect_domains_api() {
        let mut ctx = SpecContext::new("The REST API endpoint accepts JSON requests");
        NLParser::parse(&mut ctx);

        assert!(ctx.detected_domains.contains(&SpecDomain::APIs));
    }

    #[test]
    fn test_detect_domains_security() {
        let mut ctx = SpecContext::new("Users must authenticate with OAuth tokens");
        NLParser::parse(&mut ctx);

        assert!(ctx.detected_domains.contains(&SpecDomain::Security));
    }

    #[test]
    fn test_find_ambiguities() {
        let mut ctx = SpecContext::new("The system should return appropriate results in a reasonable time");
        NLParser::parse(&mut ctx);

        assert!(!ctx.ambiguities.is_empty());
        assert!(ctx.ambiguities.iter().any(|a| a.severity == AmbiguitySeverity::High));
    }

    #[test]
    fn test_extract_entities() {
        let entities = NLParser::extract_entities("Each User has an Order with multiple OrderItems");
        assert!(entities.contains(&"User".to_string()));
        assert!(entities.contains(&"Order".to_string()));
        assert!(entities.contains(&"OrderItems".to_string()));
    }

    #[test]
    fn test_generate_questions_for_ambiguities() {
        let mut ctx = SpecContext::new("The system should use appropriate validation");
        NLParser::parse(&mut ctx);

        let questions = NLParser::generate_questions(&ctx);
        assert!(!questions.is_empty());
        assert!(questions.iter().any(|q| q.required));
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(NLParser::to_pascal_case("order"), "Order");
        assert_eq!(NLParser::to_pascal_case("order_item"), "OrderItem");
        assert_eq!(NLParser::to_pascal_case("user-profile"), "UserProfile");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(NLParser::to_snake_case("create"), "create");
        assert_eq!(NLParser::to_snake_case("createOrder"), "create_order");
        assert_eq!(NLParser::to_snake_case("CreateOrder"), "create_order");
    }
}
