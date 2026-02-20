//! Typed signatures system for LLM I/O contracts.
//!
//! This module implements DSPy-inspired typed signatures that enable:
//! - **Type-safe I/O contracts**: Define inputs and outputs with validation
//! - **Automatic prompt generation**: Generate prompts from type annotations
//! - **Output validation**: Validate LLM responses before returning
//! - **Module composition**: Chain signatures where output types match input types
//!
//! # Overview
//!
//! A signature defines the contract between your code and the LLM. It specifies:
//! - Input fields with types and descriptions
//! - Output fields with types and descriptions
//! - Task instructions for the LLM
//!
//! # Example
//!
//! ```rust,ignore
//! use rlm_core::signature::{Signature, FieldSpec, FieldType};
//!
//! // Manual implementation (derive macro coming in loop-jqo)
//! struct AnalyzeCode;
//!
//! impl Signature for AnalyzeCode {
//!     type Inputs = AnalyzeCodeInputs;
//!     type Outputs = AnalyzeCodeOutputs;
//!
//!     fn instructions() -> &'static str {
//!         "Analyze the provided code for security vulnerabilities"
//!     }
//!
//!     fn input_fields() -> Vec<FieldSpec> {
//!         vec![
//!             FieldSpec::new("code", FieldType::String)
//!                 .with_description("Source code to analyze"),
//!             FieldSpec::new("language", FieldType::String)
//!                 .with_description("Programming language"),
//!         ]
//!     }
//!
//!     fn output_fields() -> Vec<FieldSpec> {
//!         vec![
//!             FieldSpec::new("vulnerabilities", FieldType::list(FieldType::String))
//!                 .with_description("List of vulnerabilities found"),
//!             FieldSpec::new("severity", FieldType::enum_of(["low", "medium", "high", "critical"]))
//!                 .with_description("Overall severity rating"),
//!         ]
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! The signature system consists of:
//!
//! - [`Signature`]: Core trait defining I/O contracts
//! - [`FieldSpec`]: Field metadata (name, type, description)
//! - [`FieldType`]: Type information for validation
//! - [`ValidationError`]: Errors from validation
//! - [`ParseError`]: Errors from parsing LLM responses
//!
//! # Related Specs
//!
//! - SPEC-20.01: Signature Trait
//! - SPEC-20.02: Field Specification
//! - SPEC-20.03: Signature Validation

pub mod fallback;
pub mod submit;
pub mod types;
pub mod validation;

pub use fallback::{
    ExecutionLimits, ExecutionResult, FallbackConfig, FallbackExtractor, FallbackTrigger,
    HistoryEntry, HistoryEntryType, ReplHistory,
};
pub use submit::{SignatureRegistration, SubmitError, SubmitMetrics, SubmitResult};
pub use types::{FieldSpec, FieldType};
pub use validation::{
    apply_defaults, validate_fields, validate_value, ValidationError, ValidationResult,
};

// Re-export derive macro
pub use rlm_core_derive::Signature;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::fmt;

/// Error that occurs when parsing an LLM response into outputs.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Response was not valid JSON
    InvalidJson {
        /// The parse error message
        message: String,
        /// Preview of the response that failed to parse
        response_preview: String,
    },

    /// JSON parsed but didn't match expected structure
    StructureMismatch {
        /// What was expected
        expected: String,
        /// What was found
        got: String,
    },

    /// Validation failed after parsing
    ValidationFailed(Vec<ValidationError>),

    /// Response was empty or contained no extractable content
    EmptyResponse,

    /// Custom parse error
    Custom(String),
}

impl ParseError {
    /// Create an invalid JSON error from a serde error.
    pub fn invalid_json(err: &serde_json::Error, response: &str) -> Self {
        Self::InvalidJson {
            message: err.to_string(),
            response_preview: truncate(response, 200),
        }
    }

    /// Create a structure mismatch error.
    pub fn structure_mismatch(expected: impl Into<String>, got: impl Into<String>) -> Self {
        Self::StructureMismatch {
            expected: expected.into(),
            got: got.into(),
        }
    }

    /// Create from validation errors.
    pub fn validation_failed(errors: Vec<ValidationError>) -> Self {
        Self::ValidationFailed(errors)
    }

