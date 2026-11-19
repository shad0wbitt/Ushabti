//! Core agent loop and traits.

pub mod agent;
pub mod error;
pub mod traits;

pub use agent::{run_agent, AgentConfig};
pub use error::{AgentError, ProviderError, ToolError};
pub use traits::{Provider, Tool, CompletionOpts, tool_schema};
EOF