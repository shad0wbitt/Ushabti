//! Provider trait and tool trait for `kestrel-core`.

use crate::error::{ProviderError, ToolError};
use async_trait::async_trait;
use futures::Stream;
use kestrel_protocol::{Chunk, Message, ToolContext, ToolOutput, ToolSchema};
use std::pin::Pin;

/// A streaming chat completion provider.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stream a completion. Yields normalized [`Chunk`]s.
    async fn complete_stream(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        opts: CompletionOpts,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Chunk, ProviderError>> + Send>>, ProviderError>;

    /// Human-readable name.
    fn name(&self) -> &str;
}

/// Completion options.
#[derive(Debug, Clone, Default)]
pub struct CompletionOpts {
    /// Model id.
    pub model: String,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Max output tokens.
    pub max_tokens: Option<u32>,
}

/// A tool callable by the model.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must match schema name).
    fn name(&self) -> &str;
    /// Description.
    fn description(&self) -> &str;
    /// JSON Schema for arguments.
    fn schema(&self) -> serde_json::Value;
    /// Execute with parsed arguments.
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError>;
}

/// Build a [`kestrel_protocol::ToolSchema`] from a [`Tool`].
pub fn tool_schema<T: Tool + ?Sized>(t: &T) -> ToolSchema {
    ToolSchema {
        name: t.name().to_string(),
        description: t.description().to_string(),
        parameters: t.schema(),
    }
}
EOF