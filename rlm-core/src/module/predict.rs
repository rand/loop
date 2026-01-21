//! Predict wrapper for executing signatures with LLMs.
//!
//! The `Predict` struct wraps a signature and handles:
//! - Prompt generation from signature and inputs
//! - LLM invocation
//! - Output parsing and validation
//! - Few-shot demonstration injection

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;

use super::example::ErasedDemonstration;
use super::{Module, ModuleConfig, Predictor};
use crate::error::{Error, Result};
use crate::llm::{ChatMessage, CompletionRequest, LLMClient};
use crate::signature::Signature;

/// Configuration for a Predict module.
#[derive(Debug, Clone)]
pub struct PredictConfig {
    /// Base module configuration.
    pub module: ModuleConfig,
    /// Model to use (overrides default if set).
    pub model: Option<String>,
    /// Whether to include chain-of-thought reasoning.
    pub chain_of_thought: bool,
}

impl Default for PredictConfig {
    fn default() -> Self {
        Self {
            module: ModuleConfig::default(),
            model: None,
            chain_of_thought: false,
        }
    }
}

impl PredictConfig {
    /// Create a new configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Enable chain-of-thought reasoning.
    pub fn with_chain_of_thought(mut self) -> Self {
        self.chain_of_thought = true;
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.module.temperature = temp;
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.module.max_tokens = Some(tokens);
        self
    }
}

/// A module that predicts outputs for a given signature.
///
/// `Predict` is the fundamental building block for LLM-based modules.
/// It handles prompt generation, LLM calls, and output parsing.
///
/// # Type Parameters
///
/// - `S`: The signature to implement
///
/// # Example
///
/// ```ignore
/// use rlm_core::module::Predict;
///
/// let predictor = Predict::<MySignature>::new()
///     .with_config(PredictConfig::new().with_temperature(0.7));
///
/// let outputs = predictor.forward(inputs).await?;
/// ```
pub struct Predict<S: Signature> {
    _phantom: PhantomData<S>,
    lm: Arc<RwLock<Option<Arc<dyn LLMClient>>>>,
    config: PredictConfig,
    demonstrations: Arc<RwLock<Vec<ErasedDemonstration>>>,
    name: String,
}

impl<S: Signature> Predict<S> {
    /// Create a new Predict module.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
            lm: Arc::new(RwLock::new(None)),
            config: PredictConfig::default(),
            demonstrations: Arc::new(RwLock::new(Vec::new())),
            name: format!("Predict<{}>", std::any::type_name::<S>()),
        }
    }

    /// Create with a language model.
    pub fn with_lm(lm: Arc<dyn LLMClient>) -> Self {
        Self {
            _phantom: PhantomData,
            lm: Arc::new(RwLock::new(Some(lm))),
            config: PredictConfig::default(),
            demonstrations: Arc::new(RwLock::new(Vec::new())),
            name: format!("Predict<{}>", std::any::type_name::<S>()),
        }
    }

    /// Create with configuration.
    pub fn with_config(mut self, config: PredictConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the module name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add a typed demonstration.
    pub async fn add_typed_demonstration(
        &self,
        inputs: S::Inputs,
        outputs: S::Outputs,
    ) -> Result<()> {
        let erased = ErasedDemonstration::new(
            serde_json::to_value(&inputs)?,
            serde_json::to_value(&outputs)?,
        );
        self.demonstrations.write().await.push(erased);
        Ok(())
    }

    /// Build the prompt for the LLM.
    async fn build_prompt(&self, inputs: &S::Inputs) -> Result<Vec<ChatMessage>> {
        let mut messages = Vec::new();

        // Build system message with instructions and field descriptions
        let system_content = self.build_system_prompt();
        messages.push(ChatMessage::system(system_content));

        // Add demonstrations if enabled
        if self.config.module.use_demonstrations {
            let demos = self.demonstrations.read().await;
            for demo in demos.iter() {
                // User message with demo inputs
                let demo_input = format_inputs_for_prompt(&demo.inputs);
                messages.push(ChatMessage::user(demo_input));

                // Assistant message with demo outputs (and reasoning if available)
                let mut demo_output = String::new();
                if let Some(ref reasoning) = demo.reasoning {
                    demo_output.push_str("Reasoning: ");
                    demo_output.push_str(reasoning);
                    demo_output.push_str("\n\n");
                }
                demo_output.push_str(&format_outputs_for_prompt(&demo.outputs));
                messages.push(ChatMessage::assistant(demo_output));
            }
        }

        // Add the actual input
        let input_value = serde_json::to_value(inputs)?;
        let user_content = format_inputs_for_prompt(&input_value);
        messages.push(ChatMessage::user(user_content));

        Ok(messages)
    }

    /// Build the system prompt from the signature.
    fn build_system_prompt(&self) -> String {
        let mut prompt = String::new();

        // Add instructions
        prompt.push_str(S::instructions());
        prompt.push_str("\n\n");

        // Add input field descriptions
        prompt.push_str("## Input Fields\n\n");
        for field in S::input_fields() {
            let prefix = field.prefix.as_deref().unwrap_or(&field.name);
            prompt.push_str(&format!(
                "- **{}**: {} ({})\n",
                prefix,
                field.description,
                field.field_type.to_prompt_hint()
            ));
        }
        prompt.push('\n');

        // Add output field descriptions
        prompt.push_str("## Output Fields\n\n");
        prompt.push_str("Respond with a JSON object containing these fields:\n\n");
        for field in S::output_fields() {
            let prefix = field.prefix.as_deref().unwrap_or(&field.name);
            let required = if field.required { "required" } else { "optional" };
            prompt.push_str(&format!(
                "- **{}**: {} ({}, {})\n",
                prefix,
                field.description,
                field.field_type.to_prompt_hint(),
                required
            ));
        }

        if self.config.chain_of_thought {
            prompt.push_str("\nFirst explain your reasoning step by step, then provide the JSON output.\n");
        } else {
            prompt.push_str("\nRespond with only the JSON object, no additional text.\n");
        }

        prompt
    }

    /// Parse the LLM response into outputs.
    fn parse_response(&self, response: &str) -> Result<S::Outputs> {
        S::from_response(response).map_err(|e| Error::Internal(format!("Failed to parse response: {}", e)))
    }
}

