//! Python bindings for LLM types.

use pyo3::prelude::*;
use std::collections::HashMap;

use crate::llm::{
    ChatMessage, ChatRole, CompletionRequest, CompletionResponse, CostTracker, ModelSpec,
    ModelTier, Provider, QueryType, RoutingContext, RoutingDecision, SmartRouter, StopReason,
    TokenUsage,
};

/// Python enum for Provider.
#[pyclass(name = "Provider", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyProvider {
    Anthropic = 0,
    OpenAI = 1,
    OpenRouter = 2,
}

impl From<Provider> for PyProvider {
    fn from(p: Provider) -> Self {
        match p {
            Provider::Anthropic => PyProvider::Anthropic,
            Provider::OpenAI => PyProvider::OpenAI,
            Provider::OpenRouter => PyProvider::OpenRouter,
        }
    }
}

impl From<PyProvider> for Provider {
    fn from(p: PyProvider) -> Self {
        match p {
            PyProvider::Anthropic => Provider::Anthropic,
            PyProvider::OpenAI => Provider::OpenAI,
            PyProvider::OpenRouter => Provider::OpenRouter,
        }
    }
}

#[pymethods]
impl PyProvider {
    fn __repr__(&self) -> &'static str {
        match self {
            PyProvider::Anthropic => "Provider.Anthropic",
            PyProvider::OpenAI => "Provider.OpenAI",
            PyProvider::OpenRouter => "Provider.OpenRouter",
        }
    }
}

/// Python enum for ModelTier.
#[pyclass(name = "ModelTier", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyModelTier {
    Flagship = 0,
    Balanced = 1,
    Fast = 2,
}

impl From<ModelTier> for PyModelTier {
    fn from(t: ModelTier) -> Self {
        match t {
            ModelTier::Flagship => PyModelTier::Flagship,
            ModelTier::Balanced => PyModelTier::Balanced,
            ModelTier::Fast => PyModelTier::Fast,
        }
    }
}

impl From<PyModelTier> for ModelTier {
    fn from(t: PyModelTier) -> Self {
        match t {
            PyModelTier::Flagship => ModelTier::Flagship,
            PyModelTier::Balanced => ModelTier::Balanced,
            PyModelTier::Fast => ModelTier::Fast,
        }
    }
}

#[pymethods]
impl PyModelTier {
    fn __repr__(&self) -> &'static str {
        match self {
            PyModelTier::Flagship => "ModelTier.Flagship",
            PyModelTier::Balanced => "ModelTier.Balanced",
            PyModelTier::Fast => "ModelTier.Fast",
        }
    }
}

/// Python enum for QueryType.
#[pyclass(name = "QueryType", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyQueryType {
    Architecture = 0,
    MultiFile = 1,
    Debugging = 2,
    Extraction = 3,
    Simple = 4,
}

impl From<QueryType> for PyQueryType {
    fn from(q: QueryType) -> Self {
        match q {
            QueryType::Architecture => PyQueryType::Architecture,
            QueryType::MultiFile => PyQueryType::MultiFile,
            QueryType::Debugging => PyQueryType::Debugging,
            QueryType::Extraction => PyQueryType::Extraction,
            QueryType::Simple => PyQueryType::Simple,
        }
    }
}

impl From<PyQueryType> for QueryType {
    fn from(q: PyQueryType) -> Self {
        match q {
            PyQueryType::Architecture => QueryType::Architecture,
            PyQueryType::MultiFile => QueryType::MultiFile,
            PyQueryType::Debugging => QueryType::Debugging,
            PyQueryType::Extraction => QueryType::Extraction,
            PyQueryType::Simple => QueryType::Simple,
        }
    }
}

#[pymethods]
impl PyQueryType {
    /// Classify a query string.
    #[staticmethod]
    fn classify(query: &str) -> Self {
        QueryType::classify(query).into()
    }

    /// Get the base tier for this query type.
    fn base_tier(&self) -> PyModelTier {
        QueryType::from(*self).base_tier().into()
    }

    fn __repr__(&self) -> &'static str {
        match self {
            PyQueryType::Architecture => "QueryType.Architecture",
            PyQueryType::MultiFile => "QueryType.MultiFile",
            PyQueryType::Debugging => "QueryType.Debugging",
            PyQueryType::Extraction => "QueryType.Extraction",
            PyQueryType::Simple => "QueryType.Simple",
        }
    }
}

/// Python wrapper for ModelSpec.
#[pyclass(name = "ModelSpec")]
#[derive(Clone)]
pub struct PyModelSpec {
    pub(crate) inner: ModelSpec,
}

