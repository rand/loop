//! SUBMIT mechanism types for structured output termination.
//!
//! This module provides the types for the SUBMIT() function that allows
//! LLM-generated code to return validated, structured outputs from the REPL.
//!
//! # Overview
//!
//! The SUBMIT mechanism works as follows:
//! 1. A signature is registered with the REPL before execution
//! 2. LLM-generated code calls `SUBMIT(outputs)` when done
//! 3. Outputs are validated against the registered signature
//! 4. Results are returned to the Rust orchestrator
//!
//! # Example
//!
//! ```python
//! # In REPL sandbox
//! result = analyze_code(code)
//! SUBMIT({
//!     "vulnerabilities": result.findings,
//!     "severity": "high"
//! })
//! ```

use super::types::{FieldSpec, FieldType};
use super::validation::ValidationError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Result of a SUBMIT call from the REPL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubmitResult {
    /// Successful submission with validated outputs.
    Success {
        /// The validated output values
        outputs: Value,
        /// Execution metrics
        #[serde(skip_serializing_if = "Option::is_none")]
        metrics: Option<SubmitMetrics>,
    },

    /// Validation failed for the submitted outputs.
    ValidationError {
        /// List of validation errors
        errors: Vec<SubmitError>,
        /// The original (invalid) outputs for debugging
        #[serde(skip_serializing_if = "Option::is_none")]
        original_outputs: Option<Value>,
    },

    /// No SUBMIT was called (execution completed without submitting).
    NotSubmitted {
        /// Reason why no submit occurred
        reason: String,
    },
}

impl SubmitResult {
    /// Create a successful submit result.
    pub fn success(outputs: Value) -> Self {
        Self::Success {
            outputs,
            metrics: None,
        }
    }

    /// Create a successful submit result with metrics.
    pub fn success_with_metrics(outputs: Value, metrics: SubmitMetrics) -> Self {
        Self::Success {
            outputs,
            metrics: Some(metrics),
        }
    }

    /// Create a validation error result.
    pub fn validation_error(errors: Vec<SubmitError>) -> Self {
        Self::ValidationError {
            errors,
            original_outputs: None,
        }
    }

    /// Create a validation error result with original outputs.
    pub fn validation_error_with_outputs(errors: Vec<SubmitError>, outputs: Value) -> Self {
        Self::ValidationError {
            errors,
            original_outputs: Some(outputs),
        }
    }

    /// Create a not-submitted result.
    pub fn not_submitted(reason: impl Into<String>) -> Self {
        Self::NotSubmitted {
            reason: reason.into(),
        }
    }

    /// Check if submission was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Get the outputs if successful.
    pub fn outputs(&self) -> Option<&Value> {
        match self {
            Self::Success { outputs, .. } => Some(outputs),
            _ => None,
        }
    }

    /// Get validation errors if any.
    pub fn errors(&self) -> Option<&[SubmitError]> {
        match self {
            Self::ValidationError { errors, .. } => Some(errors),
            _ => None,
        }
    }
}

/// Metrics from a successful SUBMIT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitMetrics {
    /// Number of iterations before SUBMIT
    pub iterations: u32,
    /// Total execution time in milliseconds
    pub execution_time_ms: f64,
    /// Number of LLM calls made during execution
    pub llm_calls: u32,
}

/// Error that occurs during SUBMIT validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum SubmitError {
    /// A required field is missing from the output.
    MissingField {
        /// Name of the missing field
        field: String,
        /// Expected type of the field
        expected_type: FieldType,
    },

    /// Field value has the wrong type.
    TypeMismatch {
        /// Name of the field
        field: String,
        /// Expected type
        expected: FieldType,
        /// Actual type description
        got: String,
        /// Preview of the actual value (first 100 chars)
        value_preview: String,
    },

    /// Enum field has an invalid value.
    EnumInvalid {
        /// Name of the field
        field: String,
        /// The invalid value that was provided
        value: String,
        /// List of allowed values
        allowed: Vec<String>,
    },

    /// A validation constraint was violated.
    ValidationFailed {
        /// Name of the field
        field: String,
        /// Description of why validation failed
        reason: String,
    },

    /// No signature was registered before SUBMIT.
    NoSignatureRegistered,

    /// SUBMIT was called multiple times (only first is processed).
    MultipleSubmits {
        /// Number of SUBMIT calls
        count: u32,
    },
}

impl SubmitError {
    /// Create a missing field error.
    pub fn missing_field(field: impl Into<String>, expected_type: FieldType) -> Self {
        Self::MissingField {
            field: field.into(),
            expected_type,
        }
    }

    /// Create a type mismatch error.
    pub fn type_mismatch(
        field: impl Into<String>,
        expected: FieldType,
        got: impl Into<String>,
        value_preview: impl Into<String>,
    ) -> Self {
        Self::TypeMismatch {
            field: field.into(),
            expected,
            got: got.into(),
            value_preview: value_preview.into(),
        }
    }

    /// Create an enum invalid error.
    pub fn enum_invalid(
        field: impl Into<String>,
        value: impl Into<String>,
        allowed: Vec<String>,
    ) -> Self {
        Self::EnumInvalid {
            field: field.into(),
            value: value.into(),
            allowed,
        }
    }

