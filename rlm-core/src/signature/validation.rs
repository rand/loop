//! Validation for typed signatures.
//!
//! This module provides validation errors and functions for ensuring
//! inputs and outputs conform to their signature specifications.

use super::types::{FieldSpec, FieldType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Error that occurs during signature validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum ValidationError {
    /// A required field is missing.
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

    /// A custom constraint was violated.
    ConstraintViolated {
        /// Name of the field
        field: String,
        /// Description of the constraint that was violated
        constraint: String,
    },

    /// Nested object validation failed.
    NestedError {
        /// Path to the nested field (e.g., "user.address.city")
        path: String,
        /// The underlying validation error
        error: Box<ValidationError>,
    },

    /// Custom validation error.
    Custom(String),
}

impl ValidationError {
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
        value: &Value,
    ) -> Self {
        let got = value_type_name(value);
        let value_preview = truncate_preview(&value.to_string(), 100);
        Self::TypeMismatch {
            field: field.into(),
            expected,
            got,
            value_preview,
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

    /// Create a constraint violated error.
    pub fn constraint_violated(field: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::ConstraintViolated {
            field: field.into(),
            constraint: constraint.into(),
        }
    }

    /// Wrap this error with a path prefix for nested fields.
    pub fn with_path(self, parent: impl Into<String>) -> Self {
        let parent = parent.into();
        match self {
            Self::NestedError { path, error } => Self::NestedError {
                path: format!("{}.{}", parent, path),
                error,
            },
            other => Self::NestedError {
                path: parent,
                error: Box::new(other),
            },
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
                    "Field '{}' has invalid value '{}'. Allowed values: {}",
                    field,
                    value,
                    allowed.join(", ")
                )
            }
            Self::ConstraintViolated { field, constraint } => {
                format!("Field '{}' violates constraint: {}", field, constraint)
            }
            Self::NestedError { path, error } => {
                format!("At '{}': {}", path, error.to_user_message())
            }
            Self::Custom(msg) => msg.clone(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_user_message())
    }
}

impl std::error::Error for ValidationError {}

/// Result of validating a value against a field spec.
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// Validate a JSON value against a list of field specifications.
///
/// # Arguments
///
/// * `value` - The JSON object to validate
/// * `fields` - The field specifications to validate against
///
/// # Returns
///
/// * `Ok(())` if all validations pass
/// * `Err(Vec<ValidationError>)` with all validation errors
///
/// # Example
///
/// ```
/// use rlm_core::signature::{validate_fields, FieldSpec, FieldType};
/// use serde_json::json;
///
/// let fields = vec![
///     FieldSpec::new("name", FieldType::String),
///     FieldSpec::new("age", FieldType::Integer).optional(),
/// ];
///
/// let value = json!({"name": "Alice", "age": 30});
/// assert!(validate_fields(&value, &fields).is_ok());
///
/// let missing = json!({"age": 30});
/// assert!(validate_fields(&missing, &fields).is_err());
/// ```
pub fn validate_fields(value: &Value, fields: &[FieldSpec]) -> ValidationResult {
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => {
            return Err(vec![ValidationError::Custom(
                "Expected an object".to_string(),
            )]);
        }
    };

    let mut errors = Vec::new();

    for field in fields {
        match obj.get(&field.name) {
            Some(field_value) => {
                // Validate the field type
                if let Err(e) = validate_value(field_value, &field.field_type, &field.name) {
                    errors.extend(e);
                }
            }
            None => {
                if field.required {
                    errors.push(ValidationError::missing_field(
                        &field.name,
                        field.field_type.clone(),
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a single value against a field type.
pub fn validate_value(value: &Value, field_type: &FieldType, field_name: &str) -> ValidationResult {
    let mut errors = Vec::new();

    match field_type {
        FieldType::String => {
            if !value.is_string() {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    FieldType::String,
                    value,
                ));
            }
        }
        FieldType::Integer => {
            if let Some(n) = value.as_number() {
                if !n.is_i64() && !n.is_u64() {
                    errors.push(ValidationError::type_mismatch(
                        field_name,
                        FieldType::Integer,
                        value,
                    ));
                }
            } else {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    FieldType::Integer,
                    value,
                ));
            }
        }
        FieldType::Float => {
            if !value.is_number() {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    FieldType::Float,
                    value,
                ));
            }
        }
        FieldType::Boolean => {
            if !value.is_boolean() {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    FieldType::Boolean,
                    value,
                ));
            }
        }
        FieldType::List(inner) => {
            if let Some(arr) = value.as_array() {
                for (i, item) in arr.iter().enumerate() {
                    let item_path = format!("{}[{}]", field_name, i);
                    if let Err(e) = validate_value(item, inner, &item_path) {
                        errors.extend(e);
                    }
                }
            } else {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    field_type.clone(),
                    value,
                ));
            }
        }
        FieldType::Object(fields) => {
            if value.is_object() {
                if let Err(e) = validate_fields(value, fields) {
                    for err in e {
                        errors.push(err.with_path(field_name));
                    }
                }
            } else {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    field_type.clone(),
                    value,
                ));
            }
        }
        FieldType::Enum(allowed) => {
            if let Some(s) = value.as_str() {
                if !allowed.contains(&s.to_string()) {
                    errors.push(ValidationError::enum_invalid(
                        field_name,
                        s,
                        allowed.clone(),
                    ));
                }
            } else {
                errors.push(ValidationError::type_mismatch(
                    field_name,
                    field_type.clone(),
                    value,
                ));
            }
        }
        FieldType::Custom(_) => {
            // Custom types pass validation - they rely on external validation
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Apply default values to missing optional fields.
///
/// Returns a new JSON object with defaults applied.
pub fn apply_defaults(value: &Value, fields: &[FieldSpec]) -> Value {
    let mut obj = match value.as_object() {
        Some(obj) => obj.clone(),
        None => return value.clone(),
    };

    for field in fields {
        if !obj.contains_key(&field.name) {
            if let Some(default) = &field.default {
                obj.insert(field.name.clone(), default.clone());
            }
        }
    }

    Value::Object(obj)
}

/// Get a human-readable type name for a JSON value.
fn value_type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer".to_string()
            } else {
                "number".to_string()
            }
        }
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