#[pymethods]
impl PyModelSpec {
    #[new]
    #[pyo3(signature = (id, name, provider, tier, context_window, max_output, input_cost, output_cost))]
    fn new(
        id: String,
        name: String,
        provider: PyProvider,
        tier: PyModelTier,
        context_window: u32,
        max_output: u32,
        input_cost: f64,
        output_cost: f64,
    ) -> Self {
        Self {
            inner: ModelSpec {
                id,
                name,
                provider: provider.into(),
                tier: tier.into(),
                context_window,
                max_output,
                input_cost_per_m: input_cost,
                output_cost_per_m: output_cost,
                supports_caching: false,
                supports_vision: false,
                supports_tools: false,
            },
        }
    }

    /// Create Claude Opus spec.
    #[staticmethod]
    fn claude_opus() -> Self {
        Self {
            inner: ModelSpec::claude_opus(),
        }
    }

    /// Create Claude Sonnet spec.
    #[staticmethod]
    fn claude_sonnet() -> Self {
        Self {
            inner: ModelSpec::claude_sonnet(),
        }
    }

    /// Create Claude Haiku spec.
    #[staticmethod]
    fn claude_haiku() -> Self {
        Self {
            inner: ModelSpec::claude_haiku(),
        }
    }

    /// Create GPT-4o spec.
    #[staticmethod]
    fn gpt4o() -> Self {
        Self {
            inner: ModelSpec::gpt4o(),
        }
    }

    /// Create GPT-4o Mini spec.
    #[staticmethod]
    fn gpt4o_mini() -> Self {
        Self {
            inner: ModelSpec::gpt4o_mini(),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn provider(&self) -> PyProvider {
        self.inner.provider.into()
    }

    #[getter]
    fn tier(&self) -> PyModelTier {
        self.inner.tier.into()
    }

    #[getter]
    fn context_window(&self) -> u32 {
        self.inner.context_window
    }

    #[getter]
    fn max_output(&self) -> u32 {
        self.inner.max_output
    }

    #[getter]
    fn input_cost_per_m(&self) -> f64 {
        self.inner.input_cost_per_m
    }

    #[getter]
    fn output_cost_per_m(&self) -> f64 {
        self.inner.output_cost_per_m
    }

    #[getter]
    fn supports_caching(&self) -> bool {
        self.inner.supports_caching
    }

    #[getter]
    fn supports_vision(&self) -> bool {
        self.inner.supports_vision
    }

    #[getter]
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools
    }

    /// Calculate cost for given token usage.
    fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        self.inner.calculate_cost(input_tokens, output_tokens)
    }

    fn __repr__(&self) -> String {
        format!("ModelSpec(id={:?}, tier={:?})", self.inner.id, self.inner.tier)
    }
}

/// Python wrapper for ChatMessage.
#[pyclass(name = "ChatMessage")]
#[derive(Clone)]
pub struct PyChatMessage {
    pub(crate) inner: ChatMessage,
}

#[pymethods]
impl PyChatMessage {
    /// Create a system message.
    #[staticmethod]
    fn system(content: String) -> Self {
        Self {
            inner: ChatMessage::system(content),
        }
    }

    /// Create a user message.
    #[staticmethod]
    fn user(content: String) -> Self {
        Self {
            inner: ChatMessage::user(content),
        }
    }

    /// Create an assistant message.
    #[staticmethod]
    fn assistant(content: String) -> Self {
        Self {
            inner: ChatMessage::assistant(content),
        }
    }

    #[getter]
    fn role(&self) -> &'static str {
        match self.inner.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ChatMessage(role={:?}, content={:?})",
            self.role(),
            truncate(&self.inner.content, 50)
        )
    }
}

/// Python wrapper for TokenUsage.
#[pyclass(name = "TokenUsage")]
#[derive(Clone)]
pub struct PyTokenUsage {
    pub(crate) inner: TokenUsage,
}

#[pymethods]
impl PyTokenUsage {
    #[new]
    #[pyo3(signature = (input_tokens, output_tokens, cache_read=None, cache_creation=None))]
    fn new(
        input_tokens: u64,
        output_tokens: u64,
        cache_read: Option<u64>,
        cache_creation: Option<u64>,
    ) -> Self {
        Self {
            inner: TokenUsage {
                input_tokens,
                output_tokens,
                cache_read_tokens: cache_read,
                cache_creation_tokens: cache_creation,
            },
        }
    }

    #[getter]
    fn input_tokens(&self) -> u64 {
        self.inner.input_tokens
    }

    #[getter]
    fn output_tokens(&self) -> u64 {
        self.inner.output_tokens
    }

    #[getter]
    fn cache_read_tokens(&self) -> Option<u64> {
        self.inner.cache_read_tokens
    }

