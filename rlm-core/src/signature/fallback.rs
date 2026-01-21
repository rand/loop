//! Fallback extraction for REPL execution timeout.
//!
//! When REPL execution exceeds max iterations or LLM calls without a clean
//! SUBMIT, this module provides fallback extraction to salvage partial results.
//!
//! # SPEC-27: Fallback Extraction
//!
//! - SPEC-27.01: Trigger on max iterations/LLM calls exceeded
//! - SPEC-27.02: ExtractFallback with history and variables
//! - SPEC-27.03: Extraction prompt template
//! - SPEC-27.04: ExecutionResult with confidence
//!
//! # Example
//!
//! ```rust,ignore
//! use rlm_core::signature::{Signature, ExecutionResult, FallbackExtractor};
//!
//! let extractor = FallbackExtractor::<MySignature>::new(llm_client)
//!     .with_extraction_model("claude-3-5-haiku-20241022");
//!
//! let result = extractor.extract(&history, &variables).await?;
//!
//! match result {
//!     ExecutionResult::Submitted(outputs) => {
//!         println!("Clean submission: {:?}", outputs);
//!     }
//!     ExecutionResult::Extracted { outputs, confidence } => {
//!         println!("Extracted with {}% confidence: {:?}", confidence * 100.0, outputs);
//!     }
//!     ExecutionResult::Failed { reason } => {
//!         eprintln!("Failed: {}", reason);
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Signature;

/// Result of REPL execution with fallback support (SPEC-27.04).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionResult<O> {
    /// Clean termination via SUBMIT.
    Submitted(O),

    /// Extracted via fallback when max iterations exceeded.
    Extracted {
        /// The extracted outputs.
        outputs: O,
        /// Confidence in extraction (0.0 - 1.0).
        confidence: f64,
        /// Reason fallback was triggered.
        trigger_reason: FallbackTrigger,
    },

    /// Failed to extract outputs.
    Failed {
        /// Reason for failure.
        reason: String,
        /// Trigger that caused fallback attempt.
        trigger: FallbackTrigger,
    },
}

impl<O> ExecutionResult<O> {
    /// Create a submitted result.
    pub fn submitted(outputs: O) -> Self {
        Self::Submitted(outputs)
    }

    /// Create an extracted result.
    pub fn extracted(outputs: O, confidence: f64, trigger: FallbackTrigger) -> Self {
        Self::Extracted {
            outputs,
            confidence: confidence.clamp(0.0, 1.0),
            trigger_reason: trigger,
        }
    }

    /// Create a failed result.
    pub fn failed(reason: impl Into<String>, trigger: FallbackTrigger) -> Self {
        Self::Failed {
            reason: reason.into(),
            trigger,
        }
    }

    /// Check if this was a clean submission.
    pub fn is_submitted(&self) -> bool {
        matches!(self, Self::Submitted(_))
    }

    /// Check if this was an extraction.
    pub fn is_extracted(&self) -> bool {
        matches!(self, Self::Extracted { .. })
    }

    /// Check if extraction failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get outputs if available (from either Submitted or Extracted).
    pub fn outputs(&self) -> Option<&O> {
        match self {
            Self::Submitted(o) => Some(o),
            Self::Extracted { outputs, .. } => Some(outputs),
            Self::Failed { .. } => None,
        }
    }

    /// Get confidence (1.0 for submitted, actual for extracted, 0.0 for failed).
    pub fn confidence(&self) -> f64 {
        match self {
            Self::Submitted(_) => 1.0,
            Self::Extracted { confidence, .. } => *confidence,
            Self::Failed { .. } => 0.0,
        }
    }

    /// Map the outputs to a new type.
    pub fn map<U, F: FnOnce(O) -> U>(self, f: F) -> ExecutionResult<U> {
        match self {
            Self::Submitted(o) => ExecutionResult::Submitted(f(o)),
            Self::Extracted {
                outputs,
                confidence,
                trigger_reason,
            } => ExecutionResult::Extracted {
                outputs: f(outputs),
                confidence,
                trigger_reason,
            },
            Self::Failed { reason, trigger } => ExecutionResult::Failed { reason, trigger },
        }
    }
}

/// Reason for triggering fallback extraction (SPEC-27.01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTrigger {
    /// Max iterations reached.
    MaxIterations,
    /// Max LLM calls reached.
    MaxLLMCalls,
    /// Execution timeout.
    Timeout,
    /// Manual trigger (for testing).
    Manual,
}