    /// Get a human-readable error message.
    pub fn to_user_message(&self) -> String {
        match self {
            Self::InvalidJson {
                message,
                response_preview,
            } => {
                format!(
                    "Failed to parse response as JSON: {}. Response: {}",
                    message, response_preview
                )
            }
            Self::StructureMismatch { expected, got } => {
                format!(
                    "Response structure mismatch: expected {}, got {}",
                    expected, got
                )
            }
            Self::ValidationFailed(errors) => {
                let messages: Vec<_> = errors.iter().map(|e| e.to_user_message()).collect();
                format!("Validation failed:\n  - {}", messages.join("\n  - "))
            }
            Self::EmptyResponse => "LLM returned an empty response".to_string(),
            Self::Custom(msg) => msg.clone(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_user_message())
    }
}

impl std::error::Error for ParseError {}

/// Core trait defining a typed LLM I/O contract.
///
/// A Signature specifies:
/// - Input type that must be serializable
/// - Output type that must be deserializable
/// - Task instructions for the LLM
/// - Field specifications for validation and prompt generation
///
/// # Implementing Signature
///
/// Signatures can be implemented manually or (in the future) via derive macro.
///
/// ```rust,ignore
/// use rlm_core::signature::{Signature, FieldSpec, FieldType, ParseError};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Clone)]
/// struct SummarizeInputs {
///     text: String,
///     max_length: Option<u32>,
/// }
///
/// #[derive(Deserialize, Clone)]
/// struct SummarizeOutputs {
///     summary: String,
///     key_points: Vec<String>,
/// }
///
/// struct Summarize;
///
/// impl Signature for Summarize {
///     type Inputs = SummarizeInputs;
///     type Outputs = SummarizeOutputs;
///
///     fn instructions() -> &'static str {
///         "Summarize the given text, extracting key points"
///     }
///
///     fn input_fields() -> Vec<FieldSpec> {
///         vec![
///             FieldSpec::new("text", FieldType::String)
///                 .with_description("Text to summarize"),
///             FieldSpec::new("max_length", FieldType::Integer)
///                 .with_description("Maximum summary length in words")
///                 .optional(),
///         ]
///     }
///
///     fn output_fields() -> Vec<FieldSpec> {
///         vec![
///             FieldSpec::new("summary", FieldType::String)
///                 .with_description("Concise summary of the text"),
///             FieldSpec::new("key_points", FieldType::list(FieldType::String))
///                 .with_description("Main points from the text"),
///         ]
///     }
/// }
/// ```
pub trait Signature: Send + Sync + 'static {
    /// Input type (must be serializable).
    type Inputs: Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Output type (must be deserializable).
    type Outputs: Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Task instructions for the LLM.
    ///
    /// This should be a clear, concise description of the task.
    fn instructions() -> &'static str;

    /// Input field specifications.
    ///
    /// Used for:
    /// - Prompt generation
    /// - Input validation
    /// - Documentation
    fn input_fields() -> Vec<FieldSpec>;

    /// Output field specifications.
    ///
    /// Used for:
    /// - Response parsing hints
    /// - Output validation
    /// - Documentation
    fn output_fields() -> Vec<FieldSpec>;

    /// Generate a prompt from inputs.
    ///
    /// Default implementation creates a structured prompt with:
    /// - Instructions
    /// - Input field values
    /// - Output field specifications
    fn to_prompt(inputs: &Self::Inputs) -> String
    where
        Self: Sized,
    {
        let mut prompt = String::new();

        // Instructions
        prompt.push_str("## Task\n\n");
        prompt.push_str(Self::instructions());
        prompt.push_str("\n\n");

        // Inputs
        prompt.push_str("## Inputs\n\n");
        let input_json = serde_json::to_value(inputs).unwrap_or(Value::Null);
        for field in Self::input_fields() {
            let value = input_json.get(&field.name);
            let label = field.display_label();
            match value {
                Some(v) => {
                    prompt.push_str(&format!("**{}**: {}\n", label, format_value(v)));
                }
                None if !field.required => {
                    // Skip optional missing fields
                }
                None => {
                    prompt.push_str(&format!("**{}**: (not provided)\n", label));
                }
            }
        }
        prompt.push('\n');

        // Output specification
        prompt.push_str("## Required Output\n\n");
        prompt.push_str("Respond with a JSON object containing:\n\n");
        for field in Self::output_fields() {
            prompt.push_str(&format!("- {}\n", field.to_prompt_line()));
        }
        prompt.push_str("\n```json\n");
        prompt.push_str(&generate_output_template::<Self>());
        prompt.push_str("\n```\n");

        prompt
    }

