//! LLM client trait and provider implementations.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::error::{Error, Result};

use super::types::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse, ModelSpec,
    Provider, StopReason, TokenUsage,
};

/// LLM client trait for making completions and embeddings.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Complete a prompt.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Create embeddings for texts.
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Get the provider for this client.
    fn provider(&self) -> Provider;

    /// List available models.
    fn available_models(&self) -> Vec<ModelSpec>;
}

/// Configuration for LLM clients.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API key
    pub api_key: String,
    /// Base URL override
    pub base_url: Option<String>,
    /// Default model
    pub default_model: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Max retries on failure
    pub max_retries: u32,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            default_model: None,
            timeout_secs: 120,
            max_retries: 3,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

fn build_http_client(timeout_secs: u64) -> Client {
    let timeout = Duration::from_secs(timeout_secs);

    // Some sandboxed macOS environments can panic during proxy auto-detection
    // in reqwest's default client builder. Fall back to no-proxy in that case.
    match catch_unwind(AssertUnwindSafe(|| {
        Client::builder().timeout(timeout).build()
    })) {
        Ok(Ok(client)) => client,
        Ok(Err(_)) | Err(_) => Client::builder()
            .no_proxy()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client"),
    }
}

/// Anthropic Claude client.
pub struct AnthropicClient {
    config: ClientConfig,
    http: Client,
}

impl AnthropicClient {
    const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com";
    const API_VERSION: &'static str = "2023-06-01";

    pub fn new(config: ClientConfig) -> Self {
        let http = build_http_client(config.timeout_secs);

        Self { config, http }
    }

    fn base_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_BASE_URL)
    }
}

// Anthropic API types
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let model = request
            .model
            .or(self.config.default_model.clone())
            .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .map(|m| AnthropicMessage {
                role: match m.role {
                    super::types::ChatRole::User => "user".to_string(),
                    super::types::ChatRole::Assistant => "assistant".to_string(),
                    super::types::ChatRole::System => "user".to_string(), // System handled separately
                },
                content: m.content.clone(),
            })
            .collect();

        let api_request = AnthropicRequest {
            model: model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: request.system,
            temperature: request.temperature,
            stop_sequences: request.stop,
        };

        let url = format!("{}/v1/messages", self.base_url());

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", Self::API_VERSION)
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::LLM(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {
                return Err(Error::LLM(format!(
                    "Anthropic API error ({}): {}",
                    error.error.error_type, error.error.message
                )));
            }
            return Err(Error::LLM(format!(
                "Anthropic API error ({}): {}",
                status, body
            )));
        }

        let api_response: AnthropicResponse = serde_json::from_str(&body)
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))?;

        let content = api_response
            .content
            .iter()
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        let stop_reason = api_response.stop_reason.as_deref().map(|r| match r {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "tool_use" => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        });

        let usage = TokenUsage {
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
            cache_read_tokens: api_response.usage.cache_read_input_tokens,
            cache_creation_tokens: api_response.usage.cache_creation_input_tokens,
        };

        // Calculate cost based on model
        let model_spec = self
            .available_models()
            .into_iter()
            .find(|m| m.id == model)
            .unwrap_or_else(ModelSpec::claude_sonnet);
        let cost = model_spec.calculate_cost(usage.input_tokens, usage.output_tokens);

        Ok(CompletionResponse {
            id: api_response.id,
            model: api_response.model,
            content,
            stop_reason,
            usage,
            timestamp: Utc::now(),
            cost: Some(cost),
        })
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Anthropic doesn't have a native embedding API
        // In production, this would use a partner service or Voyage AI
        Err(Error::LLM(
            "Anthropic does not provide direct embedding API".to_string(),
        ))
    }

    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    fn available_models(&self) -> Vec<ModelSpec> {
        vec![
            ModelSpec::claude_opus(),
            ModelSpec::claude_sonnet(),
            ModelSpec::claude_haiku(),
        ]
    }
}

/// OpenAI client.
pub struct OpenAIClient {
    config: ClientConfig,
    http: Client,
}

