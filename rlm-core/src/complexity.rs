//! Task complexity analysis and RLM activation decisions.
//!
//! The complexity module provides pattern-based classification of tasks to determine
//! whether RLM orchestration should be activated. It analyzes:
//! - Query patterns (keywords, structure)
//! - User intent signals
//! - Context characteristics (file count, token volume)
//! - Historical signals (previous turn state)

use crate::context::SessionContext;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Signals extracted from task analysis that indicate complexity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskComplexitySignals {
    // Prompt analysis
    /// Query references multiple files or paths
    pub references_multiple_files: bool,
    /// Requires reasoning across different parts of context
    pub requires_cross_context_reasoning: bool,
    /// Involves temporal reasoning (before/after, history)
    pub involves_temporal_reasoning: bool,
    /// Asks about patterns, relationships, or structure
    pub asks_about_patterns: bool,
    /// Task is debugging-related
    pub debugging_task: bool,
    /// Requires exhaustive search (all instances, every occurrence)
    pub requires_exhaustive_search: bool,
    /// Security review or audit task
    pub security_review_task: bool,
    /// Architecture analysis or design task
    pub architecture_analysis: bool,

    // User intent signals
    /// User explicitly wants thorough analysis
    pub user_wants_thorough: bool,
    /// User wants quick/simple response
    pub user_wants_fast: bool,

    // Context analysis
    /// Context spans multiple domains/modules
    pub context_has_multiple_domains: bool,
    /// Recent tool outputs are large (>10K tokens)
    pub recent_tool_outputs_large: bool,
    /// Cached files span multiple modules/directories
    pub files_span_multiple_modules: bool,

    // Historical signals
    /// Previous turn showed confusion or needed clarification
    pub previous_turn_was_confused: bool,
    /// Task is a continuation of previous work
    pub task_is_continuation: bool,
}

impl TaskComplexitySignals {
    /// Calculate a complexity score from the signals.
    /// Higher score = more complex task.
    pub fn score(&self) -> i32 {
        let mut score = 0;

        // Strong positive signals (+3 each)
        if self.architecture_analysis {
            score += 3;
        }
        if self.requires_exhaustive_search {
            score += 3;
        }
        if self.security_review_task {
            score += 3;
        }
        if self.user_wants_thorough {
            score += 3;
        }

        // Medium positive signals (+2 each)
        if self.references_multiple_files {
            score += 2;
        }
        if self.requires_cross_context_reasoning {
            score += 2;
        }
        if self.asks_about_patterns {
            score += 2;
        }
        if self.debugging_task {
            score += 2;
        }
        if self.context_has_multiple_domains {
            score += 2;
        }
        if self.files_span_multiple_modules {
            score += 2;
        }

        // Weak positive signals (+1 each)
        if self.involves_temporal_reasoning {
            score += 1;
        }
        if self.recent_tool_outputs_large {
            score += 1;
        }
        if self.previous_turn_was_confused {
            score += 1;
        }
        if self.task_is_continuation {
            score += 1;
        }

        // Negative signals
        if self.user_wants_fast {
            score -= 3;
        }

        score
    }

    /// Check if any strong complexity signal is present.
    pub fn has_strong_signal(&self) -> bool {
        self.architecture_analysis
            || self.requires_exhaustive_search
            || self.security_review_task
            || self.user_wants_thorough
    }

    /// Get human-readable list of active signals.
    /// Uses snake_case format to match Python test expectations.
    pub fn active_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.references_multiple_files {
            signals.push("multi_file");
        }
        if self.requires_cross_context_reasoning {
            signals.push("cross_context");
        }
        if self.involves_temporal_reasoning {
            signals.push("temporal");
        }
        if self.asks_about_patterns {
            signals.push("pattern_search");
        }
        if self.debugging_task {
            signals.push("debugging");
        }
        if self.requires_exhaustive_search {
            signals.push("exhaustive_search");
        }
        if self.security_review_task {
            signals.push("security_review");
        }
        if self.architecture_analysis {
            signals.push("architecture_analysis");
        }
        if self.user_wants_thorough {
            signals.push("user_thorough");
        }
        if self.user_wants_fast {
            signals.push("user_fast");
        }
        if self.context_has_multiple_domains {
            signals.push("multi_domain");
        }
        if self.recent_tool_outputs_large {
            signals.push("large_outputs");
        }
        if self.files_span_multiple_modules {
            signals.push("multi_module");
        }
        if self.previous_turn_was_confused {
            signals.push("prior_confusion");
        }
        if self.task_is_continuation {
            signals.push("continuation");
        }

        signals
    }
}