impl std::fmt::Display for FallbackTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxIterations => write!(f, "max iterations reached"),
            Self::MaxLLMCalls => write!(f, "max LLM calls reached"),
            Self::Timeout => write!(f, "execution timeout"),
            Self::Manual => write!(f, "manual trigger"),
        }
    }
}

/// REPL history entry for extraction context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Entry type (code, output, error, llm_call, etc.)
    pub entry_type: HistoryEntryType,
    /// Content of the entry.
    pub content: String,
    /// Timestamp (milliseconds since start).
    pub timestamp_ms: u64,
}

/// Type of history entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryEntryType {
    /// Code executed.
    Code,
    /// Output from execution.
    Output,
    /// Error from execution.
    Error,
    /// LLM query.
    LLMQuery,
    /// LLM response.
    LLMResponse,
    /// Variable assignment.
    VariableSet,
}

/// REPL execution history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplHistory {
    /// History entries in chronological order.
    pub entries: Vec<HistoryEntry>,
    /// Current iteration count.
    pub iteration_count: usize,
    /// Current LLM call count.
    pub llm_call_count: usize,
    /// Total execution time in milliseconds.
    pub total_time_ms: u64,
}

impl ReplHistory {
    /// Create new empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a code execution entry.
    pub fn add_code(&mut self, code: impl Into<String>, timestamp_ms: u64) {
        self.entries.push(HistoryEntry {
            entry_type: HistoryEntryType::Code,
            content: code.into(),
            timestamp_ms,
        });
        self.iteration_count += 1;
    }

    /// Add an output entry.
    pub fn add_output(&mut self, output: impl Into<String>, timestamp_ms: u64) {
        self.entries.push(HistoryEntry {
            entry_type: HistoryEntryType::Output,
            content: output.into(),
            timestamp_ms,
        });
    }

    /// Add an error entry.
    pub fn add_error(&mut self, error: impl Into<String>, timestamp_ms: u64) {
        self.entries.push(HistoryEntry {
            entry_type: HistoryEntryType::Error,
            content: error.into(),
            timestamp_ms,
        });
    }

    /// Add an LLM query entry.
    pub fn add_llm_query(&mut self, query: impl Into<String>, timestamp_ms: u64) {
        self.entries.push(HistoryEntry {
            entry_type: HistoryEntryType::LLMQuery,
            content: query.into(),
            timestamp_ms,
        });
        self.llm_call_count += 1;
    }

    /// Add an LLM response entry.
    pub fn add_llm_response(&mut self, response: impl Into<String>, timestamp_ms: u64) {
        self.entries.push(HistoryEntry {
            entry_type: HistoryEntryType::LLMResponse,
            content: response.into(),
            timestamp_ms,
        });
    }

    /// Format history for extraction prompt.
    pub fn format_for_prompt(&self, max_entries: usize) -> String {
        let entries: Vec<_> = if self.entries.len() > max_entries {
            // Take first few and last entries
            let take_start = max_entries / 3;
            let take_end = max_entries - take_start;
            let mut result = self.entries[..take_start].to_vec();
            result.push(HistoryEntry {
                entry_type: HistoryEntryType::Output,
                content: format!("... [{} entries omitted] ...", self.entries.len() - max_entries),
                timestamp_ms: 0,
            });
            result.extend(self.entries[self.entries.len() - take_end..].to_vec());
            result
        } else {
            self.entries.clone()
        };

        let mut output = String::new();
        for entry in entries {
            let prefix = match entry.entry_type {
                HistoryEntryType::Code => ">>> ",
                HistoryEntryType::Output => "    ",
                HistoryEntryType::Error => "!!! ",
                HistoryEntryType::LLMQuery => "[LLM Query] ",
                HistoryEntryType::LLMResponse => "[LLM Response] ",
                HistoryEntryType::VariableSet => "[Set] ",
            };

            // Truncate long content
            let content = if entry.content.len() > 500 {
                format!("{}... [truncated]", &entry.content[..500])
            } else {
                entry.content.clone()
            };

            for line in content.lines() {
                output.push_str(prefix);
                output.push_str(line);
                output.push('\n');
            }
        }
        output
    }
}