/// Truncate a string for preview purposes.
fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_fields_success() {
        let fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("age", FieldType::Integer),
        ];

        let value = json!({"name": "Alice", "age": 30});
        assert!(validate_fields(&value, &fields).is_ok());
    }

    #[test]
    fn test_validate_fields_missing_required() {
        let fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("age", FieldType::Integer),
        ];

        let value = json!({"name": "Alice"});
        let result = validate_fields(&value, &fields);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::MissingField { .. }));
    }

    #[test]
    fn test_validate_fields_optional_missing() {
        let fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("age", FieldType::Integer).optional(),
        ];

        let value = json!({"name": "Alice"});
        assert!(validate_fields(&value, &fields).is_ok());
    }

    #[test]
    fn test_validate_fields_type_mismatch() {
        let fields = vec![FieldSpec::new("age", FieldType::Integer)];

        let value = json!({"age": "thirty"});
        let result = validate_fields(&value, &fields);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::TypeMismatch { .. }));
    }

    #[test]
    fn test_validate_enum() {
        let fields = vec![FieldSpec::new(
            "status",
            FieldType::enum_of(["active", "inactive"]),
        )];

        let valid = json!({"status": "active"});
        assert!(validate_fields(&valid, &fields).is_ok());

        let invalid = json!({"status": "unknown"});
        let result = validate_fields(&invalid, &fields);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err()[0],
            ValidationError::EnumInvalid { .. }
        ));
    }

    #[test]
    fn test_validate_list() {
        let fields = vec![FieldSpec::new("items", FieldType::list(FieldType::String))];

        let valid = json!({"items": ["a", "b", "c"]});
        assert!(validate_fields(&valid, &fields).is_ok());

        let invalid = json!({"items": ["a", 1, "c"]});
        let result = validate_fields(&invalid, &fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_nested_object() {
        let address_fields = vec![
            FieldSpec::new("city", FieldType::String),
            FieldSpec::new("zip", FieldType::String),
        ];
        let fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("address", FieldType::object(address_fields)),
        ];

        let valid = json!({
            "name": "Alice",
            "address": {"city": "NYC", "zip": "10001"}
        });
        assert!(validate_fields(&valid, &fields).is_ok());

        let invalid = json!({
            "name": "Alice",
            "address": {"city": "NYC"}
        });
        let result = validate_fields(&invalid, &fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_defaults() {
        let fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("count", FieldType::Integer).with_default(json!(10)),
        ];

        let value = json!({"name": "test"});
        let with_defaults = apply_defaults(&value, &fields);

        assert_eq!(with_defaults["name"], "test");
        assert_eq!(with_defaults["count"], 10);
    }

    #[test]
    fn test_error_user_message() {
        let missing = ValidationError::missing_field("name", FieldType::String);
        assert!(missing.to_user_message().contains("name"));
        assert!(missing.to_user_message().contains("string"));

        let enum_err = ValidationError::enum_invalid("status", "bad", vec!["a".into(), "b".into()]);
        assert!(enum_err.to_user_message().contains("bad"));
        assert!(enum_err.to_user_message().contains("a, b"));
    }

    #[test]
    fn test_nested_error_path() {
        let inner = ValidationError::missing_field("city", FieldType::String);
        let nested = inner.with_path("address");

        assert!(matches!(nested, ValidationError::NestedError { .. }));
        assert!(nested.to_user_message().contains("address"));
    }

    #[test]
    fn test_serialization() {
        let error = ValidationError::type_mismatch(
            "age",
            FieldType::Integer,
            &json!("not a number"),
        );

        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ValidationError = serde_json::from_str(&json).unwrap();

        assert_eq!(error, deserialized);
    }
}
