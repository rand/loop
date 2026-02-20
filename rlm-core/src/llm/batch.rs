//! Batched LLM query execution with concurrency control.
//!
//! This module provides parallel batch execution of LLM queries with:
//! - Configurable concurrency limits (SPEC-26.03)
//! - Graceful error handling for partial failures (SPEC-26.04)
//! - Order-preserving result collection
//!
//! # Example
//!
//! ```rust,ignore
//! use rlm_core::llm::{BatchedLLMQuery, BatchExecutor, AnthropicClient};
//!
//! let client = AnthropicClient::new(config);
//! let executor = BatchExecutor::new(client)
//!     .with_max_parallel(5);
//!
//! let batch = BatchedLLMQuery::new()
//!     .add_prompt("Summarize section 1")
//!     .add_prompt("Summarize section 2")
//!     .add_prompt("Summarize section 3");
//!
//! let results = executor.execute(batch).await?;
//! // Results are in original order, with errors for failed queries
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

use super::types::{ChatMessage, CompletionRequest, Provider};
use super::LLMClient;
use crate::error::{Error, Result};

/// Default maximum parallel queries.
pub const DEFAULT_MAX_PARALLEL: usize = 5;
/// Default rate-limit window for provider throttling.
pub const DEFAULT_RATE_LIMIT_WINDOW_MS: u64 = 60_000;

fn default_provider_rate_limits() -> HashMap<Provider, u32> {
    #[allow(unused_mut)]
    let mut limits = HashMap::from([
        (Provider::Anthropic, 60),
        (Provider::OpenAI, 60),
        (Provider::OpenRouter, 100),
    ]);
    #[cfg(feature = "gemini")]
    limits.insert(Provider::Google, 60);
    limits
}

#[derive(Debug, Clone, Copy)]
struct ProviderWindowState {
    window_start: Instant,
    used: u32,
}

#[derive(Debug)]
struct ProviderRateLimiter {
    limits: HashMap<Provider, u32>,
    window: Duration,
    state: Mutex<HashMap<Provider, ProviderWindowState>>,
}

impl ProviderRateLimiter {
    fn new(limits: HashMap<Provider, u32>, window: Duration) -> Self {
        Self {
            limits,
            window,
            state: Mutex::new(HashMap::new()),
        }
    }

    async fn acquire(&self, provider: Provider) {
        let limit = match self.limits.get(&provider).copied() {
            Some(limit) if limit > 0 => limit,
            _ => return,
        };

        loop {
            let wait = {
                let mut state = self.state.lock().await;
                let entry = state.entry(provider).or_insert(ProviderWindowState {
                    window_start: Instant::now(),
                    used: 0,
                });

                let elapsed = entry.window_start.elapsed();
                if elapsed >= self.window {
                    entry.window_start = Instant::now();
                    entry.used = 0;
                }

                if entry.used < limit {
                    entry.used += 1;
                    None
                } else {
                    Some(self.window.saturating_sub(elapsed))
                }
            };

            if let Some(wait) = wait {
                sleep(wait).await;
            } else {
                break;
            }
        }
    }
}

/// Retry configuration for batched requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retries for a single query.
    pub max_retries: u32,
    /// Base delay used for exponential backoff.
    pub base_delay_ms: u64,
    /// Backoff multiplier applied per retry attempt.
    pub backoff_factor: f64,
}

impl RetryConfig {
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let factor = self.backoff_factor.max(1.0).powi(attempt as i32);
        let millis = (self.base_delay_ms as f64 * factor).round().max(0.0) as u64;
        Duration::from_millis(millis)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 200,
            backoff_factor: 2.0,
        }
    }
}

/// A batched LLM query request (SPEC-26.01, SPEC-26.02).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedLLMQuery {
    /// Prompts to execute.
    pub prompts: Vec<String>,
    /// Optional contexts for each prompt (same length as prompts or empty).
    pub contexts: Vec<Option<String>>,
    /// Maximum parallel queries (default: 5).
    pub max_parallel: usize,
    /// Model to use for all queries (optional).
    pub model: Option<String>,
    /// Temperature for all queries (optional).
    pub temperature: Option<f64>,
    /// Max tokens for each query (optional).
    pub max_tokens: Option<u32>,
}

