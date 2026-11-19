//! Error types for `kestrel-core`.

use thiserror::Error;

/// Provider errors.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Network/transport error.
    #[error("network error: {0}")]
    Network(String),
    /// HTTP error with status.
    #[error("http {status}: {body}")]
    Http {
        /// Status code.
        status: u16,
        /// Body.
        body: String,
    },
    /// Failed to parse response.
    #[error("parse error: {0}")]
    Parse(String),
    /// Request cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Other.
    #[error("{0}")]
    Other(String),
}

impl ProviderError {
    /// Whether the error is transient and worth retrying.
    pub fn is_transient(&self) -> bool {
        match self {
            ProviderError::Network(_) => true,
            ProviderError::Http { status, .. } => *status == 429 || (500..=599).contains(status),
            _ => false,
        }
    }
}

/// Tool errors.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Invalid arguments.
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    /// Execution failed.
    #[error("execution failed: {0}")]
    Execution(String),
    /// Cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Other.
    #[error("{0}")]
    Other(String),
}

/// Agent loop errors.
#[derive(Debug, Error)]
pub enum AgentError {
    /// Provider error.
    #[error("provider: {0}")]
    Provider(#[from] ProviderError),
    /// Tool error.
    #[error("tool: {0}")]
    Tool(#[from] ToolError),
    /// Cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Max turns exceeded.
    #[error("max turns ({0}) exceeded")]
    MaxTurns(usize),
    /// Other.
    #[error("{0}")]
    Other(String),
}