impl OpenAIClient {
    const DEFAULT_BASE_URL: &'static str = "https://api.openai.com";

    pub fn new(config: ClientConfig) -> Self {
        let http = build_http_client(config.timeout_secs);

        Self { config, http }
    }

    fn base_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_BASE_URL)
    }
}

// OpenAI API types
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
}

// OpenAI Embedding types
#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    model: String,
    data: Vec<OpenAIEmbeddingData>,
    usage: OpenAIEmbeddingUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingUsage {
    prompt_tokens: u64,
    #[allow(dead_code)]
    total_tokens: u64,
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let model = request
            .model
            .or(self.config.default_model.clone())
            .unwrap_or_else(|| "gpt-4o".to_string());

        let mut messages: Vec<OpenAIMessage> = Vec::new();

        // Add system message if present
        if let Some(system) = &request.system {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for m in &request.messages {
            messages.push(OpenAIMessage {
                role: match m.role {
                    super::types::ChatRole::User => "user".to_string(),
                    super::types::ChatRole::Assistant => "assistant".to_string(),
                    super::types::ChatRole::System => "system".to_string(),
                },
                content: m.content.clone(),
            });
        }

        let api_request = OpenAIRequest {
            model: model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stop: request.stop,
        };

        let url = format!("{}/v1/chat/completions", self.base_url());

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::LLM(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
                return Err(Error::LLM(format!(
                    "OpenAI API error: {}",
                    error.error.message
                )));
            }
            return Err(Error::LLM(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let api_response: OpenAIResponse = serde_json::from_str(&body)
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))?;

        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| Error::LLM("No choices in response".to_string()))?;

        let stop_reason = choice.finish_reason.as_deref().map(|r| match r {
            "stop" => StopReason::EndTurn,
            "length" => StopReason::MaxTokens,
            "tool_calls" => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        });

        let usage = TokenUsage {
            input_tokens: api_response.usage.prompt_tokens,
            output_tokens: api_response.usage.completion_tokens,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        };

        // Calculate cost based on model
        let model_spec = self
            .available_models()
            .into_iter()
            .find(|m| m.id == model || model.starts_with(&m.id))
            .unwrap_or_else(ModelSpec::gpt4o);
        let cost = model_spec.calculate_cost(usage.input_tokens, usage.output_tokens);

        Ok(CompletionResponse {
            id: api_response.id,
            model: api_response.model,
            content: choice.message.content.clone(),
            stop_reason,
            usage,
            timestamp: Utc::now(),
            cost: Some(cost),
        })
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let model = request
            .model
            .unwrap_or_else(|| "text-embedding-3-small".to_string());

        let api_request = OpenAIEmbeddingRequest {
            model: model.clone(),
            input: request.texts,
        };

        let url = format!("{}/v1/embeddings", self.base_url());

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::LLM(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
                return Err(Error::LLM(format!(
                    "OpenAI API error: {}",
                    error.error.message
                )));
            }
            return Err(Error::LLM(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let api_response: OpenAIEmbeddingResponse = serde_json::from_str(&body)
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))?;

        let embeddings = api_response.data.into_iter().map(|d| d.embedding).collect();

        Ok(EmbeddingResponse {
            model: api_response.model,
            embeddings,
            usage: TokenUsage {
                input_tokens: api_response.usage.prompt_tokens,
                output_tokens: 0,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            },
        })
    }

    fn provider(&self) -> Provider {
        Provider::OpenAI
    }

    fn available_models(&self) -> Vec<ModelSpec> {
        vec![ModelSpec::gpt4o(), ModelSpec::gpt4o_mini()]
    }
}

/// Google Gemini client.
#[cfg(feature = "gemini")]
pub struct GoogleClient {
    config: ClientConfig,
    http: Client,
}

#[cfg(feature = "gemini")]
impl GoogleClient {
    const DEFAULT_BASE_URL: &'static str = "https://generativelanguage.googleapis.com";

    pub fn new(config: ClientConfig) -> Self {
        let http = build_http_client(config.timeout_secs);

        Self { config, http }
    }

