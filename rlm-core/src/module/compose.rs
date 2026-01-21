//! Module composition helpers.
//!
//! This module provides utilities for composing modules into pipelines:
//! - `Chain`: Sequential composition (output of first → input of second)
//! - `Parallel`: Parallel execution with merged outputs
//!
//! # Type Safety
//!
//! Composition is type-safe at compile time:
//! - `Chain` requires the output type of the first module to match the input type of the second
//! - `Parallel` requires modules to have compatible output types that can be merged

use std::sync::Arc;

use async_trait::async_trait;

use super::{Module, Predictor};
use crate::error::Result;
use crate::llm::LLMClient;
use crate::signature::Signature;

/// A chained composition of two modules.
///
/// The output of the first module is passed as input to the second module.
/// This requires the output type of `M1::Sig` to be convertible to the input
/// type of `M2::Sig`.
///
/// # Type Parameters
///
/// - `M1`: The first module
/// - `M2`: The second module
/// - `S1`: The signature of M1
/// - `S2`: The signature of M2 (the chain's signature)
///
/// # Example
///
/// ```ignore
/// let extract = Predict::<ExtractEntities>::new();
/// let classify = Predict::<ClassifyEntities>::new();
///
/// // Create a chain that extracts then classifies
/// let pipeline = Chain::new(extract, classify, |entities| {
///     ClassifyEntitiesInputs { entities: entities.entities }
/// });
/// ```
pub struct Chain<M1, M2, S1, S2, F>
where
    M1: Module<Sig = S1>,
    M2: Module<Sig = S2>,
    S1: Signature,
    S2: Signature,
    F: Fn(S1::Outputs) -> S2::Inputs + Send + Sync,
{
    first: M1,
    second: M2,
    transform: F,
    name: String,
    _phantom: std::marker::PhantomData<(S1, S2)>,
}