/// Configuration for fallback extraction.
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Max history entries to include in extraction prompt.
    pub max_history_entries: usize,
    /// Max variables to include in extraction prompt.
    pub max_variables: usize,
    /// Model to use for extraction (smaller/cheaper is fine).
    pub extraction_model: Option<String>,
    /// Temperature for extraction (lower = more deterministic).
    pub extraction_temperature: f64,
    /// Max tokens for extraction response.
    pub max_extraction_tokens: u32,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            max_history_entries: 50,
            max_variables: 20,
            extraction_model: None, // Use default
            extraction_temperature: 0.0,
            max_extraction_tokens: 2048,
        }
    }
}

/// Fallback extractor for a signature (SPEC-27.02).
pub struct FallbackExtractor<S: Signature> {
    config: FallbackConfig,
    _marker: PhantomData<S>,
}

impl<S: Signature> FallbackExtractor<S> {
    /// Create a new fallback extractor.
    pub fn new() -> Self {
        Self {
            config: FallbackConfig::default(),
            _marker: PhantomData,
        }
    }

    /// Create with custom config.
    pub fn with_config(config: FallbackConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// Set the extraction model.
    pub fn with_extraction_model(mut self, model: impl Into<String>) -> Self {
        self.config.extraction_model = Some(model.into());
        self
    }

    /// Check if fallback should be triggered (SPEC-27.01).
    pub fn should_trigger(
        &self,
        history: &ReplHistory,
        limits: &ExecutionLimits,
    ) -> Option<FallbackTrigger> {
        if history.iteration_count >= limits.max_iterations {
            return Some(FallbackTrigger::MaxIterations);
        }
        if history.llm_call_count >= limits.max_llm_calls {
            return Some(FallbackTrigger::MaxLLMCalls);
        }
        if history.total_time_ms >= limits.timeout_ms {
            return Some(FallbackTrigger::Timeout);
        }
        None
    }

    /// Generate the extraction prompt (SPEC-27.03).
    pub fn extraction_prompt(
        &self,
        history: &ReplHistory,
        variables: &HashMap<String, Value>,
    ) -> String {
        let mut prompt = String::new();

        // Header
        prompt.push_str("# Fallback Output Extraction\n\n");
        prompt.push_str("The REPL execution exceeded limits before completing. ");
        prompt.push_str("Extract the required outputs from the history and variables below.\n\n");

        // History section
        prompt.push_str("## REPL History\n\n");
        prompt.push_str("```\n");
        prompt.push_str(&history.format_for_prompt(self.config.max_history_entries));
        prompt.push_str("```\n\n");

        // Variables section
        prompt.push_str("## Current Variables\n\n");
        prompt.push_str("```json\n");

        // Limit and truncate variables
        let vars_to_show: HashMap<_, _> = variables
            .iter()
            .take(self.config.max_variables)
            .map(|(k, v)| {
                let v_str = v.to_string();
                let truncated = if v_str.len() > 1000 {
                    Value::String(format!("{}... [truncated, {} chars total]", &v_str[..1000], v_str.len()))
                } else {
                    v.clone()
                };
                (k.clone(), truncated)
            })
            .collect();

        prompt.push_str(&serde_json::to_string_pretty(&vars_to_show).unwrap_or_default());
        prompt.push_str("\n```\n\n");

        // Required outputs
        prompt.push_str("## Required Outputs\n\n");
        prompt.push_str("Extract the following fields based on the history and variables:\n\n");

        for field in S::output_fields() {
            prompt.push_str(&format!("- **{}**: {}\n", field.name, field.description));
            prompt.push_str(&format!("  - Type: {:?}\n", field.field_type));
            if !field.required {
                prompt.push_str("  - Optional\n");
            }
        }
        prompt.push('\n');

        // Output format
        prompt.push_str("## Response Format\n\n");
        prompt.push_str("Return a JSON object with the required fields. ");
        prompt.push_str("If a value cannot be determined, use null for optional fields or your best guess for required fields.\n\n");
        prompt.push_str("Also include a `_confidence` field (0.0-1.0) indicating your confidence in the extraction.\n\n");
        prompt.push_str("```json\n");
        prompt.push_str(&Self::generate_output_template());
        prompt.push_str("\n```\n");

        prompt
    }

    /// Generate output template with placeholders.
    fn generate_output_template() -> String {
        let mut obj = serde_json::Map::new();

        for field in S::output_fields() {
            let placeholder = match &field.field_type {
                super::types::FieldType::String => Value::String("<extracted value>".to_string()),
                super::types::FieldType::Integer => Value::String("<integer>".to_string()),
                super::types::FieldType::Float => Value::String("<number>".to_string()),
                super::types::FieldType::Boolean => Value::String("<true|false>".to_string()),
                super::types::FieldType::List(_) => Value::Array(vec![Value::String("<items>".to_string())]),
                _ => Value::String("<value>".to_string()),
            };
            obj.insert(field.name, placeholder);
        }

        obj.insert("_confidence".to_string(), Value::String("<0.0-1.0>".to_string()));

        serde_json::to_string_pretty(&Value::Object(obj)).unwrap_or_default()
    }

    /// Parse extraction response into ExecutionResult.
    pub fn parse_extraction_response(
        &self,
        response: &str,
        trigger: FallbackTrigger,
    ) -> ExecutionResult<S::Outputs> {
        // Try to extract JSON
        let json_str = extract_json_block(response);

        // Parse JSON
        let value: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                return ExecutionResult::failed(
                    format!("Failed to parse extraction response: {}", e),
                    trigger,
                );
            }
        };

