//! Example and Demonstration types for few-shot learning.
//!
//! This module provides types for representing training examples and
//! demonstrations that can be injected into prompts for few-shot learning.

use crate::signature::Signature;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A training example with inputs and expected outputs.
///
/// Examples are used for:
/// - Training data for optimizers like BootstrapFewShot
/// - Evaluation data for measuring module performance
/// - Labeled demonstrations for few-shot prompts
///
/// # Type Parameters
///
/// - `S`: The signature this example conforms to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example<S: Signature> {
    /// The input values for this example.
    pub inputs: S::Inputs,
    /// The expected output values (ground truth).
    pub outputs: S::Outputs,
    /// Optional metadata about this example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExampleMetadata>,
}

impl<S: Signature> Example<S> {
    /// Create a new example with inputs and outputs.
    pub fn new(inputs: S::Inputs, outputs: S::Outputs) -> Self {
        Self {
            inputs,
            outputs,
            metadata: None,
        }
    }

    /// Create an example with metadata.
    pub fn with_metadata(
        inputs: S::Inputs,
        outputs: S::Outputs,
        metadata: ExampleMetadata,
    ) -> Self {
        Self {
            inputs,
            outputs,
            metadata: Some(metadata),
        }
    }

    /// Add metadata to this example.
    pub fn set_metadata(mut self, metadata: ExampleMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Metadata about an example.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleMetadata {
    /// Source of this example (e.g., "manual", "bootstrapped", "synthetic").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Unique identifier for this example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Quality score if evaluated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<f64>,
}

impl ExampleMetadata {
    /// Create new metadata with a source.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: Some(source.into()),
            id: None,
            tags: Vec::new(),
            quality_score: None,
        }
    }

    /// Set the example ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the quality score.
    pub fn with_quality_score(mut self, score: f64) -> Self {
        self.quality_score = Some(score);
        self
    }
}

impl Default for ExampleMetadata {
    fn default() -> Self {
        Self {
            source: None,
            id: None,
            tags: Vec::new(),
            quality_score: None,
        }
    }
}

/// A demonstration is an example with optional reasoning trace.
///
/// Demonstrations are injected into prompts as few-shot examples.
/// They may include the reasoning trace that led to the output,
/// which can help the model understand the expected reasoning process.
///
/// # Type Parameters
///
/// - `S`: The signature this demonstration conforms to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demonstration<S: Signature> {
    /// The input values for this demonstration.
    pub inputs: S::Inputs,
    /// The output values produced.
    pub outputs: S::Outputs,
    /// Optional reasoning trace (chain of thought).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Metric score achieved by this demonstration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_score: Option<f64>,
}

impl<S: Signature> Demonstration<S> {
    /// Create a new demonstration from inputs and outputs.
    pub fn new(inputs: S::Inputs, outputs: S::Outputs) -> Self {
        Self {
            inputs,
            outputs,
            reasoning: None,
            metric_score: None,
        }
    }

    /// Create a demonstration with reasoning trace.
    pub fn with_reasoning(
        inputs: S::Inputs,
        outputs: S::Outputs,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            inputs,
            outputs,
            reasoning: Some(reasoning.into()),
            metric_score: None,
        }
    }

    /// Create from an example.
    pub fn from_example(example: Example<S>) -> Self {
        Self {
            inputs: example.inputs,
            outputs: example.outputs,
            reasoning: None,
            metric_score: example.metadata.and_then(|m| m.quality_score),
        }
    }

    /// Set the metric score.
    pub fn with_metric_score(mut self, score: f64) -> Self {
        self.metric_score = Some(score);
        self
    }

    /// Set the reasoning trace.
    pub fn set_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }
}

/// Type-erased demonstration for storage in predictors.
///
/// This allows predictors to store demonstrations without knowing
/// the specific signature type at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasedDemonstration {
    /// The input values as JSON.
    pub inputs: Value,
    /// The output values as JSON.
    pub outputs: Value,
    /// Optional reasoning trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Metric score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_score: Option<f64>,
}

impl ErasedDemonstration {
    /// Create a new erased demonstration.
    pub fn new(inputs: Value, outputs: Value) -> Self {
        Self {
            inputs,
            outputs,
            reasoning: None,
            metric_score: None,
        }
    }

    /// Create from a typed demonstration.
    pub fn from_typed<S: Signature>(demo: &Demonstration<S>) -> Self {
        Self {
            inputs: serde_json::to_value(&demo.inputs).unwrap_or(Value::Null),
            outputs: serde_json::to_value(&demo.outputs).unwrap_or(Value::Null),
            reasoning: demo.reasoning.clone(),
            metric_score: demo.metric_score,
        }
    }

    /// Set reasoning.
    pub fn with_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    /// Set metric score.
    pub fn with_metric_score(mut self, score: f64) -> Self {
        self.metric_score = Some(score);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock signature for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockInputs {
        text: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockOutputs {
        result: String,
    }

    struct MockSignature;

    impl crate::signature::Signature for MockSignature {
        type Inputs = MockInputs;
        type Outputs = MockOutputs;

        fn instructions() -> &'static str {
            "Test signature"
        }

        fn input_fields() -> Vec<crate::signature::FieldSpec> {
            vec![]
        }

        fn output_fields() -> Vec<crate::signature::FieldSpec> {
            vec![]
        }
    }

    #[test]
    fn test_example_creation() {
        let example = Example::<MockSignature>::new(
            MockInputs {
                text: "input".to_string(),
            },
            MockOutputs {
                result: "output".to_string(),
            },
        );

        assert_eq!(example.inputs.text, "input");
        assert_eq!(example.outputs.result, "output");
        assert!(example.metadata.is_none());
    }

    #[test]
    fn test_example_with_metadata() {
        let metadata = ExampleMetadata::new("manual")
            .with_id("ex-001")
            .with_tag("test")
            .with_quality_score(0.95);

        let example = Example::<MockSignature>::with_metadata(
            MockInputs {
                text: "input".to_string(),
            },
            MockOutputs {
                result: "output".to_string(),
            },
            metadata,
        );

        let meta = example.metadata.unwrap();
        assert_eq!(meta.source, Some("manual".to_string()));
        assert_eq!(meta.id, Some("ex-001".to_string()));
        assert_eq!(meta.tags, vec!["test"]);
        assert_eq!(meta.quality_score, Some(0.95));
    }

    #[test]
    fn test_demonstration_creation() {
        let demo = Demonstration::<MockSignature>::with_reasoning(
            MockInputs {
                text: "input".to_string(),
            },
            MockOutputs {
                result: "output".to_string(),
            },
            "First I analyzed the input, then I produced the output.",
        );

        assert!(demo.reasoning.is_some());
        assert_eq!(
            demo.reasoning.unwrap(),
            "First I analyzed the input, then I produced the output."
        );
    }

    #[test]
    fn test_erased_demonstration() {
        let erased =
            ErasedDemonstration::new(json!({"text": "input"}), json!({"result": "output"}))
                .with_reasoning("reasoning trace")
                .with_metric_score(0.9);

        assert_eq!(erased.inputs["text"], "input");
        assert_eq!(erased.outputs["result"], "output");
        assert_eq!(erased.reasoning, Some("reasoning trace".to_string()));
        assert_eq!(erased.metric_score, Some(0.9));
    }
}
