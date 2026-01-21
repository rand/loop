//! Type definitions for the typed signatures system.
//!
//! This module provides the core types for defining LLM I/O contracts:
//! - **FieldSpec**: Metadata for input and output fields
//! - **FieldType**: Type information for validation and prompt generation

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Specification for a field in a signature.
///
/// FieldSpec describes metadata about an input or output field including
/// its type, description (for prompt generation), and validation constraints.
///
/// # Example
///
/// ```
/// use rlm_core::signature::{FieldSpec, FieldType};
///
/// let field = FieldSpec::new("query", FieldType::String)
///     .with_description("The search query to execute")
///     .with_prefix("Query");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// Field name (matches struct field)
    pub name: String,
    /// Field type for validation
    pub field_type: FieldType,
    /// Human-readable description (for prompt generation)
    pub description: String,
    /// Optional display prefix/label
    pub prefix: Option<String>,
    /// Whether field is required
    pub required: bool,
    /// Default value (JSON) if not required
    pub default: Option<Value>,
}

impl FieldSpec {
    /// Create a new required field specification.
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            description: String::new(),
            prefix: None,
            required: true,
            default: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the display prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Mark the field as optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set a default value for optional fields.
    pub fn with_default(mut self, default: impl Into<Value>) -> Self {
        self.default = Some(default.into());
        self.required = false;
        self
    }

    /// Get the display label (prefix if set, otherwise name).
    pub fn display_label(&self) -> &str {
        self.prefix.as_deref().unwrap_or(&self.name)
    }

    /// Format the field for prompt generation.
    ///
    /// Returns a string like "Query (string): The search query to execute"
    pub fn to_prompt_line(&self) -> String {
        let type_hint = self.field_type.to_prompt_hint();
        let label = self.display_label();
        let required_marker = if self.required { "" } else { " (optional)" };

        if self.description.is_empty() {
            format!("{label} ({type_hint}){required_marker}")
        } else {
            format!("{label} ({type_hint}){required_marker}: {}", self.description)
        }
    }
}

/// Type of a field for validation and prompt generation.
///
/// FieldType represents the expected data type for a field, enabling:
/// - Validation of inputs and outputs
/// - Type hints in generated prompts
/// - Schema generation for LLM responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldType {
    /// String value
    String,
    /// Integer value (any size)
    Integer,
    /// Floating point value
    Float,
    /// Boolean value
    Boolean,
    /// List of items of a specific type
    List(Box<FieldType>),
    /// Nested object with fields
    Object(Vec<FieldSpec>),
    /// Enumeration with allowed values
    Enum(Vec<String>),
    /// Custom type (name only, validation deferred)
    Custom(String),
}

impl FieldType {
    /// Create a list type.
    pub fn list(inner: FieldType) -> Self {
        Self::List(Box::new(inner))
    }

    /// Create an object type with fields.
    pub fn object(fields: Vec<FieldSpec>) -> Self {
        Self::Object(fields)
    }

    /// Create an enum type with allowed values.
    pub fn enum_of(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Enum(values.into_iter().map(|v| v.into()).collect())
    }

    /// Create a custom type.
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }

    /// Get a hint string for prompts (e.g., "string", "list[string]").
    pub fn to_prompt_hint(&self) -> String {
        match self {
            Self::String => "string".to_string(),
            Self::Integer => "integer".to_string(),
            Self::Float => "number".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::List(inner) => format!("list[{}]", inner.to_prompt_hint()),
            Self::Object(_) => "object".to_string(),
            Self::Enum(values) => {
                if values.len() <= 5 {
                    values.join("|")
                } else {
                    format!("one of {} values", values.len())
                }
            }
            Self::Custom(name) => name.clone(),
        }
    }

    /// Check if this type is compatible with a JSON value.
    pub fn is_compatible(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::String, Value::String(_)) => true,
            (Self::Integer, Value::Number(n)) => n.is_i64() || n.is_u64(),
            (Self::Float, Value::Number(_)) => true,
            (Self::Boolean, Value::Bool(_)) => true,
            (Self::List(inner), Value::Array(arr)) => {
                arr.iter().all(|v| inner.is_compatible(v))
            }
            (Self::Object(fields), Value::Object(obj)) => {
                // All required fields must be present and compatible
                fields.iter().all(|f| {
                    if f.required {
                        obj.get(&f.name)
                            .map(|v| f.field_type.is_compatible(v))
                            .unwrap_or(false)
                    } else {
                        obj.get(&f.name)
                            .map(|v| f.field_type.is_compatible(v))
                            .unwrap_or(true)
                    }
                })
            }
            (Self::Enum(values), Value::String(s)) => values.contains(s),
            (Self::Custom(_), _) => true, // Custom types accept anything
            _ => false,
        }
    }

    /// Generate a JSON schema fragment for this type.
    pub fn to_json_schema(&self) -> Value {
        match self {
            Self::String => serde_json::json!({ "type": "string" }),
            Self::Integer => serde_json::json!({ "type": "integer" }),
            Self::Float => serde_json::json!({ "type": "number" }),
            Self::Boolean => serde_json::json!({ "type": "boolean" }),
            Self::List(inner) => serde_json::json!({
                "type": "array",
                "items": inner.to_json_schema()
            }),
            Self::Object(fields) => {
                let properties: serde_json::Map<String, Value> = fields
                    .iter()
                    .map(|f| (f.name.clone(), f.field_type.to_json_schema()))
                    .collect();
                let required: Vec<&str> = fields
                    .iter()
                    .filter(|f| f.required)
                    .map(|f| f.name.as_str())
                    .collect();
                serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                })
            }
            Self::Enum(values) => serde_json::json!({
                "type": "string",
                "enum": values
            }),
            Self::Custom(name) => serde_json::json!({
                "type": "object",
                "$ref": format!("#/definitions/{}", name)
            }),
        }
    }
}