    fn base_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_BASE_URL)
    }
}

// Google Gemini API types
#[cfg(feature = "gemini")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    prompt_token_count: u64,
    candidates_token_count: Option<u64>,
    #[allow(dead_code)]
    total_token_count: Option<u64>,
    cached_content_token_count: Option<u64>,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Deserialize)]
struct GeminiError {
    error: GeminiErrorDetail,
}

#[cfg(feature = "gemini")]
#[derive(Debug, Deserialize)]
struct GeminiErrorDetail {
    message: String,
    #[allow(dead_code)]
    status: Option<String>,
}

#[cfg(feature = "gemini")]
#[async_trait]
impl LLMClient for GoogleClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let model = request
            .model
            .or(self.config.default_model.clone())
            .unwrap_or_else(|| "gemini-2.0-flash".to_string());

        // Build contents from messages
        let contents: Vec<GeminiContent> = request
            .messages
            .iter()
            .map(|m| GeminiContent {
                role: match m.role {
                    super::types::ChatRole::User => "user".to_string(),
                    super::types::ChatRole::Assistant => "model".to_string(),
                    super::types::ChatRole::System => "user".to_string(), // Handled separately
                },
                parts: vec![GeminiPart {
                    text: m.content.clone(),
                }],
            })
            .collect();

        // System instruction (Gemini's equivalent of system prompt)
        let system_instruction = request.system.map(|s| GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart { text: s }],
        });

        let generation_config = Some(GeminiGenerationConfig {
            max_output_tokens: request.max_tokens,
            temperature: request.temperature,
            stop_sequences: request.stop,
        });

        let api_request = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        };

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url(),
            model,
            self.config.api_key
        );

        let response = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::LLM(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<GeminiError>(&body) {
                return Err(Error::LLM(format!(
                    "Gemini API error: {}",
                    error.error.message
                )));
            }
            return Err(Error::LLM(format!(
                "Gemini API error ({}): {}",
                status, body
            )));
        }

        let api_response: GeminiResponse = serde_json::from_str(&body)
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))?;

        let candidate = api_response
            .candidates
            .first()
            .ok_or_else(|| Error::LLM("No candidates in response".to_string()))?;

        let content = candidate
            .content
            .parts
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let stop_reason = candidate.finish_reason.as_deref().map(|r| match r {
            "STOP" => StopReason::EndTurn,
            "MAX_TOKENS" => StopReason::MaxTokens,
            "STOP_SEQUENCE" => StopReason::StopSequence,
            _ => StopReason::EndTurn,
        });

        let usage_metadata = api_response.usage_metadata.unwrap_or(GeminiUsageMetadata {
            prompt_token_count: 0,
            candidates_token_count: Some(0),
            total_token_count: Some(0),
            cached_content_token_count: None,
        });

        let usage = TokenUsage {
            input_tokens: usage_metadata.prompt_token_count,
            output_tokens: usage_metadata.candidates_token_count.unwrap_or(0),
            cache_read_tokens: usage_metadata.cached_content_token_count,
            cache_creation_tokens: None,
        };

        // Calculate cost based on model
        let model_spec = self
            .available_models()
            .into_iter()
            .find(|m| m.id == model || model.contains(&m.id))
            .unwrap_or_else(ModelSpec::gemini_2_0_flash);
        let cost = model_spec.calculate_cost(usage.input_tokens, usage.output_tokens);

        // Generate a unique ID since Gemini doesn't return one
        let id = format!("gemini-{}", Utc::now().timestamp_millis());

        Ok(CompletionResponse {
            id,
            model,
            content,
            stop_reason,
            usage,
            timestamp: Utc::now(),
            cost: Some(cost),
        })
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Gemini has embedding API but using different endpoint
        // For now, return not supported - can be added later
        Err(Error::LLM(
            "Gemini embedding not yet implemented".to_string(),
        ))
    }

    fn provider(&self) -> Provider {
        Provider::Google
    }

    fn available_models(&self) -> Vec<ModelSpec> {
        vec![
            ModelSpec::gemini_2_0_flash(),
            ModelSpec::gemini_1_5_pro(),
            ModelSpec::gemini_1_5_flash(),
        ]
    }
}