impl BatchedLLMQuery {
    /// Create a new empty batch query.
    pub fn new() -> Self {
        Self {
            prompts: Vec::new(),
            contexts: Vec::new(),
            max_parallel: DEFAULT_MAX_PARALLEL,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Create from a list of prompts.
    pub fn from_prompts(prompts: Vec<String>) -> Self {
        Self {
            prompts,
            contexts: Vec::new(),
            max_parallel: DEFAULT_MAX_PARALLEL,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Add a prompt to the batch.
    pub fn add_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompts.push(prompt.into());
        self
    }

    /// Add a prompt with context.
    pub fn add_prompt_with_context(
        mut self,
        prompt: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        // Pad contexts to match prompts length
        while self.contexts.len() < self.prompts.len() {
            self.contexts.push(None);
        }
        self.prompts.push(prompt.into());
        self.contexts.push(Some(context.into()));
        self
    }

    /// Set contexts for all prompts.
    pub fn with_contexts(mut self, contexts: Vec<Option<String>>) -> Self {
        self.contexts = contexts;
        self
    }

    /// Set maximum parallel queries.
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = max.max(1); // At least 1
        self
    }

    /// Set the model for all queries.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature for all queries.
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens for each query.
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }

    /// Get the number of queries in the batch.
    pub fn len(&self) -> usize {
        self.prompts.len()
    }
}

impl Default for BatchedLLMQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single query in a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQueryResult {
    /// Index of this query in the original batch.
    pub index: usize,
    /// Whether the query succeeded.
    pub success: bool,
    /// Response text (if successful).
    pub response: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Token usage for this query.
    pub tokens_used: Option<u32>,
}

impl BatchQueryResult {
    /// Create a successful result.
    pub fn success(index: usize, response: String, tokens: Option<u32>) -> Self {
        Self {
            index,
            success: true,
            response: Some(response),
            error: None,
            tokens_used: tokens,
        }
    }

    /// Create a failed result.
    pub fn failure(index: usize, error: String) -> Self {
        Self {
            index,
            success: false,
            response: None,
            error: Some(error),
            tokens_used: None,
        }
    }
}

/// Results of a batched query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedQueryResults {
    /// Results in original order.
    pub results: Vec<BatchQueryResult>,
    /// Number of successful queries.
    pub success_count: usize,
    /// Number of failed queries.
    pub failure_count: usize,
    /// Total tokens used.
    pub total_tokens: u32,
}

impl BatchedQueryResults {
    /// Create from a list of results.
    pub fn from_results(mut results: Vec<BatchQueryResult>) -> Self {
        // Sort by index to ensure original order
        results.sort_by_key(|r| r.index);

        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;
        let total_tokens = results.iter().filter_map(|r| r.tokens_used).sum();

        Self {
            results,
            success_count,
            failure_count,
            total_tokens,
        }
    }

    /// Get successful responses in order.
    pub fn responses(&self) -> Vec<Option<&str>> {
        self.results.iter().map(|r| r.response.as_deref()).collect()
    }

    /// Check if all queries succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.failure_count == 0
    }

    /// Get only the successful responses.
    pub fn successful_responses(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| r.success)
            .filter_map(|r| r.response.as_deref())
            .collect()
    }

    /// Get error messages for failed queries.
    pub fn errors(&self) -> Vec<(usize, &str)> {
        self.results
            .iter()
            .filter(|r| !r.success)
            .filter_map(|r| r.error.as_deref().map(|e| (r.index, e)))
            .collect()
    }
}

/// Executor for batched LLM queries (SPEC-26.02, SPEC-26.03).
///
/// Uses a semaphore for concurrency control to respect rate limits.
pub struct BatchExecutor<C: LLMClient> {
    client: Arc<C>,
    max_parallel: usize,
    retry_config: RetryConfig,
    retry_failures: bool,
    provider_rate_limits: HashMap<Provider, u32>,
    rate_limit_window: Duration,
}