impl Default for FieldType {
    fn default() -> Self {
        Self::String
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_spec_creation() {
        let field = FieldSpec::new("query", FieldType::String);
        assert_eq!(field.name, "query");
        assert!(field.required);
        assert!(field.prefix.is_none());
    }

    #[test]
    fn test_field_spec_builder() {
        let field = FieldSpec::new("severity", FieldType::enum_of(["low", "medium", "high"]))
            .with_description("The severity level")
            .with_prefix("Severity")
            .optional();

        assert_eq!(field.name, "severity");
        assert_eq!(field.description, "The severity level");
        assert_eq!(field.prefix, Some("Severity".to_string()));
        assert!(!field.required);
    }

    #[test]
    fn test_field_spec_with_default() {
        let field = FieldSpec::new("count", FieldType::Integer)
            .with_default(serde_json::json!(10));

        assert!(!field.required);
        assert_eq!(field.default, Some(serde_json::json!(10)));
    }

    #[test]
    fn test_display_label() {
        let with_prefix = FieldSpec::new("user_query", FieldType::String)
            .with_prefix("Query");
        let without_prefix = FieldSpec::new("query", FieldType::String);

        assert_eq!(with_prefix.display_label(), "Query");
        assert_eq!(without_prefix.display_label(), "query");
    }

    #[test]
    fn test_to_prompt_line() {
        let field = FieldSpec::new("query", FieldType::String)
            .with_description("The search query")
            .with_prefix("Query");

        assert_eq!(field.to_prompt_line(), "Query (string): The search query");

        let optional = FieldSpec::new("limit", FieldType::Integer)
            .with_description("Max results")
            .optional();

        assert_eq!(optional.to_prompt_line(), "limit (integer) (optional): Max results");
    }

    #[test]
    fn test_field_type_prompt_hints() {
        assert_eq!(FieldType::String.to_prompt_hint(), "string");
        assert_eq!(FieldType::Integer.to_prompt_hint(), "integer");
        assert_eq!(FieldType::Float.to_prompt_hint(), "number");
        assert_eq!(FieldType::Boolean.to_prompt_hint(), "boolean");
        assert_eq!(
            FieldType::list(FieldType::String).to_prompt_hint(),
            "list[string]"
        );
        assert_eq!(
            FieldType::enum_of(["a", "b", "c"]).to_prompt_hint(),
            "a|b|c"
        );
    }

    #[test]
    fn test_field_type_compatibility() {
        assert!(FieldType::String.is_compatible(&serde_json::json!("hello")));
        assert!(!FieldType::String.is_compatible(&serde_json::json!(42)));

        assert!(FieldType::Integer.is_compatible(&serde_json::json!(42)));
        assert!(!FieldType::Integer.is_compatible(&serde_json::json!(3.14)));

        assert!(FieldType::Float.is_compatible(&serde_json::json!(3.14)));
        assert!(FieldType::Float.is_compatible(&serde_json::json!(42)));

        assert!(FieldType::Boolean.is_compatible(&serde_json::json!(true)));
        assert!(!FieldType::Boolean.is_compatible(&serde_json::json!("true")));

        let list_type = FieldType::list(FieldType::String);
        assert!(list_type.is_compatible(&serde_json::json!(["a", "b", "c"])));
        assert!(!list_type.is_compatible(&serde_json::json!([1, 2, 3])));

        let enum_type = FieldType::enum_of(["low", "medium", "high"]);
        assert!(enum_type.is_compatible(&serde_json::json!("low")));
        assert!(!enum_type.is_compatible(&serde_json::json!("invalid")));
    }

    #[test]
    fn test_field_type_json_schema() {
        let schema = FieldType::String.to_json_schema();
        assert_eq!(schema["type"], "string");

        let list_schema = FieldType::list(FieldType::Integer).to_json_schema();
        assert_eq!(list_schema["type"], "array");
        assert_eq!(list_schema["items"]["type"], "integer");

        let enum_schema = FieldType::enum_of(["a", "b"]).to_json_schema();
        assert_eq!(enum_schema["type"], "string");
        assert_eq!(enum_schema["enum"], serde_json::json!(["a", "b"]));
    }

    #[test]
    fn test_nested_object_compatibility() {
        let inner_fields = vec![
            FieldSpec::new("name", FieldType::String),
            FieldSpec::new("age", FieldType::Integer).optional(),
        ];
        let obj_type = FieldType::object(inner_fields);

        // Valid: has required field
        assert!(obj_type.is_compatible(&serde_json::json!({"name": "Alice"})));

        // Valid: has both fields
        assert!(obj_type.is_compatible(&serde_json::json!({"name": "Bob", "age": 30})));

        // Invalid: missing required field
        assert!(!obj_type.is_compatible(&serde_json::json!({"age": 30})));

        // Invalid: wrong type for field
        assert!(!obj_type.is_compatible(&serde_json::json!({"name": 123})));
    }

    #[test]
    fn test_serialization() {
        let field = FieldSpec::new("items", FieldType::list(FieldType::String))
            .with_description("List of items");

        let json = serde_json::to_string(&field).unwrap();
        let deserialized: FieldSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(field, deserialized);
    }
}