    /// Parse outputs from an LLM response.
    ///
    /// Default implementation:
    /// 1. Extracts JSON from the response (handles markdown code blocks)
    /// 2. Parses into the output type
    /// 3. Validates against output field specs
    fn from_response(response: &str) -> Result<Self::Outputs, ParseError>
    where
        Self: Sized,
    {
        let response = response.trim();

        if response.is_empty() {
            return Err(ParseError::EmptyResponse);
        }

        // Extract JSON from response (may be wrapped in markdown)
        let json_str = extract_json(response);

        // Parse JSON
        let value: Value =
            serde_json::from_str(json_str).map_err(|e| ParseError::invalid_json(&e, json_str))?;

        // Validate against output fields
        if let Err(errors) = validate_fields(&value, &Self::output_fields()) {
            return Err(ParseError::validation_failed(errors));
        }

        // Parse into output type
        serde_json::from_value(value.clone()).map_err(|e| {
            ParseError::structure_mismatch(std::any::type_name::<Self::Outputs>(), e.to_string())
        })
    }

    /// Get the signature name (defaults to type name).
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Generate a JSON schema for the output type.
    fn output_schema() -> Value
    where
        Self: Sized,
    {
        let output_fields = Self::output_fields();

        let properties: serde_json::Map<String, Value> = output_fields
            .iter()
            .map(|f| (f.name.clone(), f.field_type.to_json_schema()))
            .collect();

        let required: Vec<String> = output_fields
            .iter()
            .filter(|f| f.required)
            .map(|f| f.name.clone())
            .collect();

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }
}

/// Extract JSON from a response that may contain markdown or other text.
fn extract_json(response: &str) -> &str {
    // Try to find JSON in code blocks
    if let Some(start) = response.find("```json") {
        let content_start = start + 7;
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Try generic code block
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        // Skip language identifier if present
        let content_start = response[content_start..]
            .find('\n')
            .map(|i| content_start + i + 1)
            .unwrap_or(content_start);
        if let Some(end) = response[content_start..].find("```") {
            return response[content_start..content_start + end].trim();
        }
    }

    // Try to find raw JSON object
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return &response[start..=end];
            }
        }
    }

    // Return as-is
    response
}

/// Generate an output template with placeholder values.
fn generate_output_template<S: Signature>() -> String {
    let mut obj = serde_json::Map::new();

    for field in S::output_fields() {
        let placeholder = field_placeholder(&field.field_type);
        obj.insert(field.name.clone(), placeholder);
    }

    serde_json::to_string_pretty(&Value::Object(obj)).unwrap_or_default()
}

/// Generate a placeholder value for a field type.
fn field_placeholder(field_type: &FieldType) -> Value {
    match field_type {
        FieldType::String => Value::String("<string>".to_string()),
        FieldType::Integer => Value::String("<integer>".to_string()),
        FieldType::Float => Value::String("<number>".to_string()),
        FieldType::Boolean => Value::String("<true|false>".to_string()),
        FieldType::List(inner) => Value::Array(vec![field_placeholder(inner)]),
        FieldType::Object(fields) => {
            let mut obj = serde_json::Map::new();
            for f in fields {
                obj.insert(f.name.clone(), field_placeholder(&f.field_type));
            }
            Value::Object(obj)
        }
        FieldType::Enum(values) => Value::String(values.join("|")),
        FieldType::Custom(name) => Value::String(format!("<{}>", name)),
    }
}