    #[getter]
    fn cache_creation_tokens(&self) -> Option<u64> {
        self.inner.cache_creation_tokens
    }

    /// Get total tokens.
    fn total(&self) -> u64 {
        self.inner.total()
    }

    /// Get effective input tokens (accounting for cache).
    fn effective_input_tokens(&self) -> u64 {
        self.inner.effective_input_tokens()
    }

    fn __repr__(&self) -> String {
        format!(
            "TokenUsage(input={}, output={})",
            self.inner.input_tokens, self.inner.output_tokens
        )
    }
}

/// Python wrapper for CompletionRequest.
#[pyclass(name = "CompletionRequest")]
#[derive(Clone)]
pub struct PyCompletionRequest {
    pub(crate) inner: CompletionRequest,
}

#[pymethods]
impl PyCompletionRequest {
    #[new]
    fn new() -> Self {
        Self {
            inner: CompletionRequest::new(),
        }
    }

    /// Set the model.
    fn with_model(&mut self, model: String) -> Self {
        self.inner.model = Some(model);
        self.clone()
    }

    /// Set the system prompt.
    fn with_system(&mut self, system: String) -> Self {
        self.inner.system = Some(system);
        self.clone()
    }

    /// Add a message.
    fn with_message(&mut self, message: &PyChatMessage) -> Self {
        self.inner.messages.push(message.inner.clone());
        self.clone()
    }

    /// Set max tokens.
    fn with_max_tokens(&mut self, max_tokens: u32) -> Self {
        self.inner.max_tokens = Some(max_tokens);
        self.clone()
    }

    /// Set temperature.
    fn with_temperature(&mut self, temperature: f64) -> Self {
        self.inner.temperature = Some(temperature.clamp(0.0, 1.0));
        self.clone()
    }

    /// Enable caching.
    fn with_caching(&mut self, enable: bool) -> Self {
        self.inner.enable_caching = enable;
        self.clone()
    }

    #[getter]
    fn model(&self) -> Option<String> {
        self.inner.model.clone()
    }

    #[getter]
    fn system(&self) -> Option<String> {
        self.inner.system.clone()
    }

    #[getter]
    fn max_tokens(&self) -> Option<u32> {
        self.inner.max_tokens
    }

    #[getter]
    fn temperature(&self) -> Option<f64> {
        self.inner.temperature
    }

    fn __repr__(&self) -> String {
        format!(
            "CompletionRequest(model={:?}, messages={})",
            self.inner.model,
            self.inner.messages.len()
        )
    }
}

/// Python wrapper for CompletionResponse.
#[pyclass(name = "CompletionResponse")]
#[derive(Clone)]
pub struct PyCompletionResponse {
    pub(crate) inner: CompletionResponse,
}

#[pymethods]
impl PyCompletionResponse {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn model(&self) -> String {
        self.inner.model.clone()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn stop_reason(&self) -> Option<&'static str> {
        self.inner.stop_reason.map(|r| match r {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::StopSequence => "stop_sequence",
            StopReason::ToolUse => "tool_use",
        })
    }

    #[getter]
    fn usage(&self) -> PyTokenUsage {
        PyTokenUsage {
            inner: self.inner.usage.clone(),
        }
    }

    #[getter]
    fn timestamp(&self) -> String {
        self.inner.timestamp.to_rfc3339()
    }

    #[getter]
    fn cost(&self) -> Option<f64> {
        self.inner.cost
    }

    fn __repr__(&self) -> String {
        format!(
            "CompletionResponse(model={:?}, tokens={})",
            self.inner.model,
            self.inner.usage.total()
        )
    }
}

/// Python wrapper for RoutingContext.
#[pyclass(name = "RoutingContext")]
#[derive(Clone)]
pub struct PyRoutingContext {
    pub(crate) inner: RoutingContext,
}

#[pymethods]
impl PyRoutingContext {
    #[new]
    fn new() -> Self {
        Self {
            inner: RoutingContext::new(),
        }
    }

    /// Set the depth.
    fn with_depth(&mut self, depth: u32) -> Self {
        self.inner.depth = depth;
        self.clone()
    }

    /// Set the max depth.
    fn with_max_depth(&mut self, max_depth: u32) -> Self {
        self.inner.max_depth = max_depth;
        self.clone()
    }

    /// Set the remaining budget.
    fn with_budget(&mut self, budget: f64) -> Self {
        self.inner.remaining_budget = Some(budget);
        self.clone()
    }

    /// Set the preferred provider.
    fn with_provider(&mut self, provider: PyProvider) -> Self {
        self.inner.preferred_provider = Some(provider.into());
        self.clone()
    }