    /// Create a validation failed error.
    pub fn validation_failed(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ValidationFailed {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Get a human-readable error message.
    pub fn to_user_message(&self) -> String {
        match self {
            Self::MissingField { field, expected_type } => {
                format!(
                    "Missing required field '{}' (expected {})",
                    field,
                    expected_type.to_prompt_hint()
                )
            }
            Self::TypeMismatch {
                field,
                expected,
                got,
                value_preview,
            } => {
                format!(
                    "Field '{}' has wrong type: expected {}, got {} (value: {})",
                    field,
                    expected.to_prompt_hint(),
                    got,
                    value_preview
                )
            }
            Self::EnumInvalid {
                field,
                value,
                allowed,
            } => {
                format!(
                    "Field '{}' has invalid value '{}'. Allowed: {}",
                    field,
                    value,
                    allowed.join(", ")
                )
            }
            Self::ValidationFailed { field, reason } => {
                format!("Field '{}' validation failed: {}", field, reason)
            }
            Self::NoSignatureRegistered => {
                "SUBMIT called but no signature was registered".to_string()
            }
            Self::MultipleSubmits { count } => {
                format!(
                    "SUBMIT was called {} times. Only the first call is processed.",
                    count
                )
            }
        }
    }
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_user_message())
    }
}

impl std::error::Error for SubmitError {}

/// Convert ValidationError to SubmitError.
impl From<ValidationError> for SubmitError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::MissingField { field, expected_type } => {
                Self::MissingField { field, expected_type }
            }
            ValidationError::TypeMismatch {
                field,
                expected,
                got,
                value_preview,
            } => Self::TypeMismatch {
                field,
                expected,
                got,
                value_preview,
            },
            ValidationError::EnumInvalid {
                field,
                value,
                allowed,
            } => Self::EnumInvalid {
                field,
                value,
                allowed,
            },
            ValidationError::ConstraintViolated { field, constraint } => {
                Self::ValidationFailed {
                    field,
                    reason: constraint,
                }
            }
            ValidationError::NestedError { path, error } => {
                // Flatten nested errors by prefixing the path
                match *error {
                    ValidationError::MissingField { field, expected_type } => {
                        Self::MissingField {
                            field: format!("{}.{}", path, field),
                            expected_type,
                        }
                    }
                    other => Self::from(other),
                }
            }
            ValidationError::Custom(msg) => Self::ValidationFailed {
                field: "".to_string(),
                reason: msg,
            },
        }
    }
}

/// Specification for registering a signature with the REPL.
///
/// This is sent to the REPL before execution to enable SUBMIT validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRegistration {
    /// Output field specifications for validation
    pub output_fields: Vec<FieldSpec>,
    /// Optional signature name for error messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_name: Option<String>,
}

impl SignatureRegistration {
    /// Create a new signature registration.
    pub fn new(output_fields: Vec<FieldSpec>) -> Self {
        Self {
            output_fields,
            signature_name: None,
        }
    }

    /// Create with a signature name.
    pub fn with_name(output_fields: Vec<FieldSpec>, name: impl Into<String>) -> Self {
        Self {
            output_fields,
            signature_name: Some(name.into()),
        }
    }

    /// Convert to JSON-RPC params format.
    pub fn to_params(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_result_success() {
        let result = SubmitResult::success(serde_json::json!({"answer": "test"}));
        assert!(result.is_success());
        assert!(result.outputs().is_some());
    }

    #[test]
    fn test_submit_result_validation_error() {
        let errors = vec![SubmitError::missing_field("name", FieldType::String)];
        let result = SubmitResult::validation_error(errors);
        assert!(!result.is_success());
        assert!(result.errors().is_some());
    }

    #[test]
    fn test_submit_error_messages() {
        let missing = SubmitError::missing_field("name", FieldType::String);
        assert!(missing.to_user_message().contains("name"));
        assert!(missing.to_user_message().contains("string"));

        let type_mismatch = SubmitError::type_mismatch(
            "age",
            FieldType::Integer,
            "string",
            "\"twenty\"",
        );
        assert!(type_mismatch.to_user_message().contains("age"));
        assert!(type_mismatch.to_user_message().contains("integer"));

        let enum_invalid = SubmitError::enum_invalid(
            "status",
            "invalid",
            vec!["active".into(), "inactive".into()],
        );
        assert!(enum_invalid.to_user_message().contains("invalid"));
        assert!(enum_invalid.to_user_message().contains("active"));
    }

    #[test]
    fn test_submit_error_from_validation_error() {
        let validation_err = ValidationError::MissingField {
            field: "test".into(),
            expected_type: FieldType::String,
        };
        let submit_err: SubmitError = validation_err.into();
        assert!(matches!(submit_err, SubmitError::MissingField { .. }));
    }

    #[test]
    fn test_signature_registration() {
        let fields = vec![
            FieldSpec::new("answer", FieldType::String),
            FieldSpec::new("confidence", FieldType::Float),
        ];
        let reg = SignatureRegistration::with_name(fields, "TestSignature");

        assert_eq!(reg.signature_name, Some("TestSignature".to_string()));
        assert_eq!(reg.output_fields.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let result = SubmitResult::success(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));

        let parsed: SubmitResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_success());
    }
}
