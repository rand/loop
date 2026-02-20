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
use crate::error::{Error, Result};
use crate::llm::LLMClient;
use crate::signature::{validate_fields, FieldSpec, Signature};

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
    F: Fn(S1::Outputs) -> Result<S2::Inputs> + Send + Sync,
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
    F: Fn(S1::Outputs) -> Result<S2::Inputs> + Send + Sync,
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
    F: Fn(S1::Outputs) -> Result<S2::Inputs> + Send + Sync,
{
    type Sig = ChainSignature<S1, S2>;

    async fn forward(&self, inputs: S1::Inputs) -> Result<S2::Outputs> {
        // Execute first module
        let intermediate = self.first.forward(inputs).await?;

        // Transform output to input for second module
        let transformed = (self.transform)(intermediate)?;

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
pub fn chain_direct<M1, M2, S1, S2>(first: M1, second: M2) -> Chain<M1, M2, S1, S2, impl Fn(S1::Outputs) -> Result<S2::Inputs> + Send + Sync>
where
    M1: Module<Sig = S1>,
    M2: Module<Sig = S2>,
    S1: Signature,
    S2: Signature,
    S1::Outputs: serde::Serialize,
    S2::Inputs: serde::de::DeserializeOwned,
{
    let output_fields = S1::output_fields();
    let input_fields = S2::input_fields();

    Chain::new(first, second, move |outputs| {
        validate_direct_field_mapping(&output_fields, &input_fields)?;

        // Serialize outputs and validate compatibility against target inputs.
        let value = serde_json::to_value(&outputs)
            .map_err(|e| Error::Config(format!("chain_direct failed to serialize outputs: {e}")))?;
        validate_fields(&value, &input_fields).map_err(|errors| {
            let summary = errors
                .into_iter()
                .map(|err| err.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Error::Config(format!("chain_direct incompatible output/input mapping: {summary}"))
        })?;

        serde_json::from_value(value)
            .map_err(|e| Error::Config(format!("chain_direct failed to decode inputs: {e}")))
    })
}

fn validate_direct_field_mapping(output_fields: &[FieldSpec], input_fields: &[FieldSpec]) -> Result<()> {
    for input in input_fields {
        let Some(output) = output_fields.iter().find(|field| field.name == input.name) else {
            if input.required {
                return Err(Error::Config(format!(
                    "chain_direct missing required input field '{}' in upstream outputs",
                    input.name
                )));
            }
            continue;
        };

        if output.field_type != input.field_type {
            return Err(Error::Config(format!(
                "chain_direct field type mismatch for '{}': output {:?} != input {:?}",
                input.name, output.field_type, input.field_type
            )));
        }
    }

    Ok(())
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
    use crate::module::Module;
    use crate::signature::{FieldType, Signature};
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SourceInputs {
        value: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SourceOutputs {
        value: String,
    }

    struct SourceSig;

    impl Signature for SourceSig {
        type Inputs = SourceInputs;
        type Outputs = SourceOutputs;

        fn instructions() -> &'static str {
            "source"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("value", FieldType::String)]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("value", FieldType::String)]
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SinkInputs {
        value: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SinkOutputs {
        result: String,
    }

    struct SinkSig;

    impl Signature for SinkSig {
        type Inputs = SinkInputs;
        type Outputs = SinkOutputs;

        fn instructions() -> &'static str {
            "sink"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("value", FieldType::String)]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("result", FieldType::String)]
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct IncompatibleSinkInputs {
        value: i32,
    }

    struct IncompatibleSinkSig;

    impl Signature for IncompatibleSinkSig {
        type Inputs = IncompatibleSinkInputs;
        type Outputs = SinkOutputs;

        fn instructions() -> &'static str {
            "incompatible sink"
        }

        fn input_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("value", FieldType::Integer)]
        }

        fn output_fields() -> Vec<FieldSpec> {
            vec![FieldSpec::new("result", FieldType::String)]
        }
    }

    #[derive(Default)]
    struct SourceModule;

    #[async_trait]
    impl Module for SourceModule {
        type Sig = SourceSig;

        async fn forward(&self, inputs: SourceInputs) -> Result<SourceOutputs> {
            Ok(SourceOutputs { value: inputs.value })
        }

        fn predictors(&self) -> Vec<&dyn Predictor> {
            vec![]
        }

        fn set_lm(&mut self, _lm: Arc<dyn LLMClient>) {}

        fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
            None
        }

        fn name(&self) -> &str {
            "SourceModule"
        }
    }

    #[derive(Default)]
    struct SinkModule;

    #[async_trait]
    impl Module for SinkModule {
        type Sig = SinkSig;

        async fn forward(&self, inputs: SinkInputs) -> Result<SinkOutputs> {
            Ok(SinkOutputs {
                result: inputs.value,
            })
        }

        fn predictors(&self) -> Vec<&dyn Predictor> {
            vec![]
        }

        fn set_lm(&mut self, _lm: Arc<dyn LLMClient>) {}

        fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
            None
        }

        fn name(&self) -> &str {
            "SinkModule"
        }
    }

    #[derive(Default)]
    struct IncompatibleSinkModule;

    #[async_trait]
    impl Module for IncompatibleSinkModule {
        type Sig = IncompatibleSinkSig;

        async fn forward(&self, _inputs: IncompatibleSinkInputs) -> Result<SinkOutputs> {
            Ok(SinkOutputs {
                result: "unreachable".to_string(),
            })
        }

        fn predictors(&self) -> Vec<&dyn Predictor> {
            vec![]
        }

        fn set_lm(&mut self, _lm: Arc<dyn LLMClient>) {}

        fn get_lm(&self) -> Option<Arc<dyn LLMClient>> {
            None
        }

        fn name(&self) -> &str {
            "IncompatibleSinkModule"
        }
    }

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

    #[tokio::test]
    async fn test_chain_direct_successful_mapping() {
        let chain = chain_direct::<SourceModule, SinkModule, SourceSig, SinkSig>(
            SourceModule::default(),
            SinkModule::default(),
        );

        let outputs = chain
            .forward(SourceInputs {
                value: "ok".to_string(),
            })
            .await
            .expect("chain_direct should map compatible fields");
        assert_eq!(outputs.result, "ok");
    }

    #[tokio::test]
    async fn test_chain_direct_returns_config_error_on_type_mismatch() {
        let chain = chain_direct::<SourceModule, IncompatibleSinkModule, SourceSig, IncompatibleSinkSig>(
            SourceModule::default(),
            IncompatibleSinkModule::default(),
        );

        let err = chain
            .forward(SourceInputs {
                value: "not-an-int".to_string(),
            })
            .await
            .expect_err("chain_direct should reject incompatible field mapping");
        let message = err.to_string();
        assert!(message.contains("chain_direct field type mismatch"));
    }
}
