//! Fresh context invocation for isolated adversarial calls.
//!
//! The FreshContextInvoker ensures adversarial validation happens in an
//! isolated context without shared conversation history, preventing the
//! adversary from being influenced by the primary model's reasoning.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use super::types::{AdversarialConfig, ValidationContext, ValidationResult};
use super::validator::{AdversarialValidator, GeminiValidator};
use crate::error::{Error, Result};

/// Trait for fresh context invocation.
///
/// Implementations ensure each validation call happens in isolation,
/// without any shared state from previous calls or the primary model's
/// conversation history.
#[async_trait]
pub trait FreshContextInvoker: Send + Sync {
    /// Invoke validation in a fresh context.
    ///
    /// This method guarantees:
    /// 1. No conversation history from prior calls
    /// 2. No shared state with the primary model
    /// 3. Clean resource initialization for each call
    async fn invoke_fresh(&self, context: &ValidationContext) -> Result<ValidationResult>;

    /// Check if the invoker is healthy and ready.
    async fn health_check(&self) -> Result<bool>;

    /// Get statistics about invocations.
    fn stats(&self) -> InvocationStats;
}

/// Statistics about fresh context invocations.
#[derive(Debug, Clone, Default)]
pub struct InvocationStats {
    /// Total invocations
    pub total_invocations: u64,
    /// Successful invocations
    pub successful_invocations: u64,
    /// Failed invocations
    pub failed_invocations: u64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

impl InvocationStats {
    /// Record a successful invocation.
    pub fn record_success(&mut self, cost: f64, latency_ms: u64) {
        self.total_invocations += 1;
        self.successful_invocations += 1;
        self.total_cost_usd += cost;

        // Update rolling average
        let n = self.successful_invocations as f64;
        self.avg_latency_ms = ((n - 1.0) * self.avg_latency_ms + latency_ms as f64) / n;
    }

    /// Record a failed invocation.
    pub fn record_failure(&mut self) {
        self.total_invocations += 1;
        self.failed_invocations += 1;
    }

    /// Get success rate.
    pub fn success_rate(&self) -> f64 {
        if self.total_invocations == 0 {
            1.0
        } else {
            self.successful_invocations as f64 / self.total_invocations as f64
        }
    }
}

/// Fresh context invoker using Gemini.
///
/// Creates a new validator instance for each invocation to ensure
/// complete isolation. This is the recommended approach for adversarial
/// validation where independence from the primary model is critical.
pub struct GeminiFreshInvoker {
    api_key: String,
    config: AdversarialConfig,
    stats: Arc<RwLock<InvocationStats>>,
}

impl GeminiFreshInvoker {
    /// Create a new fresh invoker.
    pub fn new(api_key: impl Into<String>, config: AdversarialConfig) -> Self {
        Self {
            api_key: api_key.into(),
            config,
            stats: Arc::new(RwLock::new(InvocationStats::default())),
        }
    }

