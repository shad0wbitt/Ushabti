//! Shared protocol types for the Kestrel agent harness.
//!
//! All clients (CLI, TUI, desktop, daemon) exchange [`Event`]s and [`Command`]s
//! serialized as newline-delimited JSON with `#[serde(tag = "type")]`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Protocol version. Bump on breaking changes to [`Event`] or [`Command`].
pub const PROTOCOL_VERSION: u32 = 1;

/// Role of a message participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// System prompt.
    System,
    /// End user.
    User,
    /// Model assistant.
    Assistant,
    /// Tool result carrier.
    Tool,
}

/// A single tool invocation requested by the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned call id.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Parsed JSON arguments.
    pub arguments: serde_json::Value,
}

/// Result of executing a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResult {
    /// Matching [`ToolCall::id`].
    pub tool_call_id: String,
    /// Tool output (possibly truncated).
    pub content: String,
    /// Whether execution failed.
    pub is_error: bool,
    /// Whether output was truncated.
    pub truncated: bool,
}

/// A chat message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Participant role.
    pub role: Role,
    /// Text content.
    pub content: String,
    /// Tool calls requested (assistant messages).
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tool results (tool messages).
    #[serde(default)]
    pub tool_results: Vec<ToolResult>,
}

impl Message {
    /// Construct a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }
    /// Construct a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }
    /// Construct an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }
}

/// Token usage reported by a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt tokens.
    pub prompt_tokens: u32,
    /// Completion tokens.
    pub completion_tokens: u32,
    /// Total tokens.
    pub total_tokens: u32,
}

/// Why a completion finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop.
    Stop,
    /// Tool calls requested.
    ToolCalls,
    /// Length limit reached.
    Length,
    /// Content filter.
    ContentFilter,
    /// Unknown/other.
    Other,
}


/// Context passed to a tool during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Session id.
    pub session_id: String,
    /// Working directory.
    pub cwd: std::path::PathBuf,
    /// Approval sender (placeholder).
    pub approval_tx: tokio::sync::mpsc::Sender<crate::Event>,
}

/// Output of a tool execution.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolOutput {
    /// Content.
    pub content: String,
    /// Whether it is an error.
    pub is_error: bool,
}

/// Normalized streaming chunk from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Chunk {
    /// Text delta.
    TextDelta {
        /// Delta text.
        delta: String,
    },
    /// Tool call delta (partial JSON accumulated by caller).
    ToolCallDelta {
        /// Call index within this completion.
        index: usize,
        /// Call id (may arrive once).
        id: Option<String>,
        /// Tool name (may arrive once).
        name: Option<String>,
        /// Argument JSON fragment.
        arguments_delta: String,
    },
    /// Usage update.
    Usage {
        /// Usage.
        usage: Usage,
    },
    /// Finish reason.
    Finish {
        /// Reason.
        reason: FinishReason,
    },
}

/// Tool schema advertised to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for arguments.
    pub parameters: serde_json::Value,
}

/// Events emitted by the daemon/core to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A session has started.
    SessionStarted {
        /// Session id.
        session_id: String,
        /// Protocol version.
        protocol_version: u32,
    },
    /// Incremental assistant text.
    MessageDelta {
        /// Session id.
        session_id: String,
        /// Text delta.
        delta: String,
    },
    /// A complete assistant message.
    MessageComplete {
        /// Session id.
        session_id: String,
        /// Message.
        message: Message,
    },
    /// A tool call has started.
    ToolCallStarted {
        /// Session id.
        session_id: String,
        /// Call.
        call: ToolCall,
    },
    /// A tool call finished.
    ToolCallFinished {
        /// Session id.
        session_id: String,
        /// Result.
        result: ToolResult,
    },
    /// Approval is required before a tool runs.
    ApprovalRequest {
        /// Approval id.
        approval_id: String,
        /// Session id.
        session_id: String,
        /// Tool name.
        tool: String,
        /// Arguments.
        arguments: serde_json::Value,
        /// Human-readable reason.
        reason: String,
    },
    /// Usage update.
    UsageUpdate {
        /// Session id.
        session_id: String,
        /// Usage.
        usage: Usage,
    },
    /// Error.
    Error {
        /// Session id (if any).
        session_id: Option<String>,
        /// Error message.
        message: String,
    },
    /// Session ended.
    SessionEnded {
        /// Session id.
        session_id: String,
    },
}

/// Commands sent by clients to the daemon/core.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Send a user message.
    SendMessage {
        /// Session id.
        session_id: String,
        /// Content.
        content: String,
    },
    /// Respond to an approval request.
    ApprovalResponse {
        /// Approval id.
        approval_id: String,
        /// Approved?
        approved: bool,
    },
    /// Cancel a running session.
    Cancel {
        /// Session id.
        session_id: String,
    },
}

/// Generic key/value config map used by modules.
pub type ConfigMap = BTreeMap<String, serde_json::Value>;