impl<S: Signature> Default for Predict<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S: Signature + 'static> Module for Predict<S> {
    type Sig = S;

    async fn forward(&self, inputs: S::Inputs) -> Result<S::Outputs> {
        // Get the LM
        let lm_guard = self.lm.read().await;
        let lm = lm_guard.as_ref().ok_or_else(|| {
            Error::Config("No language model set for Predict module".to_string())
        })?;

        // Build the prompt
        let messages = self.build_prompt(&inputs).await?;

        // Create completion request
        let request = CompletionRequest {
            model: self.config.model.clone(),
            system: None, // System is in messages
            messages,
            max_tokens: self.config.module.max_tokens,
            temperature: Some(self.config.module.temperature),
            stop: None,
            enable_caching: true,
            metadata: None,
        };

        // Call LLM with retries
        let mut last_error = None;
        for attempt in 0..=self.config.module.max_retries {
            match lm.complete(request.clone()).await {
                Ok(response) => {
                    // Parse the response
                    match self.parse_response(&response.content) {
                        Ok(outputs) => return Ok(outputs),
                        Err(e) if attempt < self.config.module.max_retries => {
                            last_error = Some(e);
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) if attempt < self.config.module.max_retries => {
                    last_error = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("Unexpected retry loop exit".to_string())))
    }

    fn predictors(&self) -> Vec<&dyn Predictor> {
        vec![self]
    }

    fn set_lm(&mut self, lm: Arc<dyn LLMClient>) {
        // Use try_write to avoid blocking; if it fails, the LM is being used
        if let Ok(mut guard) = self.lm.try_write() {
            *guard = Some(lm);
        }
    }

    fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
        self.lm.try_read().ok().and_then(|g| g.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl<S: Signature + 'static> Predictor for Predict<S> {
    fn add_demonstration(&mut self, inputs: Value, outputs: Value) {
        let demo = ErasedDemonstration::new(inputs, outputs);
        // Use blocking lock since this is called from non-async context
        if let Ok(mut guard) = self.demonstrations.try_write() {
            guard.push(demo);
        }
    }

    fn clear_demonstrations(&mut self) {
        if let Ok(mut guard) = self.demonstrations.try_write() {
            guard.clear();
        }
    }

    fn demonstration_count(&self) -> usize {
        self.demonstrations
            .try_read()
            .map(|g| g.len())
            .unwrap_or(0)
    }

    fn predictor_name(&self) -> &str {
        &self.name
    }
}

/// Format inputs as a prompt string.
fn format_inputs_for_prompt(inputs: &Value) -> String {
    match inputs {
        Value::Object(map) => {
            let mut parts = Vec::new();
            for (key, value) in map {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                parts.push(format!("{}: {}", key, value_str));
            }
            parts.join("\n")
        }
        other => other.to_string(),
    }
}

/// Format outputs as a prompt string.
fn format_outputs_for_prompt(outputs: &Value) -> String {
    serde_json::to_string_pretty(outputs).unwrap_or_else(|_| outputs.to_string())
}

// Implement Clone manually since we use Arc<RwLock>
impl<S: Signature> Clone for Predict<S> {
    fn clone(&self) -> Self {
        Self {
            _phantom: PhantomData,
            lm: self.lm.clone(),
            config: self.config.clone(),
            demonstrations: self.demonstrations.clone(),
            name: self.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::{FieldSpec, FieldType};
    use serde::{Deserialize, Serialize};

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

    impl Signature for MockSignature {
        type Inputs = MockInputs;
        type Outputs = MockOutputs;

        fn instructions() -> &'static str {
            "Process the input text and produce a result."
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("text", FieldType::String).with_description("Input text")]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("result", FieldType::String).with_description("Output result")]
        }
    }

    #[test]
    fn test_predict_creation() {
        let predict = Predict::<MockSignature>::new();
        assert!(predict.get_lm().is_none());
        assert_eq!(predict.demonstration_count(), 0);
    }

    #[test]
    fn test_predict_config() {
        let config = PredictConfig::new()
            .with_model("claude-3-opus")
            .with_temperature(0.5)
            .with_chain_of_thought();

        assert_eq!(config.model, Some("claude-3-opus".to_string()));
        assert_eq!(config.module.temperature, 0.5);
        assert!(config.chain_of_thought);
    }

    #[test]
    fn test_system_prompt_generation() {
        let predict = Predict::<MockSignature>::new();
        let prompt = predict.build_system_prompt();

        assert!(prompt.contains("Process the input text"));
        assert!(prompt.contains("Input Fields"));
        assert!(prompt.contains("Output Fields"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_format_inputs() {
        let inputs = serde_json::json!({
            "text": "Hello world",
            "count": 42
        });

        let formatted = format_inputs_for_prompt(&inputs);
        assert!(formatted.contains("text: Hello world"));
        assert!(formatted.contains("count: 42"));
    }
}
