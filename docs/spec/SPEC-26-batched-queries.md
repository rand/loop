# SPEC-26: Batched LLM Queries

> Parallel LLM query execution in REPL

**Status**: Draft
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-1d2

---

## Overview

Implement parallel batched LLM queries in the REPL, enabling efficient processing of multiple prompts simultaneously (e.g., for map-reduce patterns over context chunks).

## Requirements

### SPEC-26.01: Batched Query Function

Python interface for batched queries.

```python
def llm_query_batched(
    prompts: list[str],
    contexts: list[str] | None = None,
    max_parallel: int = 5,
    model: str | None = None,
) -> list[str]:
    """
    Execute multiple LLM queries in parallel.

    Args:
        prompts: List of prompts to execute
        contexts: Optional list of contexts (one per prompt)
        max_parallel: Maximum concurrent queries (default 5)
        model: Model to use (default: recursive model from config)

    Returns:
        List of responses in same order as prompts

    Raises:
        BatchedQueryError: If all queries fail

    Example:
        >>> results = llm_query_batched([
        ...     "Summarize section 1",
        ...     "Summarize section 2",
        ...     "Summarize section 3"
        ... ])
        >>> len(results)
        3
    """
```

**Behavior**:
- Queries execute in parallel up to max_parallel
- Results returned in original order
- Individual failures don't abort batch
- Failed queries return error string in result

**Acceptance Criteria**:
- [ ] Function available in REPL sandbox
- [ ] Parallel execution works
- [ ] Order preserved in results

### SPEC-26.02: Rust-Side Implementation

Rust types and handling for batched queries.

```rust
/// Request for batched LLM queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedLLMQuery {
    /// Prompts to execute
    pub prompts: Vec<String>,
    /// Optional contexts (one per prompt, or empty)
    pub contexts: Vec<Option<String>>,
    /// Maximum parallel queries
    pub max_parallel: usize,
    /// Model override (None = use default)
    pub model: Option<String>,
}

/// Result of a single query in batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchQueryResult {
    Success(String),
    Error(String),
}

/// Response from batched query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedLLMResponse {
    /// Results in same order as prompts
    pub results: Vec<BatchQueryResult>,
    /// Total tokens used
    pub total_tokens: TokenUsage,
    /// Total time
    pub total_time_ms: u64,
}

impl ReplHandle {
    /// Execute batched LLM queries
    pub async fn execute_batched_llm(
        &mut self,
        batch: BatchedLLMQuery,
    ) -> Result<BatchedLLMResponse, ReplError> {
        // 1. Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(batch.max_parallel));

        // 2. Spawn tasks for each query
        let tasks: Vec<_> = batch.prompts
            .iter()
            .zip(batch.contexts.iter())
            .enumerate()
            .map(|(i, (prompt, context))| {
                let sem = semaphore.clone();
                let llm = self.llm_client.clone();
                let prompt = prompt.clone();
                let context = context.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let result = llm.complete(&prompt, context.as_deref()).await;
                    (i, result)
                })
            })
            .collect();

        // 3. Collect results in order
        let mut results = vec![BatchQueryResult::Error("Not executed".into()); batch.prompts.len()];
        for task in tasks {
            let (i, result) = task.await?;
            results[i] = match result {
                Ok(response) => BatchQueryResult::Success(response),
                Err(e) => BatchQueryResult::Error(e.to_string()),
            };
        }

        Ok(BatchedLLMResponse {
            results,
            total_tokens: self.aggregate_tokens(),
            total_time_ms: elapsed.as_millis() as u64,
        })
    }
}
```

**Acceptance Criteria**:
- [ ] Semaphore controls concurrency
- [ ] Results collected in order
- [ ] Errors captured per-query

### SPEC-26.03: Concurrency Control

Configuration and rate limiting.

```rust
/// Configuration for batched queries
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Default maximum parallel queries
    pub default_max_parallel: usize,
    /// Maximum allowed max_parallel (hard limit)
    pub max_allowed_parallel: usize,
    /// Rate limit per provider (requests per minute)
    pub provider_rate_limits: HashMap<String, u32>,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            default_max_parallel: 5,
            max_allowed_parallel: 20,
            provider_rate_limits: HashMap::from([
                ("anthropic".into(), 60),
                ("openai".into(), 60),
                ("openrouter".into(), 100),
            ]),
            retry_config: RetryConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retries per query
    pub max_retries: u32,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Exponential backoff factor
    pub backoff_factor: f64,
}
```

**Rate Limiting**:
- Respect per-provider rate limits
- Backoff on rate limit errors
- Distribute requests over time if needed

**Acceptance Criteria**:
- [ ] Rate limits respected
- [ ] Exponential backoff on errors
- [ ] Hard limit on parallelism

### SPEC-26.04: Error Handling

Graceful handling of partial failures.

```rust
/// Error types for batched queries
#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    #[error("All {total} queries failed")]
    AllFailed { total: usize },

    #[error("Rate limited by provider: {provider}")]
    RateLimited { provider: String, retry_after: Option<Duration> },

    #[error("Batch too large: {size} > {max}")]
    BatchTooLarge { size: usize, max: usize },
}

impl BatchedLLMResponse {
    /// Check if all queries succeeded
    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| matches!(r, BatchQueryResult::Success(_)))
    }

    /// Get successful results only
    pub fn successes(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter_map(|r| match r {
                BatchQueryResult::Success(s) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r, BatchQueryResult::Error(_)))
            .count()
    }

    /// Get detailed error report
    pub fn error_report(&self) -> Vec<(usize, &str)> {
        self.results
            .iter()
            .enumerate()
            .filter_map(|(i, r)| match r {
                BatchQueryResult::Error(e) => Some((i, e.as_str())),
                _ => None,
            })
            .collect()
    }
}
```

**Acceptance Criteria**:
- [ ] Partial results available
- [ ] Error details preserved
- [ ] Success/failure counts accurate

---

## Usage Examples

### Basic Batched Query

```python
# Summarize multiple sections
sections = [context[i:i+1000] for i in range(0, len(context), 1000)]
prompts = [f"Summarize: {s}" for s in sections]
summaries = llm_query_batched(prompts)
```

### Map-Reduce Pattern

```python
# Map: Extract facts from each file
facts_prompts = [f"Extract facts from:\n{content}" for content in files.values()]
facts_list = llm_query_batched(facts_prompts, max_parallel=10)

# Reduce: Combine facts
combined = "\n".join(facts_list)
final = llm_query(f"Synthesize these facts:\n{combined}")
```

### With Error Handling

```python
results = llm_query_batched(prompts)
errors = [(i, r) for i, r in enumerate(results) if r.startswith("[Error]")]
if errors:
    print(f"Warning: {len(errors)} queries failed")
    for i, err in errors:
        print(f"  Query {i}: {err}")
```

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_batch_basic` | Basic batched query | SPEC-26.01 |
| `test_batch_order` | Results in order | SPEC-26.01 |
| `test_batch_parallel` | Parallel execution | SPEC-26.02 |
| `test_batch_semaphore` | Concurrency limit | SPEC-26.03 |
| `test_batch_partial_fail` | Partial failure | SPEC-26.04 |
| `test_batch_all_fail` | All queries fail | SPEC-26.04 |
| `test_batch_rate_limit` | Rate limiting | SPEC-26.03 |

---

## References

- [DSPy llm_query_batched](https://github.com/stanfordnlp/dspy/blob/main/dspy/predict/rlm.py)
- Existing REPL: `src/repl.rs`
- Existing LLM Client: `src/llm/client.rs`