/// Multi-provider client that manages multiple LLM providers.
pub struct MultiProviderClient {
    clients: HashMap<Provider, Arc<dyn LLMClient>>,
    default_provider: Provider,
}

impl MultiProviderClient {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            default_provider: Provider::Anthropic,
        }
    }

    /// Add a client for a provider.
    pub fn with_client(mut self, client: Arc<dyn LLMClient>) -> Self {
        let provider = client.provider();
        self.clients.insert(provider, client);
        self
    }

    /// Set the default provider.
    pub fn with_default_provider(mut self, provider: Provider) -> Self {
        self.default_provider = provider;
        self
    }

    /// Get a client for a specific provider.
    pub fn get_client(&self, provider: Provider) -> Option<&Arc<dyn LLMClient>> {
        self.clients.get(&provider)
    }

    /// Get the default client.
    pub fn default_client(&self) -> Option<&Arc<dyn LLMClient>> {
        self.clients.get(&self.default_provider)
    }

    /// Complete using a specific provider.
    pub async fn complete_with(
        &self,
        provider: Provider,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        let client = self
            .clients
            .get(&provider)
            .ok_or_else(|| Error::LLM(format!("No client for provider: {}", provider)))?;
        client.complete(request).await
    }

    /// Complete using the default provider.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.complete_with(self.default_provider, request).await
    }

    /// Create embeddings using a specific provider.
    pub async fn embed_with(
        &self,
        provider: Provider,
        request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse> {
        let client = self
            .clients
            .get(&provider)
            .ok_or_else(|| Error::LLM(format!("No client for provider: {}", provider)))?;
        client.embed(request).await
    }

    /// List all available models across providers.
    pub fn all_models(&self) -> Vec<ModelSpec> {
        self.clients
            .values()
            .flat_map(|c| c.available_models())
            .collect()
    }
}

impl Default for MultiProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe client wrapper with cost tracking.
pub struct TrackedClient {
    inner: Arc<dyn LLMClient>,
    costs: Arc<RwLock<super::types::CostTracker>>,
}

impl TrackedClient {
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self {
            inner: client,
            costs: Arc::new(RwLock::new(super::types::CostTracker::new())),
        }
    }

    /// Complete and track costs.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let response = self.inner.complete(request).await?;

        let mut costs = self.costs.write().await;
        costs.record(&response.model, &response.usage, response.cost);

        Ok(response)
    }

    /// Get current cost summary.
    pub async fn get_costs(&self) -> super::types::CostTracker {
        self.costs.read().await.clone()
    }

    /// Reset cost tracking.
    pub async fn reset_costs(&self) {
        let mut costs = self.costs.write().await;
        *costs = super::types::CostTracker::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("test-key")
            .with_base_url("https://custom.api.com")
            .with_default_model("claude-3-5-haiku")
            .with_timeout(60);

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, Some("https://custom.api.com".to_string()));
        assert_eq!(config.default_model, Some("claude-3-5-haiku".to_string()));
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_multi_provider_client() {
        let client = MultiProviderClient::new().with_default_provider(Provider::OpenAI);

        assert!(client.default_client().is_none()); // No clients added yet
        assert_eq!(client.default_provider, Provider::OpenAI);
    }

    #[test]
    fn test_anthropic_available_models() {
        let client = AnthropicClient::new(ClientConfig::new("test"));
        let models = client.available_models();

        assert_eq!(models.len(), 3);
        assert!(models.iter().any(|m| m.id.contains("opus")));
        assert!(models.iter().any(|m| m.id.contains("sonnet")));
        assert!(models.iter().any(|m| m.id.contains("haiku")));
    }

    #[test]
    fn test_openai_available_models() {
        let client = OpenAIClient::new(ClientConfig::new("test"));
        let models = client.available_models();

        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|m| m.id == "gpt-4o"));
        assert!(models.iter().any(|m| m.id == "gpt-4o-mini"));
    }
}