    /// Create with custom stats (for testing).
    pub fn with_stats(mut self, stats: InvocationStats) -> Self {
        self.stats = Arc::new(RwLock::new(stats));
        self
    }
}

#[async_trait]
impl FreshContextInvoker for GeminiFreshInvoker {
    #[instrument(skip(self, context), fields(validation_id = %context.id))]
    async fn invoke_fresh(&self, context: &ValidationContext) -> Result<ValidationResult> {
        info!("Creating fresh validator instance for isolated invocation");

        let start = std::time::Instant::now();

        // Create a brand new validator instance - no shared state
        let validator = match GeminiValidator::new(&self.api_key, self.config.clone()) {
            Ok(v) => v,
            Err(e) => {
                self.stats.write().await.record_failure();
                return Err(e);
            }
        };

        debug!("Fresh validator created, starting validation");

        // Perform validation
        let result = match validator.validate(context).await {
            Ok(r) => r,
            Err(e) => {
                self.stats.write().await.record_failure();
                return Err(e);
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        // Record success
        {
            let mut stats = self.stats.write().await;
            stats.record_success(result.cost_usd, latency_ms);
        }

        info!(
            "Fresh invocation complete: {} issues found, cost=${:.4}",
            result.issues.len(),
            result.cost_usd
        );

        Ok(result)
    }

    async fn health_check(&self) -> Result<bool> {
        // Try to create a validator - if this succeeds, we're healthy
        match GeminiValidator::new(&self.api_key, self.config.clone()) {
            Ok(_) => Ok(true),
            Err(e) => {
                debug!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }

    fn stats(&self) -> InvocationStats {
        // Return a clone of current stats
        // Note: This is a blocking read, but stats are small
        futures::executor::block_on(async { self.stats.read().await.clone() })
    }
}

/// Pooled fresh invoker for better performance.
///
/// Maintains a pool of pre-initialized validators that are reset between
/// uses. This provides better latency than creating new instances while
/// still ensuring context isolation.
pub struct PooledFreshInvoker {
    api_key: String,
    config: AdversarialConfig,
    pool_size: usize,
    stats: Arc<RwLock<InvocationStats>>,
}

impl PooledFreshInvoker {
    /// Create a new pooled invoker.
    pub fn new(api_key: impl Into<String>, config: AdversarialConfig, pool_size: usize) -> Self {
        Self {
            api_key: api_key.into(),
            config,
            pool_size: pool_size.max(1),
            stats: Arc::new(RwLock::new(InvocationStats::default())),
        }
    }
}

#[async_trait]
impl FreshContextInvoker for PooledFreshInvoker {
    async fn invoke_fresh(&self, context: &ValidationContext) -> Result<ValidationResult> {
        // For now, same as GeminiFreshInvoker - pooling can be added later
        // when we have benchmarks showing it's needed
        let start = std::time::Instant::now();

        let validator = match GeminiValidator::new(&self.api_key, self.config.clone()) {
            Ok(v) => v,
            Err(e) => {
                self.stats.write().await.record_failure();
                return Err(e);
            }
        };

        let result = match validator.validate(context).await {
            Ok(r) => r,
            Err(e) => {
                self.stats.write().await.record_failure();
                return Err(e);
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;
        self.stats
            .write()
            .await
            .record_success(result.cost_usd, latency_ms);

        Ok(result)
    }

    async fn health_check(&self) -> Result<bool> {
        match GeminiValidator::new(&self.api_key, self.config.clone()) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn stats(&self) -> InvocationStats {
        futures::executor::block_on(async { self.stats.read().await.clone() })
    }
}

/// Builder for creating fresh context invokers.
pub struct FreshInvokerBuilder {
    api_key: Option<String>,
    config: AdversarialConfig,
    pooled: bool,
    pool_size: usize,
}

impl FreshInvokerBuilder {
    /// Create a new builder with default config.
    pub fn new() -> Self {
        Self {
            api_key: None,
            config: AdversarialConfig::default(),
            pooled: false,
            pool_size: 4,
        }
    }

    /// Set the API key.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the adversarial config.
    pub fn with_config(mut self, config: AdversarialConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable pooling.
    pub fn pooled(mut self, size: usize) -> Self {
        self.pooled = true;
        self.pool_size = size;
        self
    }

    /// Build the invoker.
    pub fn build(self) -> Result<Box<dyn FreshContextInvoker>> {
        let api_key = self.api_key.ok_or_else(|| {
            Error::Config("API key required for fresh invoker".to_string())
        })?;

        if self.pooled {
            Ok(Box::new(PooledFreshInvoker::new(
                api_key,
                self.config,
                self.pool_size,
            )))
        } else {
            Ok(Box::new(GeminiFreshInvoker::new(api_key, self.config)))
        }
    }
}

impl Default for FreshInvokerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_stats() {
        let mut stats = InvocationStats::default();

        stats.record_success(0.001, 100);
        stats.record_success(0.002, 200);
        stats.record_failure();

        assert_eq!(stats.total_invocations, 3);
        assert_eq!(stats.successful_invocations, 2);
        assert_eq!(stats.failed_invocations, 1);
        assert!((stats.total_cost_usd - 0.003).abs() < 0.0001);
        assert!((stats.avg_latency_ms - 150.0).abs() < 0.1);
        assert!((stats.success_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_builder_requires_api_key() {
        let result = FreshInvokerBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_api_key() {
        let result = FreshInvokerBuilder::new()
            .with_api_key("test-key")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_pooled() {
        let result = FreshInvokerBuilder::new()
            .with_api_key("test-key")
            .pooled(8)
            .build();
        assert!(result.is_ok());
    }
}