        // Extract confidence
        let confidence = value
            .get("_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        // Remove _confidence before parsing outputs
        let mut output_value = value.clone();
        if let Some(obj) = output_value.as_object_mut() {
            obj.remove("_confidence");
        }

        // Parse into output type
        match serde_json::from_value::<S::Outputs>(output_value) {
            Ok(outputs) => ExecutionResult::extracted(outputs, confidence, trigger),
            Err(e) => ExecutionResult::failed(
                format!("Failed to parse extracted outputs: {}", e),
                trigger,
            ),
        }
    }
}

impl<S: Signature> Default for FallbackExtractor<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution limits that trigger fallback.
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum REPL iterations.
    pub max_iterations: usize,
    /// Maximum LLM API calls.
    pub max_llm_calls: usize,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_llm_calls: 5,
            timeout_ms: 60_000, // 1 minute
        }
    }
}

impl ExecutionLimits {
    /// Create new limits.
    pub fn new(max_iterations: usize, max_llm_calls: usize, timeout_ms: u64) -> Self {
        Self {
            max_iterations,
            max_llm_calls,
            timeout_ms,
        }
    }

    /// Create lenient limits for complex tasks.
    pub fn lenient() -> Self {
        Self {
            max_iterations: 25,
            max_llm_calls: 15,
            timeout_ms: 300_000, // 5 minutes
        }
    }

    /// Create strict limits for simple tasks.
    pub fn strict() -> Self {
        Self {
            max_iterations: 5,
            max_llm_calls: 3,
            timeout_ms: 30_000, // 30 seconds
        }
    }
}

