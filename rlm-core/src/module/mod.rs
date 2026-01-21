//! DSPy-style module composition system.
//!
//! This module provides the foundation for building composable LLM pipelines
//! with typed signatures, similar to DSPy's module system.
//!
//! # Overview
//!
//! The module system enables:
//! - Type-safe composition of LLM operations
//! - Automatic prompt generation from signatures
//! - Few-shot demonstration injection
//! - LM propagation through module hierarchies
//!
//! # Example
//!
//! ```ignore
//! use rlm_core::module::{Module, Predict};
//! use rlm_core::signature::Signature;
//!
//! // Define signatures
//! #[derive(Signature)]
//! #[signature(instructions = "Extract entities from text")]
//! struct ExtractEntities {
//!     #[input(desc = "Text to analyze")]
//!     text: String,
//!     #[output(desc = "Extracted entities")]
//!     entities: Vec<String>,
//! }
//!
//! // Create a module
//! let extractor = Predict::<ExtractEntities>::new();
//!
//! // Execute with inputs
//! let outputs = extractor.forward(ExtractEntitiesInputs {
//!     text: "John works at Google".to_string(),
//! }).await?;
//! ```

mod compose;
mod example;
mod predict;

pub use compose::{chain_direct, Chain, ChainSignature, ParallelSignature, ParallelVec};
pub use example::{Demonstration, ErasedDemonstration, Example, ExampleMetadata};
pub use predict::{Predict, PredictConfig};

use crate::error::Result;
use crate::llm::LLMClient;
use crate::signature::Signature;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for modules that can be composed into pipelines.
///
/// A module wraps a signature and provides the `forward` method for execution.
/// Modules can be nested and composed, with LMs propagated to all sub-modules.
///
/// # Associated Types
///
/// - `Sig`: The signature this module implements
///
/// # Required Methods
///
/// - `forward`: Execute the module with given inputs
/// - `predictors`: Return all predictors in this module (for optimization)
///
/// # Optional Methods
///
/// - `set_lm`: Set the language model for this module and sub-modules
/// - `get_lm`: Get the current language model
#[async_trait]
pub trait Module: Send + Sync {
    /// The signature this module implements.
    type Sig: Signature;

    /// Execute the module with the given inputs.
    ///
    /// This is the main entry point for module execution. Implementations
    /// should call their LLM, validate outputs, and return results.
    async fn forward(
        &self,
        inputs: <Self::Sig as Signature>::Inputs,
    ) -> Result<<Self::Sig as Signature>::Outputs>;

    /// Return all predictors in this module for optimization.
    ///
    /// Used by optimizers like BootstrapFewShot to collect all predictors
    /// that can receive few-shot demonstrations.
    fn predictors(&self) -> Vec<&dyn Predictor>;

    /// Set the language model for this module and all sub-modules.
    fn set_lm(&mut self, lm: Arc<dyn LLMClient>);

    /// Get the current language model.
    fn get_lm(&self) -> Option<Arc<dyn LLMClient>>;

    /// Get the module's name for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Trait for predictors that can receive demonstrations.
///
/// This is implemented by `Predict` and allows optimizers to inject
/// few-shot examples without knowing the specific signature type.
pub trait Predictor: Send + Sync {
    /// Add a demonstration to this predictor.
    fn add_demonstration(&mut self, inputs: serde_json::Value, outputs: serde_json::Value);

    /// Clear all demonstrations.
    fn clear_demonstrations(&mut self);

    /// Get the number of demonstrations.
    fn demonstration_count(&self) -> usize;

    /// Get the predictor's name for debugging.
    fn predictor_name(&self) -> &str;
}

/// Configuration for module execution.
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    /// Maximum number of retries on failure.
    pub max_retries: u32,
    /// Temperature for LLM sampling.
    pub temperature: f64,
    /// Maximum tokens for completion.
    pub max_tokens: Option<u32>,
    /// Whether to include demonstrations in prompts.
    pub use_demonstrations: bool,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            temperature: 0.0,
            max_tokens: None,
            use_demonstrations: true,
        }
    }
}

impl ModuleConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = temp;
        self
    }

    /// Set the maximum tokens.
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Disable demonstrations.
    pub fn without_demonstrations(mut self) -> Self {
        self.use_demonstrations = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_config_default() {
        let config = ModuleConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.temperature, 0.0);
        assert!(config.max_tokens.is_none());
        assert!(config.use_demonstrations);
    }

    #[test]
    fn test_module_config_builder() {
        let config = ModuleConfig::new()
            .with_max_retries(5)
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .without_demonstrations();

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, Some(1000));
        assert!(!config.use_demonstrations);
    }
}