impl<C: LLMClient + 'static> BatchExecutor<C> {
    /// Create a new batch executor.
    pub fn new(client: C) -> Self {
        Self {
            client: Arc::new(client),
            max_parallel: DEFAULT_MAX_PARALLEL,
            retry_config: RetryConfig::default(),
            retry_failures: true,
            provider_rate_limits: default_provider_rate_limits(),
            rate_limit_window: Duration::from_millis(DEFAULT_RATE_LIMIT_WINDOW_MS),
        }
    }

    /// Create from an Arc'd client.
    pub fn from_arc(client: Arc<C>) -> Self {
        Self {
            client,
            max_parallel: DEFAULT_MAX_PARALLEL,
            retry_config: RetryConfig::default(),
            retry_failures: true,
            provider_rate_limits: default_provider_rate_limits(),
            rate_limit_window: Duration::from_millis(DEFAULT_RATE_LIMIT_WINDOW_MS),
        }
    }

    /// Set the maximum parallel queries.
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = max.max(1);
        self
    }

    /// Set retry policy for retryable failures.
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Enable or disable retry behavior.
    pub fn with_retry_failures(mut self, retry_failures: bool) -> Self {
        self.retry_failures = retry_failures;
        self
    }

    /// Override the configured rate limit for one provider.
    pub fn with_provider_rate_limit(
        mut self,
        provider: Provider,
        requests_per_minute: u32,
    ) -> Self {
        self.provider_rate_limits
            .insert(provider, requests_per_minute);
        self
    }

    /// Set the rate-limit window duration.
    ///
    /// The default is one minute. This is primarily useful for tests.
    pub fn with_rate_limit_window(mut self, window: Duration) -> Self {
        self.rate_limit_window = window;
        self
    }

    /// Apply a complete batch configuration.
    pub fn with_config(mut self, config: BatchConfig) -> Self {
        self.max_parallel = config.max_parallel.max(1);
        self.retry_failures = config.retry_failures;
        self.retry_config = config.retry_config;
        self.provider_rate_limits = config.provider_rate_limits;
        self.rate_limit_window = Duration::from_millis(config.rate_limit_window_ms.max(1));
        self
    }

    fn is_retryable_error(error: &Error) -> bool {
        match error {
            Error::Timeout { .. } => true,
            Error::LLM(message) => Self::is_retryable_message(message),
            Error::LlmApi { message, .. } => Self::is_retryable_message(message),
            _ => false,
        }
    }

    fn is_retryable_message(message: &str) -> bool {
        let lower = message.to_ascii_lowercase();
        lower.contains("429")
            || lower.contains("rate limit")
            || lower.contains("rate_limit")
            || lower.contains("too many requests")
            || lower.contains("temporarily unavailable")
            || lower.contains("timeout")
    }

    async fn complete_with_retry(
        client: Arc<C>,
        request: CompletionRequest,
        retry_config: RetryConfig,
        retry_failures: bool,
    ) -> Result<super::types::CompletionResponse> {
        let mut attempt = 0;
        loop {
            match client.complete(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = retry_failures
                        && attempt < retry_config.max_retries
                        && Self::is_retryable_error(&error);
                    if !should_retry {
                        return Err(error);
                    }

                    let delay = retry_config.delay_for_attempt(attempt);
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Execute a batched query with concurrency control (SPEC-26.03, SPEC-26.04).
    ///
    /// Returns results in the original order. Failed queries don't abort the batch.
    pub async fn execute(&self, batch: BatchedLLMQuery) -> Result<BatchedQueryResults> {
        if batch.is_empty() {
            return Ok(BatchedQueryResults::from_results(Vec::new()));
        }

        // Use the smaller of batch config and executor config for max parallel
        let max_parallel = batch.max_parallel.min(self.max_parallel);
        let semaphore = Arc::new(Semaphore::new(max_parallel));
        let provider = self.client.provider();
        let rate_limiter = Arc::new(ProviderRateLimiter::new(
            self.provider_rate_limits.clone(),
            self.rate_limit_window,
        ));

        // Create tasks for each prompt
        let tasks: Vec<_> = batch
            .prompts
            .into_iter()
            .enumerate()
            .map(|(index, prompt)| {
                let client = Arc::clone(&self.client);
                let semaphore = Arc::clone(&semaphore);
                let context = batch.contexts.get(index).cloned().flatten();
                let model = batch.model.clone();
                let temperature = batch.temperature;
                let max_tokens = batch.max_tokens;
                let provider = provider;
                let rate_limiter = Arc::clone(&rate_limiter);
                let retry_config = self.retry_config.clone();
                let retry_failures = self.retry_failures;

                async move {
                    // Acquire semaphore permit
                    let _permit = semaphore
                        .acquire()
                        .await
                        .expect("Semaphore closed unexpectedly");

                    // Build request
                    let mut request = CompletionRequest::new();

                    if let Some(ref model) = model {
                        request = request.with_model(model);
                    }
                    if let Some(temp) = temperature {
                        request = request.with_temperature(temp);
                    }
                    if let Some(tokens) = max_tokens {
                        request = request.with_max_tokens(tokens);
                    }

                    // Add context as system message if provided
                    if let Some(ctx) = context {
                        request = request.with_message(ChatMessage::system(ctx));
                    }

                    // Add the prompt
                    request = request.with_message(ChatMessage::user(&prompt));

                    // Respect provider-specific rate-limit policy before calling the provider.
                    rate_limiter.acquire(provider).await;

                    // Execute query with bounded exponential-backoff retries.
                    match Self::complete_with_retry(
                        Arc::clone(&client),
                        request,
                        retry_config,
                        retry_failures,
                    )
                    .await
                    {
                        Ok(response) => {
                            let text = response.content.clone();
                            let tokens = Some(response.usage.total() as u32);
                            BatchQueryResult::success(index, text, tokens)
                        }
                        Err(e) => BatchQueryResult::failure(index, e.to_string()),
                    }
                }
            })
            .collect();

        // Execute all tasks concurrently (with semaphore limiting parallelism)
        let results = join_all(tasks).await;

        Ok(BatchedQueryResults::from_results(results))
    }
}

/// Configuration for batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum parallel queries (default: 5).
    pub max_parallel: usize,
    /// Whether retry behavior is enabled for retryable provider failures.
    pub retry_failures: bool,
    /// Timeout per query in milliseconds.
    pub query_timeout_ms: Option<u64>,
    /// Provider-specific requests-per-minute budget.
    pub provider_rate_limits: HashMap<Provider, u32>,
    /// Exponential backoff retry policy.
    pub retry_config: RetryConfig,
    /// Window duration used by provider rate limiting.
    pub rate_limit_window_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_parallel: DEFAULT_MAX_PARALLEL,
            retry_failures: true,
            query_timeout_ms: None,
            provider_rate_limits: default_provider_rate_limits(),
            retry_config: RetryConfig::default(),
            rate_limit_window_ms: DEFAULT_RATE_LIMIT_WINDOW_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    use async_trait::async_trait;

    use super::*;
    use crate::llm::{
        CompletionResponse, EmbeddingRequest, EmbeddingResponse, ModelSpec, TokenUsage,
    };

    #[test]
    fn test_batched_query_creation() {
        let batch = BatchedLLMQuery::new()
            .add_prompt("Query 1")
            .add_prompt("Query 2")
            .add_prompt("Query 3");

        assert_eq!(batch.len(), 3);
        assert!(!batch.is_empty());
        assert_eq!(batch.prompts[0], "Query 1");
    }

    #[test]
    fn test_batched_query_with_context() {
        let batch = BatchedLLMQuery::new()
            .add_prompt_with_context("Query 1", "Context 1")
            .add_prompt("Query 2");

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.contexts.len(), 1);
        assert_eq!(batch.contexts[0], Some("Context 1".to_string()));
        assert!(batch.contexts.get(1).is_none());
    }

    #[test]
    fn test_batched_query_from_prompts() {
        let prompts = vec!["Query 1".to_string(), "Query 2".to_string()];
        let batch = BatchedLLMQuery::from_prompts(prompts)
            .with_max_parallel(3)
            .with_model("claude-3-5-sonnet-20241022");

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.max_parallel, 3);
        assert_eq!(batch.model, Some("claude-3-5-sonnet-20241022".to_string()));
    }

    #[test]
    fn test_batch_query_result_success() {
        let result = BatchQueryResult::success(0, "Response text".to_string(), Some(100));

        assert!(result.success);
        assert_eq!(result.response, Some("Response text".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.tokens_used, Some(100));
    }

    #[test]
    fn test_batch_query_result_failure() {
        let result = BatchQueryResult::failure(1, "Rate limited".to_string());

        assert!(!result.success);
        assert!(result.response.is_none());
        assert_eq!(result.error, Some("Rate limited".to_string()));
    }

    #[test]
    fn test_batched_results_ordering() {
        let results = vec![
            BatchQueryResult::success(2, "Third".to_string(), Some(30)),
            BatchQueryResult::success(0, "First".to_string(), Some(10)),
            BatchQueryResult::failure(1, "Failed".to_string()),
        ];

        let batched = BatchedQueryResults::from_results(results);

        // Results should be sorted by index
        assert_eq!(batched.results[0].index, 0);
        assert_eq!(batched.results[1].index, 1);
        assert_eq!(batched.results[2].index, 2);

        assert_eq!(batched.success_count, 2);
        assert_eq!(batched.failure_count, 1);
        assert_eq!(batched.total_tokens, 40);
    }

    #[test]
    fn test_batched_results_responses() {
        let results = vec![
            BatchQueryResult::success(0, "First".to_string(), None),
            BatchQueryResult::failure(1, "Error".to_string()),
            BatchQueryResult::success(2, "Third".to_string(), None),
        ];

        let batched = BatchedQueryResults::from_results(results);

        let responses = batched.responses();
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0], Some("First"));
        assert_eq!(responses[1], None);
        assert_eq!(responses[2], Some("Third"));

        let successful = batched.successful_responses();
        assert_eq!(successful.len(), 2);
        assert_eq!(successful[0], "First");
        assert_eq!(successful[1], "Third");
    }

    #[test]
    fn test_batched_results_errors() {
        let results = vec![
            BatchQueryResult::success(0, "OK".to_string(), None),
            BatchQueryResult::failure(1, "Rate limit".to_string()),
            BatchQueryResult::failure(2, "Timeout".to_string()),
        ];

        let batched = BatchedQueryResults::from_results(results);
        let errors = batched.errors();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], (1, "Rate limit"));
        assert_eq!(errors[1], (2, "Timeout"));

        assert!(!batched.all_succeeded());
    }

    #[test]
    fn test_max_parallel_bounds() {
        let batch = BatchedLLMQuery::new().with_max_parallel(0); // Should be clamped to 1

        assert_eq!(batch.max_parallel, 1);
    }

    struct FlakyBatchClient {
        provider: Provider,
        fail_until: usize,
        calls: Arc<AtomicUsize>,
        call_times: Arc<Mutex<Vec<Instant>>>,
    }

    impl FlakyBatchClient {
        fn new(provider: Provider, fail_until: usize) -> Self {
            Self {
                provider,
                fail_until,
                calls: Arc::new(AtomicUsize::new(0)),
                call_times: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl LLMClient for FlakyBatchClient {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let mut call_times = self.call_times.lock().await;
            call_times.push(Instant::now());
            drop(call_times);

            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call <= self.fail_until {
                return Err(Error::LLM("429 rate limit exceeded".to_string()));
            }

            Ok(CompletionResponse {
                id: format!("mock-{call}"),
                model: request.model.unwrap_or_else(|| "mock-model".to_string()),
                content: "ok".to_string(),
                stop_reason: None,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_read_tokens: None,
                    cache_creation_tokens: None,
                },
                timestamp: chrono::Utc::now(),
                cost: Some(0.0),
            })
        }

        async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            Err(Error::LLM(
                "embedding not implemented in test mock".to_string(),
            ))
        }

        fn provider(&self) -> Provider {
            self.provider
        }

        fn available_models(&self) -> Vec<ModelSpec> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_retry_with_exponential_backoff_for_rate_limited_query() {
        let client = FlakyBatchClient::new(Provider::OpenAI, 2);
        let calls = Arc::clone(&client.calls);
        let executor = BatchExecutor::new(client).with_retry_config(RetryConfig {
            max_retries: 2,
            base_delay_ms: 1,
            backoff_factor: 2.0,
        });

        let results = executor
            .execute(BatchedLLMQuery::new().add_prompt("q1"))
            .await
            .expect("batch execution should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(results.success_count, 1);
        assert_eq!(results.failure_count, 0);
    }

    #[tokio::test]
    async fn test_retry_stops_after_bounded_attempts() {
        let client = FlakyBatchClient::new(Provider::Anthropic, usize::MAX);
        let calls = Arc::clone(&client.calls);
        let executor = BatchExecutor::new(client)
            .with_retry_config(RetryConfig {
                max_retries: 1,
                base_delay_ms: 1,
                backoff_factor: 2.0,
            })
            .with_retry_failures(true);

        let results = executor
            .execute(BatchedLLMQuery::new().add_prompt("q1"))
            .await
            .expect("batch execution should return partial results");

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(results.success_count, 0);
        assert_eq!(results.failure_count, 1);
        assert!(results.results[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("429"));
    }

    #[tokio::test]
    async fn test_provider_aware_rate_limit_is_enforced() {
        let client = FlakyBatchClient::new(Provider::OpenAI, 0);
        let call_times = Arc::clone(&client.call_times);
        let executor = BatchExecutor::new(client)
            .with_provider_rate_limit(Provider::OpenAI, 1)
            .with_rate_limit_window(Duration::from_millis(20))
            .with_max_parallel(2)
            .with_retry_failures(false);

        let started = Instant::now();
        let results = executor
            .execute(
                BatchedLLMQuery::new()
                    .add_prompt("q1")
                    .add_prompt("q2")
                    .with_max_parallel(2),
            )
            .await
            .expect("batch execution should succeed");
        let elapsed = started.elapsed();

        let call_times = call_times.lock().await;
        assert_eq!(results.success_count, 2);
        assert_eq!(call_times.len(), 2);
        assert!(elapsed >= Duration::from_millis(15));
    }
}
