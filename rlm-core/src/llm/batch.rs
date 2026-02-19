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

use std::sync::Arc;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use super::types::{ChatMessage, CompletionRequest};
use super::LLMClient;
use crate::error::Result;

/// Default maximum parallel queries.
pub const DEFAULT_MAX_PARALLEL: usize = 5;

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
        let total_tokens = results
            .iter()
            .filter_map(|r| r.tokens_used)
            .sum();

        Self {
            results,
            success_count,
            failure_count,
            total_tokens,
        }
    }

    /// Get successful responses in order.
    pub fn responses(&self) -> Vec<Option<&str>> {
        self.results
            .iter()
            .map(|r| r.response.as_deref())
            .collect()
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
}

impl<C: LLMClient + 'static> BatchExecutor<C> {
    /// Create a new batch executor.
    pub fn new(client: C) -> Self {
        Self {
            client: Arc::new(client),
            max_parallel: DEFAULT_MAX_PARALLEL,
        }
    }

    /// Create from an Arc'd client.
    pub fn from_arc(client: Arc<C>) -> Self {
        Self {
            client,
            max_parallel: DEFAULT_MAX_PARALLEL,
        }
    }

    /// Set the maximum parallel queries.
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = max.max(1);
        self
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

        // Create tasks for each prompt
        let tasks: Vec<_> = batch
            .prompts
            .into_iter()
            .enumerate()
            .map(|(index, prompt)| {
                let client = Arc::clone(&self.client);
                let semaphore = Arc::clone(&semaphore);
                let context = batch
                    .contexts
                    .get(index)
                    .cloned()
                    .flatten();
                let model = batch.model.clone();
                let temperature = batch.temperature;
                let max_tokens = batch.max_tokens;

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

                    // Execute query
                    match client.complete(request).await {
                        Ok(response) => {
                            let text = response.content.clone();
                            let tokens = Some(response.usage.total() as u32);
                            BatchQueryResult::success(index, text, tokens)
                        }
                        Err(e) => {
                            BatchQueryResult::failure(index, e.to_string())
                        }
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
    /// Whether to retry failed queries once.
    pub retry_failures: bool,
    /// Timeout per query in milliseconds.
    pub query_timeout_ms: Option<u64>,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_parallel: DEFAULT_MAX_PARALLEL,
            retry_failures: false,
            query_timeout_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let prompts = vec![
            "Query 1".to_string(),
            "Query 2".to_string(),
        ];
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
        let batch = BatchedLLMQuery::new()
            .with_max_parallel(0); // Should be clamped to 1

        assert_eq!(batch.max_parallel, 1);
    }
}