/// Extract JSON from response that may contain markdown.
fn extract_json_block(response: &str) -> &str {
    // Try json code block
    if let Some(start) = response.find("```json") {
        let content_start = start + 7;
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Try generic code block
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        let content_start = response[content_start..]
            .find('\n')
            .map(|i| content_start + i + 1)
            .unwrap_or(content_start);
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Try raw JSON
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return &response[start..=end];
            }
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Test signature
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestOutputs {
        answer: String,
        confidence: f64,
    }

    struct TestSignature;

    impl Signature for TestSignature {
        type Inputs = ();
        type Outputs = TestOutputs;

        fn instructions() -> &'static str {
            "Test"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![]
        }

        fn output_fields() -> Vec<FieldSpec> {
            use super::super::types::FieldType;
            vec![
                FieldSpec::new("answer", FieldType::String).with_description("The answer"),
                FieldSpec::new("confidence", FieldType::Float)
                    .with_description("Confidence score"),
            ]
        }
    }

    #[test]
    fn test_execution_result_submitted() {
        let outputs = TestOutputs {
            answer: "test".to_string(),
            confidence: 0.9,
        };
        let result: ExecutionResult<TestOutputs> = ExecutionResult::submitted(outputs.clone());

        assert!(result.is_submitted());
        assert!(!result.is_extracted());
        assert!(!result.is_failed());
        assert_eq!(result.confidence(), 1.0);
        assert_eq!(result.outputs(), Some(&outputs));
    }

    #[test]
    fn test_execution_result_extracted() {
        let outputs = TestOutputs {
            answer: "extracted".to_string(),
            confidence: 0.7,
        };
        let result: ExecutionResult<TestOutputs> =
            ExecutionResult::extracted(outputs.clone(), 0.8, FallbackTrigger::MaxIterations);

        assert!(!result.is_submitted());
        assert!(result.is_extracted());
        assert_eq!(result.confidence(), 0.8);
        assert_eq!(result.outputs(), Some(&outputs));
    }

    #[test]
    fn test_execution_result_failed() {
        let result: ExecutionResult<TestOutputs> =
            ExecutionResult::failed("timeout", FallbackTrigger::Timeout);

        assert!(result.is_failed());
        assert_eq!(result.confidence(), 0.0);
        assert!(result.outputs().is_none());
    }

    #[test]
    fn test_repl_history() {
        let mut history = ReplHistory::new();
        history.add_code("x = 1 + 1", 0);
        history.add_output("2", 100);
        history.add_llm_query("What is x?", 200);
        history.add_llm_response("x is 2", 500);

        assert_eq!(history.iteration_count, 1);
        assert_eq!(history.llm_call_count, 1);
        assert_eq!(history.entries.len(), 4);

        let formatted = history.format_for_prompt(100);
        assert!(formatted.contains(">>> x = 1 + 1"));
        assert!(formatted.contains("2"));
        assert!(formatted.contains("[LLM Query]"));
    }

    #[test]
    fn test_should_trigger() {
        let extractor = FallbackExtractor::<TestSignature>::new();
        let limits = ExecutionLimits::new(5, 3, 10000);

        let mut history = ReplHistory::new();
        assert!(extractor.should_trigger(&history, &limits).is_none());

        // Trigger max iterations
        for i in 0..5 {
            history.add_code(&format!("code {}", i), i as u64 * 100);
        }
        assert_eq!(
            extractor.should_trigger(&history, &limits),
            Some(FallbackTrigger::MaxIterations)
        );
    }

    #[test]
    fn test_extraction_prompt() {
        let extractor = FallbackExtractor::<TestSignature>::new();
        let history = ReplHistory::new();
        let variables = HashMap::new();

        let prompt = extractor.extraction_prompt(&history, &variables);

        assert!(prompt.contains("Fallback Output Extraction"));
        assert!(prompt.contains("REPL History"));
        assert!(prompt.contains("Current Variables"));
        assert!(prompt.contains("Required Outputs"));
        assert!(prompt.contains("answer"));
        assert!(prompt.contains("confidence"));
        assert!(prompt.contains("_confidence"));
    }

    #[test]
    fn test_parse_extraction_response() {
        let extractor = FallbackExtractor::<TestSignature>::new();

        let response = r#"{"answer": "extracted answer", "confidence": 0.9, "_confidence": 0.85}"#;
        let result = extractor.parse_extraction_response(response, FallbackTrigger::MaxIterations);

        assert!(result.is_extracted());
        let outputs = result.outputs().unwrap();
        assert_eq!(outputs.answer, "extracted answer");
        assert!((result.confidence() - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_parse_extraction_response_markdown() {
        let extractor = FallbackExtractor::<TestSignature>::new();

        let response = r#"
Here is the extracted data:

```json
{
    "answer": "from markdown",
    "confidence": 0.8,
    "_confidence": 0.75
}
```
"#;
        let result = extractor.parse_extraction_response(response, FallbackTrigger::Timeout);

        assert!(result.is_extracted());
        assert_eq!(result.outputs().unwrap().answer, "from markdown");
    }

    #[test]
    fn test_parse_extraction_response_failure() {
        let extractor = FallbackExtractor::<TestSignature>::new();

        let response = "This is not valid JSON";
        let result = extractor.parse_extraction_response(response, FallbackTrigger::Manual);

        assert!(result.is_failed());
    }

    #[test]
    fn test_execution_limits_presets() {
        let default = ExecutionLimits::default();
        assert_eq!(default.max_iterations, 10);

        let lenient = ExecutionLimits::lenient();
        assert!(lenient.max_iterations > default.max_iterations);

        let strict = ExecutionLimits::strict();
        assert!(strict.max_iterations < default.max_iterations);
    }

    #[test]
    fn test_fallback_trigger_display() {
        assert!(FallbackTrigger::MaxIterations.to_string().contains("iterations"));
        assert!(FallbackTrigger::MaxLLMCalls.to_string().contains("LLM"));
        assert!(FallbackTrigger::Timeout.to_string().contains("timeout"));
    }

    #[test]
    fn test_execution_result_map() {
        let result: ExecutionResult<i32> = ExecutionResult::submitted(42);
        let mapped = result.map(|x| x.to_string());

        assert!(mapped.is_submitted());
        assert_eq!(mapped.outputs(), Some(&"42".to_string()));
    }
}