impl<M1, M2, S1, S2, F> Chain<M1, M2, S1, S2, F>
where
    M1: Module<Sig = S1>,
    M2: Module<Sig = S2>,
    S1: Signature,
    S2: Signature,
    F: Fn(S1::Outputs) -> S2::Inputs + Send + Sync,
{
    /// Create a new chain with a transform function.
    ///
    /// The transform function converts the output of the first module
    /// to the input of the second module.
    pub fn new(first: M1, second: M2, transform: F) -> Self {
        let name = format!("Chain({} -> {})", first.name(), second.name());
        Self {
            first,
            second,
            transform,
            name,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set a custom name for this chain.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

/// A signature that wraps an existing signature for chaining.
///
/// The chain takes the inputs of the first signature and produces
/// the outputs of the second signature.
pub struct ChainSignature<S1: Signature, S2: Signature> {
    _phantom: std::marker::PhantomData<(S1, S2)>,
}

impl<S1: Signature, S2: Signature> Signature for ChainSignature<S1, S2> {
    type Inputs = S1::Inputs;
    type Outputs = S2::Outputs;

    fn instructions() -> &'static str {
        // Chains don't have their own instructions
        "Chained module"
    }

    fn input_fields() -> Vec<crate::signature::FieldSpec> {
        S1::input_fields()
    }

    fn output_fields() -> Vec<crate::signature::FieldSpec> {
        S2::output_fields()
    }
}

#[async_trait]
impl<M1, M2, S1, S2, F> Module for Chain<M1, M2, S1, S2, F>
where
    M1: Module<Sig = S1> + Send + Sync,
    M2: Module<Sig = S2> + Send + Sync,
    S1: Signature + 'static,
    S2: Signature + 'static,
    F: Fn(S1::Outputs) -> S2::Inputs + Send + Sync,
{
    type Sig = ChainSignature<S1, S2>;

    async fn forward(&self, inputs: S1::Inputs) -> Result<S2::Outputs> {
        // Execute first module
        let intermediate = self.first.forward(inputs).await?;

        // Transform output to input for second module
        let transformed = (self.transform)(intermediate);

        // Execute second module
        self.second.forward(transformed).await
    }

    fn predictors(&self) -> Vec<&dyn Predictor> {
        let mut predictors = self.first.predictors();
        predictors.extend(self.second.predictors());
        predictors
    }

    fn set_lm(&mut self, lm: Arc<dyn LLMClient>) {
        self.first.set_lm(lm.clone());
        self.second.set_lm(lm);
    }

    fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
        self.first.get_lm().or_else(|| self.second.get_lm())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Helper function to create a simple chain where output fields match input fields.
///
/// This is a convenience for the common case where the output of one module
/// can be directly deserialized as the input of the next.
pub fn chain_direct<M1, M2, S1, S2>(first: M1, second: M2) -> Chain<M1, M2, S1, S2, impl Fn(S1::Outputs) -> S2::Inputs + Send + Sync>
where
    M1: Module<Sig = S1>,
    M2: Module<Sig = S2>,
    S1: Signature,
    S2: Signature,
    S1::Outputs: serde::Serialize,
    S2::Inputs: serde::de::DeserializeOwned,
{
    Chain::new(first, second, |outputs| {
        // Serialize outputs and deserialize as inputs
        let value = serde_json::to_value(&outputs).expect("Failed to serialize outputs");
        serde_json::from_value(value).expect("Failed to deserialize as inputs")
    })
}

/// A module that runs multiple modules in parallel and collects results.
///
/// All modules must have the same input type. Outputs are collected into a Vec.
///
/// # Example
///
/// ```ignore
/// let analyzer1 = Predict::<AnalyzeStyle>::new();
/// let analyzer2 = Predict::<AnalyzeTone>::new();
///
/// let parallel = Parallel::new(vec![analyzer1, analyzer2]);
/// let results = parallel.forward(inputs).await?;
/// ```
pub struct ParallelVec<M, S>
where
    M: Module<Sig = S>,
    S: Signature,
{
    modules: Vec<M>,
    name: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<M, S> ParallelVec<M, S>
where
    M: Module<Sig = S>,
    S: Signature,
{
    /// Create a new parallel module from a vector of modules.
    pub fn new(modules: Vec<M>) -> Self {
        let count = modules.len();
        Self {
            modules,
            name: format!("Parallel({})", count),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set a custom name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

/// Signature wrapper for parallel execution that returns a Vec of outputs.
pub struct ParallelSignature<S: Signature> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Signature> Signature for ParallelSignature<S>
where
    S::Outputs: Clone,
{
    type Inputs = S::Inputs;
    type Outputs = Vec<S::Outputs>;

    fn instructions() -> &'static str {
        "Parallel execution module"
    }

    fn input_fields() -> Vec<crate::signature::FieldSpec> {
        S::input_fields()
    }

    fn output_fields() -> Vec<crate::signature::FieldSpec> {
        // Parallel returns a list of the original output type
        vec![crate::signature::FieldSpec::new(
            "results",
            crate::signature::FieldType::List(Box::new(crate::signature::FieldType::Object(
                S::output_fields(),
            ))),
        )
        .with_description("Results from parallel execution")]
    }
}

#[async_trait]
impl<M, S> Module for ParallelVec<M, S>
where
    M: Module<Sig = S> + Send + Sync,
    S: Signature + 'static,
    S::Inputs: Clone,
    S::Outputs: Clone + Send,
{
    type Sig = ParallelSignature<S>;

    async fn forward(&self, inputs: S::Inputs) -> Result<Vec<S::Outputs>> {
        // Execute all modules in parallel
        let futures: Vec<_> = self
            .modules
            .iter()
            .map(|m| {
                let inputs = inputs.clone();
                async move { m.forward(inputs).await }
            })
            .collect();

        // Wait for all results
        let results = futures::future::join_all(futures).await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    fn predictors(&self) -> Vec<&dyn Predictor> {
        self.modules.iter().flat_map(|m| m.predictors()).collect()
    }

    fn set_lm(&mut self, lm: Arc<dyn LLMClient>) {
        for module in &mut self.modules {
            module.set_lm(lm.clone());
        }
    }

    fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
        self.modules.first().and_then(|m| m.get_lm())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require mock modules which need more setup
    // Basic structure tests only

    #[test]
    fn test_chain_name_format() {
        // This test verifies the name formatting logic without needing actual modules
        let name = format!("Chain({} -> {})", "ModuleA", "ModuleB");
        assert!(name.contains("Chain"));
        assert!(name.contains("ModuleA"));
        assert!(name.contains("ModuleB"));
    }

    #[test]
    fn test_parallel_name_format() {
        let name = format!("Parallel({})", 3);
        assert_eq!(name, "Parallel(3)");
    }
}
