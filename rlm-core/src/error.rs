//! Error types for rlm-core.

use thiserror::Error;

/// Result type alias using rlm-core's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during RLM operations.
#[derive(Error, Debug)]
pub enum Error {
    /// REPL execution failed
    #[error("REPL execution error: {message}")]
    ReplExecution {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Subprocess communication error
    #[error("Subprocess communication error: {0}")]
    SubprocessComm(String),

    /// Timeout during operation
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// LLM API error
    #[error("LLM API error: {provider} - {message}")]
    LlmApi { provider: String, message: String },

    /// LLM error (simple variant)
    #[error("LLM error: {0}")]
    LLM(String),

    /// Memory storage error
    #[error("Memory storage error: {0}")]
    MemoryStorage(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Recursion depth exceeded
    #[error("Maximum recursion depth {max_depth} exceeded")]
    MaxDepthExceeded { max_depth: u32 },

    /// Budget exhausted
    #[error("Budget exhausted: {resource}")]
    BudgetExhausted { resource: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a REPL execution error.
    pub fn repl_execution(message: impl Into<String>) -> Self {
        Self::ReplExecution {
            message: message.into(),
            source: None,
        }
    }

    /// Create a REPL execution error with source.
    pub fn repl_execution_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ReplExecution {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an LLM API error.
    pub fn llm_api(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::LlmApi {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create a timeout error.
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create a max depth exceeded error.
    pub fn max_depth_exceeded(max_depth: u32) -> Self {
        Self::MaxDepthExceeded { max_depth }
    }

    /// Create a budget exhausted error.
    pub fn budget_exhausted(resource: impl Into<String>) -> Self {
        Self::BudgetExhausted {
            resource: resource.into(),
        }
    }
}