/// Decision about whether to activate RLM for a task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivationDecision {
    /// Whether RLM should be activated
    pub should_activate: bool,
    /// Human-readable reason for the decision
    pub reason: String,
    /// Complexity score that led to this decision
    pub score: i32,
    /// The signals that were analyzed
    pub signals: TaskComplexitySignals,
}

impl ActivationDecision {
    /// Create a decision to activate RLM.
    pub fn activate(reason: impl Into<String>, score: i32, signals: TaskComplexitySignals) -> Self {
        Self {
            should_activate: true,
            reason: reason.into(),
            score,
            signals,
        }
    }

    /// Create a decision to skip RLM.
    pub fn skip(reason: impl Into<String>, score: i32, signals: TaskComplexitySignals) -> Self {
        Self {
            should_activate: false,
            reason: reason.into(),
            score,
            signals,
        }
    }
}

/// Pattern-based complexity classifier.
///
/// Analyzes queries and context to determine task complexity using
/// regex patterns and heuristics.
#[derive(Debug, Clone)]
pub struct PatternClassifier {
    /// Minimum score threshold for activation
    pub activation_threshold: i32,
    /// Whether to always activate (for testing)
    pub force_activation: bool,
}

impl Default for PatternClassifier {
    fn default() -> Self {
        Self {
            // Threshold of 2 matches Python implementation behavior
            activation_threshold: 2,
            force_activation: false,
        }
    }
}

// Lazy-initialized regex patterns
static MULTI_FILE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(files?|modules?|components?|across|between|multiple|all\s+the)\s+(in|from|under|within)?")
        .expect("invalid regex")
});

static CROSS_CONTEXT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches "why X when Y", "how does X", relationship queries, etc.
    Regex::new(r"(?i)(why\b.*\b(when|if|given|since)|how\s+(does|do|is|are)|relationship|connect|interact|depend|flow|between|across|what\b.*\b(cause|led\s+to|result))")
        .expect("invalid regex")
});

static TEMPORAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(before|after|when|then|history|previous|changed|evolved|used\s+to)")
        .expect("invalid regex")
});

static PATTERN_ANALYSIS: LazyLock<Regex> = LazyLock::new(|| {
    // Matches "find places where", "search for X where", pattern queries, etc.
    Regex::new(r"(?i)((find|search|locate|grep)\b.*\b(where|that|which)|how\s+many|list\s+(all|every)|pattern|structure|architecture|design|organize|layout|convention|idiom)")
        .expect("invalid regex")
});

static DEBUGGING_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(debug|error|bug|issue|problem|fix|broken|failing|crash|exception|traceback|not\s+work)")
        .expect("invalid regex")
});

static EXHAUSTIVE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(all|every|each|exhaustive|comprehensive|complete|full|entire|everywhere)")
        .expect("invalid regex")
});

static SECURITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(security|auth|permission|access|credential|secret|vulnerab|injection|xss|csrf|owasp)")
        .expect("invalid regex")
});

static ARCHITECTURE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(architect|design|refactor|restructure|reorganize|system|overview|high.level)")
        .expect("invalid regex")
});

static THOROUGH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(thorough|careful|detailed|deep|exhaustive|comprehensive|make\s+sure|be\s+careful)",
    )
    .expect("invalid regex")
});

static FAST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(quick|fast|simple|just|only|brief|short|don'?t\s+overthink)")
        .expect("invalid regex")
});

static CONTINUATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(continue|also|now|next|then|keep|more|another|additionally)")
        .expect("invalid regex")
});

impl PatternClassifier {
    /// Create a new classifier with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a classifier with a custom activation threshold.
    pub fn with_threshold(threshold: i32) -> Self {
        Self {
            activation_threshold: threshold,
            force_activation: false,
        }
    }

    /// Analyze a query and context to extract complexity signals.
    pub fn analyze(&self, query: &str, context: &SessionContext) -> TaskComplexitySignals {
        let mut signals = TaskComplexitySignals::default();

        // Query pattern analysis
        signals.references_multiple_files = MULTI_FILE_PATTERN.is_match(query)
            || query.matches('/').count() > 1
            || query.matches(".rs").count() > 1
            || query.matches(".py").count() > 1
            || query.matches(".ts").count() > 1;

        signals.requires_cross_context_reasoning = CROSS_CONTEXT_PATTERN.is_match(query);
        signals.involves_temporal_reasoning = TEMPORAL_PATTERN.is_match(query);
        signals.asks_about_patterns = PATTERN_ANALYSIS.is_match(query);
        signals.debugging_task = DEBUGGING_PATTERN.is_match(query);
        signals.requires_exhaustive_search = EXHAUSTIVE_PATTERN.is_match(query);
        signals.security_review_task = SECURITY_PATTERN.is_match(query);
        signals.architecture_analysis = ARCHITECTURE_PATTERN.is_match(query);

        // User intent signals
        signals.user_wants_thorough = THOROUGH_PATTERN.is_match(query);
        signals.user_wants_fast = FAST_PATTERN.is_match(query);

        // Context analysis
        signals.context_has_multiple_domains = context.spans_multiple_directories();
        signals.files_span_multiple_modules = context.files.len() > 3;
        signals.recent_tool_outputs_large = context.total_tool_tokens() > 10000;

        // Historical signals
        if let Some(last_assistant) = context.last_assistant_message() {
            signals.previous_turn_was_confused = last_assistant.content.contains("I'm not sure")
                || last_assistant.content.contains("Could you clarify")
                || last_assistant.content.contains("I need more context");
        }

        signals.task_is_continuation = CONTINUATION_PATTERN.is_match(query)
            || context
                .last_user_message()
                .map_or(false, |m| m.content.len() < 50);

        signals
    }