/// Format a JSON value for display in a prompt.
fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(arr) if arr.len() <= 3 => {
            let items: Vec<_> = arr.iter().map(|v| format_value(v)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Array(arr) => {
            format!("[{} items]", arr.len())
        }
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        other => other.to_string(),
    }
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Test signature implementation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestInputs {
        query: String,
        limit: Option<u32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestOutputs {
        answer: String,
        confidence: f64,
    }

    struct TestSignature;

    impl Signature for TestSignature {
        type Inputs = TestInputs;
        type Outputs = TestOutputs;

        fn instructions() -> &'static str {
            "Answer the query with confidence"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![
                FieldSpec::new("query", FieldType::String).with_description("The question"),
                FieldSpec::new("limit", FieldType::Integer)
                    .with_description("Max response length")
                    .optional(),
            ]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![
                FieldSpec::new("answer", FieldType::String).with_description("The answer"),
                FieldSpec::new("confidence", FieldType::Float)
                    .with_description("Confidence score 0-1"),
            ]
        }
    }

    #[test]
    fn test_to_prompt() {
        let inputs = TestInputs {
            query: "What is Rust?".to_string(),
            limit: Some(100),
        };

        let prompt = TestSignature::to_prompt(&inputs);

        assert!(prompt.contains("Answer the query with confidence"));
        assert!(prompt.contains("What is Rust?"));
        assert!(prompt.contains("answer"));
        assert!(prompt.contains("confidence"));
    }

    #[test]
    fn test_from_response_json() {
        let response = r#"{"answer": "Rust is a programming language", "confidence": 0.95}"#;

        let outputs = TestSignature::from_response(response).unwrap();

        assert_eq!(outputs.answer, "Rust is a programming language");
        assert!((outputs.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_from_response_markdown() {
        let response = r#"
Here is my answer:

```json
{
    "answer": "Rust is awesome",
    "confidence": 0.9
}
```

I hope this helps!
"#;

        let outputs = TestSignature::from_response(response).unwrap();

        assert_eq!(outputs.answer, "Rust is awesome");
    }

    #[test]
    fn test_from_response_validation_error() {
        let response = r#"{"answer": "Test"}"#; // Missing confidence

        let result = TestSignature::from_response(response);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::ValidationFailed(_)
        ));
    }

    #[test]
    fn test_from_response_invalid_json() {
        let response = "This is not JSON";

        let result = TestSignature::from_response(response);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidJson { .. }
        ));
    }

    #[test]
    fn test_from_response_empty() {
        let result = TestSignature::from_response("");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::EmptyResponse));
    }

    #[test]
    fn test_output_schema() {
        let schema = TestSignature::output_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["answer"].is_object());
        assert!(schema["properties"]["confidence"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&Value::String("answer".to_string())));
    }

    #[test]
    fn test_extract_json_code_block() {
        let input = "Here's the result:\n```json\n{\"key\": \"value\"}\n```\nDone!";
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_raw() {
        let input = r#"Result: {"key": "value"} was found"#;
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::EmptyResponse;
        assert!(err.to_string().contains("empty"));

        let err = ParseError::validation_failed(vec![ValidationError::missing_field(
            "test",
            FieldType::String,
        )]);
        assert!(err.to_string().contains("Validation"));
    }

    // Test with enum output
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct ClassifyOutputs {
        category: String,
        confidence: f64,
    }

    struct ClassifySignature;

    impl Signature for ClassifySignature {
        type Inputs = TestInputs;
        type Outputs = ClassifyOutputs;

        fn instructions() -> &'static str {
            "Classify the input"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("query", FieldType::String)]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![
                FieldSpec::new(
                    "category",
                    FieldType::enum_of(["bug", "feature", "question"]),
                )
                .with_description("The category"),
                FieldSpec::new("confidence", FieldType::Float),
            ]
        }
    }

    #[test]
    fn test_enum_validation_in_response() {
        // Valid enum value
        let valid = r#"{"category": "bug", "confidence": 0.9}"#;
        assert!(ClassifySignature::from_response(valid).is_ok());

        // Invalid enum value
        let invalid = r#"{"category": "invalid", "confidence": 0.9}"#;
        let result = ClassifySignature::from_response(invalid);
        assert!(result.is_err());
    }

    // Tests for derive macro
    mod derive_tests {
        use super::*;

        /// Test signature using derive macro
        #[derive(rlm_core_derive::Signature)]
        #[signature(instructions = "Analyze code for security vulnerabilities")]
        struct AnalyzeCode {
            #[input(desc = "Source code to analyze")]
            code: String,

            #[input(desc = "Programming language", prefix = "Language")]
            language: String,

            #[input(desc = "Maximum issues to report")]
            max_issues: Option<u32>,

            #[output(desc = "List of vulnerabilities found")]
            vulnerabilities: Vec<String>,

            #[output(desc = "Overall severity rating")]
            severity: String,

            #[output(desc = "Confidence score")]
            confidence: f64,
        }

        #[test]
        fn test_derive_instructions() {
            assert_eq!(
                AnalyzeCode::instructions(),
                "Analyze code for security vulnerabilities"
            );
        }

        #[test]
        fn test_derive_input_fields() {
            let fields = AnalyzeCode::input_fields();
            assert_eq!(fields.len(), 3);

            assert_eq!(fields[0].name, "code");
            assert!(fields[0].required);

            assert_eq!(fields[1].name, "language");
            assert_eq!(fields[1].prefix, Some("Language".to_string()));

            assert_eq!(fields[2].name, "max_issues");
            assert!(!fields[2].required); // Option<T> infers optional
        }

        #[test]
        fn test_derive_output_fields() {
            let fields = AnalyzeCode::output_fields();
            assert_eq!(fields.len(), 3);

            assert_eq!(fields[0].name, "vulnerabilities");
            assert!(matches!(fields[0].field_type, FieldType::List(_)));

            assert_eq!(fields[1].name, "severity");
            assert_eq!(fields[2].name, "confidence");
        }

        #[test]
        fn test_derive_generated_structs() {
            // Test that input/output structs are generated and usable
            let inputs = AnalyzeCodeInputs {
                code: "fn main() {}".to_string(),
                language: "rust".to_string(),
                max_issues: Some(10),
            };

            assert_eq!(inputs.code, "fn main() {}");

            let outputs = AnalyzeCodeOutputs {
                vulnerabilities: vec!["SQL injection".to_string()],
                severity: "high".to_string(),
                confidence: 0.95,
            };

            assert_eq!(outputs.vulnerabilities.len(), 1);
        }

        #[test]
        fn test_derive_to_prompt() {
            let inputs = AnalyzeCodeInputs {
                code: "SELECT * FROM users".to_string(),
                language: "sql".to_string(),
                max_issues: None,
            };

            let prompt = AnalyzeCode::to_prompt(&inputs);

            assert!(prompt.contains("Analyze code for security vulnerabilities"));
            assert!(prompt.contains("SELECT * FROM users"));
            assert!(prompt.contains("sql"));
            assert!(prompt.contains("vulnerabilities"));
        }

        #[test]
        fn test_derive_from_response() {
            let response = r#"{
                "vulnerabilities": ["SQL injection possible"],
                "severity": "high",
                "confidence": 0.9
            }"#;

            let outputs = AnalyzeCode::from_response(response).unwrap();

            assert_eq!(outputs.vulnerabilities, vec!["SQL injection possible"]);
            assert_eq!(outputs.severity, "high");
            assert!((outputs.confidence - 0.9).abs() < 0.001);
        }

        /// Test with all supported types
        #[derive(rlm_core_derive::Signature)]
        #[signature(instructions = "Test all types")]
        struct AllTypes {
            #[input(desc = "A string")]
            string_field: String,

            #[input(desc = "An integer")]
            int_field: i32,

            #[input(desc = "A float")]
            float_field: f64,

            #[input(desc = "A boolean")]
            bool_field: bool,

            #[input(desc = "A list")]
            list_field: Vec<String>,

            #[input(desc = "An optional list")]
            optional_list_field: Option<Vec<i32>>,

            #[input(desc = "A fixed array")]
            array_field: [i32; 2],

            #[output(desc = "Output string")]
            output: String,
        }

        #[test]
        fn test_derive_type_inference() {
            let fields = AllTypes::input_fields();

            assert!(matches!(fields[0].field_type, FieldType::String));
            assert!(matches!(fields[1].field_type, FieldType::Integer));
            assert!(matches!(fields[2].field_type, FieldType::Float));
            assert!(matches!(fields[3].field_type, FieldType::Boolean));
            assert!(matches!(fields[4].field_type, FieldType::List(_)));
            assert!(matches!(fields[5].field_type, FieldType::List(_)));
            assert!(matches!(fields[6].field_type, FieldType::List(_)));
        }

        #[derive(rlm_core_derive::Signature)]
        #[signature(instructions = "Test enum field metadata")]
        struct EnumAnnotated {
            #[input(desc = "Severity level")]
            #[field(enum_values = "low,medium,high")]
            severity: String,

            #[output(desc = "Classification")]
            #[field(enum_values = "bug,feature,question")]
            category: String,
        }

        #[test]
        fn test_derive_field_enum_values_attribute() {
            let input_fields = EnumAnnotated::input_fields();
            assert_eq!(input_fields.len(), 1);
            match &input_fields[0].field_type {
                FieldType::Enum(values) => {
                    assert_eq!(
                        values,
                        &vec!["low".to_string(), "medium".to_string(), "high".to_string(),]
                    );
                }
                other => panic!("expected enum field type, got {:?}", other),
            }

            let output_fields = EnumAnnotated::output_fields();
            assert_eq!(output_fields.len(), 1);
            assert!(matches!(output_fields[0].field_type, FieldType::Enum(_)));
        }

        #[test]
        fn test_derive_field_enum_values_validation_in_from_response() {
            let valid = r#"{"category":"bug"}"#;
            assert!(EnumAnnotated::from_response(valid).is_ok());

            let invalid = r#"{"category":"other"}"#;
            assert!(EnumAnnotated::from_response(invalid).is_err());
        }
    }
}
