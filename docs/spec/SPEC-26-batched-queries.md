# SPEC-26: Batched LLM Queries

> Parallel LLM query execution in REPL

**Status**: Implemented in `rlm-core` runtime (`BatchExecutor` provider-aware rate-limit + exponential backoff retry policy, plus REPL-host integration)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-1d2

---

## Overview

Implement parallel batched LLM queries in the REPL, enabling efficient processing of multiple prompts simultaneously (e.g., for map-reduce patterns over context chunks).

## Implementation Snapshot (2026-02-20)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-26.01 Batched helper naming/shape | Implemented | `rlm-core/python/rlm_repl/helpers.py`, `rlm-core/python/rlm_repl/sandbox.py` |
| SPEC-26.02 Rust batch execution primitives | Implemented | `rlm-core/src/llm/batch.rs` |
| SPEC-26.03 Advanced rate-limit/retry policy | Implemented | Provider-aware limits + bounded exponential backoff in `rlm-core/src/llm/batch.rs` (`BatchExecutor`, `RetryConfig`) |
| SPEC-26.04 Partial-failure result handling | Implemented (data model level) | `BatchQueryResult`, `BatchedQueryResults` helpers + tests in `rlm-core/src/llm/batch.rs` |
| REPL-host orchestration wiring (`M7-T01`) | Implemented | `ReplHandle::resolve_pending_llm_batches` + pending-operation protocol wiring in `rlm-core/src/repl.rs` and `rlm-core/python/rlm_repl/main.py` |

## Requirements

### SPEC-26.01: Batched Query Function

Python interface for batched queries.

```python
def llm_batch(
    prompts: list[str],
    contexts: list[str] | None = None,
    max_parallel: int = 5,
    model: str | None = None,
) -> DeferredOperation:
    """
    Execute multiple LLM queries in parallel.

    Args:
        prompts: List of prompts to execute
        contexts: Optional list of contexts (one per prompt)
        max_parallel: Maximum concurrent queries (default 5)
        model: Model to use (default: recursive model from config)

    Returns:
        DeferredOperation that resolves to batched query results

    Raises:
        BatchedQueryError: If all queries fail

    Example:
        >>> results = llm_batch([
        ...     "Summarize section 1",
        ...     "Summarize section 2",
        ...     "Summarize section 3"
        ... ])
        >>> len(results)
        3
    """
```

Compatibility policy:
- Canonical REPL helper: `llm_batch`
- Compatibility alias: `llm_query_batched` (deprecated; emits warning)

**Behavior**:
- Helper emits deferred operation type `llm_batch`
- `max_parallel` is passed through in operation params
- Canonical helper name is `llm_batch`; alias `llm_query_batched` is supported

**Acceptance Criteria**:
- [x] Function available in REPL sandbox
- [x] Compatibility alias available during migration window
- [x] End-to-end REPL-host execution path for `LLM_BATCH` validated in integration tests

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
- [x] Semaphore controls concurrency
- [x] Results collected in order
- [x] Errors captured per-query
- [x] Direct `ReplHandle` orchestration integration implemented (`resolve_pending_llm_batches`)

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
- [x] Rate limits respected
- [x] Exponential backoff on retryable errors
- [x] Basic max-parallel clamping present

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
- [x] Partial results available
- [x] Error details preserved
- [x] Success/failure counts accurate

---

## Usage Examples

### Basic Batched Query

```python
# Summarize multiple sections
sections = [context[i:i+1000] for i in range(0, len(context), 1000)]
prompts = [f"Summarize: {s}" for s in sections]
summaries = llm_batch(prompts)
```

### Map-Reduce Pattern

```python
# Map: Extract facts from each file
facts_prompts = [f"Extract facts from:\n{content}" for content in files.values()]
facts_list = llm_batch(facts_prompts, max_parallel=10)

# Reduce: Combine facts
combined = "\n".join(facts_list)
final = llm_query(f"Synthesize these facts:\n{combined}")
```

### With Error Handling

```python
results = llm_batch(prompts)
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
| `llm::batch::tests::test_batched_query_creation` | Batched query construction | SPEC-26.01 |
| `llm::batch::tests::test_batched_query_with_context` | Context shape behavior | SPEC-26.01 |
| `llm::batch::tests::test_batched_results_ordering` | Results preserved in index order | SPEC-26.02 |
| `llm::batch::tests::test_batched_results_errors` | Partial-failure/error detail handling | SPEC-26.04 |
| `llm::batch::tests::test_max_parallel_bounds` | Basic parallelism clamping | SPEC-26.03 |
| `llm::batch::tests::test_provider_aware_rate_limit_is_enforced` | Provider-specific throttle window enforcement | SPEC-26.03 |
| `llm::batch::tests::test_retry_with_exponential_backoff_for_rate_limited_query` | Bounded exponential-backoff retry on retryable provider failure | SPEC-26.03 |
| `llm::batch::tests::test_retry_stops_after_bounded_attempts` | Retry ceiling enforcement on persistent failures | SPEC-26.03 |
| `repl::tests::test_llm_batch_operation_to_query` | Rust host parses deferred `llm_batch` operation params into batch query primitives | SPEC-26.02 |
| `repl::tests::test_llm_batch_results_payload_mixed_success_failure` | Rust host preserves mixed success/failure payload semantics when resolving deferred batch ops | SPEC-26.04 |
| `repl::tests::test_llm_batch_host_resolution_roundtrip` (ignored integration) + `python/tests/test_repl.py::test_llm_batch_mixed_success_failure_resolution` | End-to-end REPL-host batch execution and resolution path | SPEC-26.01, SPEC-26.02 |

---

## References

- [DSPy llm_query_batched](https://github.com/stanfordnlp/dspy/blob/main/dspy/predict/rlm.py) (naming inspiration)
- Existing REPL bridge: `rlm-core/src/repl.rs`
- Existing batch primitives: `rlm-core/src/llm/batch.rs`