    /// Require caching support.
    fn requiring_caching(&mut self) -> Self {
        self.inner.require_caching = true;
        self.clone()
    }

    /// Require vision support.
    fn requiring_vision(&mut self) -> Self {
        self.inner.require_vision = true;
        self.clone()
    }

    /// Require tool use support.
    fn requiring_tools(&mut self) -> Self {
        self.inner.require_tools = true;
        self.clone()
    }

    #[getter]
    fn depth(&self) -> u32 {
        self.inner.depth
    }

    #[getter]
    fn max_depth(&self) -> u32 {
        self.inner.max_depth
    }

    #[getter]
    fn remaining_budget(&self) -> Option<f64> {
        self.inner.remaining_budget
    }

    fn __repr__(&self) -> String {
        format!(
            "RoutingContext(depth={}, max_depth={})",
            self.inner.depth, self.inner.max_depth
        )
    }
}

/// Python wrapper for SmartRouter.
#[pyclass(name = "SmartRouter")]
pub struct PySmartRouter {
    inner: SmartRouter,
}

#[pymethods]
impl PySmartRouter {
    #[new]
    fn new() -> Self {
        Self {
            inner: SmartRouter::new(),
        }
    }

    /// Route a query to the best model.
    fn route(&self, query: &str, context: &PyRoutingContext) -> PyRoutingDecision {
        PyRoutingDecision {
            inner: self.inner.route(query, &context.inner),
        }
    }

    /// Get all available models.
    fn models(&self) -> Vec<PyModelSpec> {
        self.inner
            .models()
            .iter()
            .map(|m| PyModelSpec { inner: m.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("SmartRouter(models={})", self.inner.models().len())
    }
}

/// Python wrapper for RoutingDecision.
#[pyclass(name = "RoutingDecision")]
#[derive(Clone)]
pub struct PyRoutingDecision {
    inner: RoutingDecision,
}

#[pymethods]
impl PyRoutingDecision {
    #[getter]
    fn model(&self) -> PyModelSpec {
        PyModelSpec {
            inner: self.inner.model.clone(),
        }
    }

    #[getter]
    fn query_type(&self) -> PyQueryType {
        self.inner.query_type.into()
    }

    #[getter]
    fn tier(&self) -> PyModelTier {
        self.inner.tier.into()
    }

    #[getter]
    fn reason(&self) -> String {
        self.inner.reason.clone()
    }

    #[getter]
    fn estimated_cost(&self) -> Option<f64> {
        self.inner.estimated_cost
    }

    fn __repr__(&self) -> String {
        format!(
            "RoutingDecision(model={:?}, tier={:?})",
            self.inner.model.id, self.inner.tier
        )
    }
}

/// Python wrapper for CostTracker.
#[pyclass(name = "CostTracker")]
#[derive(Clone)]
pub struct PyCostTracker {
    pub(crate) inner: CostTracker,
}

#[pymethods]
impl PyCostTracker {
    #[new]
    fn new() -> Self {
        Self {
            inner: CostTracker::new(),
        }
    }

    /// Record usage from a completion.
    #[pyo3(signature = (model, usage, cost=None))]
    fn record(&mut self, model: &str, usage: &PyTokenUsage, cost: Option<f64>) {
        self.inner.record(model, &usage.inner, cost);
    }

    /// Merge another tracker into this one.
    fn merge(&mut self, other: &PyCostTracker) {
        self.inner.merge(&other.inner);
    }

    #[getter]
    fn total_input_tokens(&self) -> u64 {
        self.inner.total_input_tokens
    }

    #[getter]
    fn total_output_tokens(&self) -> u64 {
        self.inner.total_output_tokens
    }

    #[getter]
    fn total_cache_read_tokens(&self) -> u64 {
        self.inner.total_cache_read_tokens
    }

    #[getter]
    fn total_cost(&self) -> f64 {
        self.inner.total_cost
    }

    #[getter]
    fn request_count(&self) -> u64 {
        self.inner.request_count
    }

    #[getter]
    fn by_model(&self) -> HashMap<String, HashMap<String, f64>> {
        self.inner
            .by_model
            .iter()
            .map(|(k, v)| {
                let mut map = HashMap::new();
                map.insert("input_tokens".to_string(), v.input_tokens as f64);
                map.insert("output_tokens".to_string(), v.output_tokens as f64);
                map.insert("cost".to_string(), v.cost);
                map.insert("request_count".to_string(), v.request_count as f64);
                (k.clone(), map)
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CostTracker(requests={}, cost=${:.4})",
            self.inner.request_count, self.inner.total_cost
        )
    }
}

/// Truncate a string for display.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