    /// Determine if RLM should activate for the given query and context.
    pub fn should_activate(&self, query: &str, context: &SessionContext) -> ActivationDecision {
        if self.force_activation {
            return ActivationDecision::activate(
                "Force activation enabled",
                100,
                TaskComplexitySignals::default(),
            );
        }

        let signals = self.analyze(query, context);
        let score = signals.score();
        let active = signals.active_signals();

        if score >= self.activation_threshold {
            // Format reason to match Python test expectations
            let reason = if active.is_empty() {
                format!("complexity_score:{}", score)
            } else {
                format!("complexity_score:{}:{}", score, active.join("+"))
            };
            ActivationDecision::activate(reason, score, signals)
        } else {
            // Return "simple_task" to match Python test expectations
            ActivationDecision::skip("simple_task", score, signals)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signals_score() {
        let mut signals = TaskComplexitySignals::default();
        assert_eq!(signals.score(), 0);

        signals.architecture_analysis = true;
        assert_eq!(signals.score(), 3);

        signals.debugging_task = true;
        assert_eq!(signals.score(), 5);

        signals.user_wants_fast = true;
        assert_eq!(signals.score(), 2);
    }

    #[test]
    fn test_classifier_simple_query() {
        let classifier = PatternClassifier::new();
        let ctx = SessionContext::new();

        let decision = classifier.should_activate("What is 2 + 2?", &ctx);
        assert!(!decision.should_activate);
    }

    #[test]
    fn test_classifier_complex_query() {
        let classifier = PatternClassifier::new();
        let ctx = SessionContext::new();

        let decision = classifier.should_activate(
            "Analyze the architecture and find all security issues",
            &ctx,
        );
        assert!(decision.should_activate);
        assert!(decision.signals.architecture_analysis);
        assert!(decision.signals.security_review_task);
        assert!(decision.signals.requires_exhaustive_search);
    }

    #[test]
    fn test_classifier_multi_file() {
        let classifier = PatternClassifier::new();
        let mut ctx = SessionContext::new();
        ctx.cache_file("/src/lib.rs", "");
        ctx.cache_file("/src/main.rs", "");
        ctx.cache_file("/tests/test.rs", "");
        ctx.cache_file("/src/utils/helpers.rs", "");

        let decision = classifier.should_activate("How do these files interact?", &ctx);
        assert!(decision.should_activate);
        assert!(decision.signals.requires_cross_context_reasoning);
        assert!(decision.signals.files_span_multiple_modules);
    }

    #[test]
    fn test_classifier_user_wants_fast() {
        let classifier = PatternClassifier::new();
        let ctx = SessionContext::new();

        let decision = classifier.should_activate("Just quickly show me the main function", &ctx);
        assert!(!decision.should_activate);
        assert!(decision.signals.user_wants_fast);
    }

    #[test]
    fn test_classifier_thorough_override() {
        let classifier = PatternClassifier::new();
        let ctx = SessionContext::new();

        let decision =
            classifier.should_activate("Be thorough and check the authentication flow", &ctx);
        assert!(decision.should_activate);
        assert!(decision.signals.user_wants_thorough);
        assert!(decision.signals.security_review_task);
    }

    #[test]
    fn test_active_signals() {
        let mut signals = TaskComplexitySignals::default();
        signals.debugging_task = true;
        signals.architecture_analysis = true;

        let active = signals.active_signals();
        assert!(active.contains(&"debugging"));
        assert!(active.contains(&"architecture_analysis"));
    }

    #[test]
    fn test_force_activation() {
        let mut classifier = PatternClassifier::new();
        classifier.force_activation = true;

        let ctx = SessionContext::new();
        let decision = classifier.should_activate("simple", &ctx);
        assert!(decision.should_activate);
        assert_eq!(decision.score, 100);
    }
}
