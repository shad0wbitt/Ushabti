# Design

## Project name proposals

1. **Kestrel** - modular, recipe-driven agent harness. (Matches workspace identity; short, memorable.)
2. **Loom** - weave modules into a harness from a recipe.
3. **Forge** - compose agent harnesses from parts.

Recommendation: Kestrel. Crate prefix `kestrel-`.

## Scope

Build a modular, recipe-driven AI agent harness in Rust with a terminal UI and a desktop app. Users compose a harness by selecting modules (model provider, tools, memory, guardrails, channels, system prompt) in a `recipe.yaml`. The harness runs an agent loop, exposes tools, manages context, handles approvals, stores sessions, and renders via TUI and desktop.

## Explicit non-goals

- No model training or fine-tuning.
- No hosting/SaaS control plane in Phase 1; single-user local first.
- No multi-tenant auth in Phase 1.
- No plugin marketplace or remote plugin loading in Phase 1 (local modules only).
- No browser-based web UI in Phase 1 (TUI + desktop only).
- No support for non-Rust modules in Phase 1.
- No automatic code execution without approval gates.

## Crate layout (Cargo workspace)

```
kestrel/
  Cargo.toml                 # workspace root
  crates/
    kestrel-protocol/        # shared types, events, commands, recipe schema
    kestrel-core/            # agent loop, context manager, session store, orchestrator
    kestrel-modules/         # trait impls: providers, tools, memory, guardrails, channels
    kestrel-mcp/             # MCP client (and later server)
    kestrel-daemon/          # headless runtime, socket/IPC, session persistence
    kestrel-cli/             # binary: run recipes, approvals, one-shot
    kestrel-tui/             # terminal UI (ratatui)
    kestrel-desktop/         # desktop app (tauri or egui)
  recipes/                   # example recipe.yaml files
  docs/
```

Dependency direction: protocol <- core <- modules/mcp <- daemon <- (cli, tui, desktop). No crate depends upward.

## Draft trait signatures (Rust snippets)

```rust
// kestrel-protocol / kestrel-core

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Role { System, User, Assistant, Tool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
    pub is_error: bool,
    pub truncated: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        opts: CompletionOpts,
    ) -> Result<Completion, ProviderError>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct CompletionOpts {
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Clone)]
pub struct Completion {
    pub message: Message,
    pub usage: Usage,
    pub finish_reason: FinishReason,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value; // JSON Schema
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError>;
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub cwd: std::path::PathBuf,
    pub approval_tx: tokio::sync::mpsc::Sender<ApprovalRequest>,
}

#[async_trait]
pub trait Memory: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<Vec<Message>, MemoryError>;
    async fn save(&self, session_id: &str, messages: &[Message]) -> Result<(), MemoryError>;
    async fn search(&self, query: &str, k: usize) -> Result<Vec<Message>, MemoryError>;
}

#[async_trait]
pub trait Guardrail: Send + Sync {
    async fn check_input(&self, msg: &Message) -> Result<(), GuardrailError>;
    async fn check_output(&self, msg: &Message) -> Result<(), GuardrailError>;
    async fn check_tool(&self, call: &ToolCall) -> Result<(), GuardrailError>;
}

#[async_trait]
pub trait Channel: Send + Sync {
    async fn send(&self, msg: &Message) -> Result<(), ChannelError>;
    async fn recv(&self) -> Result<Message, ChannelError>;
}
```

## Protocol events and commands

Commands (client -> daemon):
- `StartSession { recipe: PathBuf }`
- `SendMessage { session_id, content }`
- `Approve { approval_id, decision: Approve|Deny }`
- `Cancel { session_id }`
- `ListSessions`
- `LoadSession { session_id }`

Events (daemon -> client):
- `SessionStarted { session_id }`
- `Message { session_id, message }`
- `ToolCallStarted { session_id, call }`
- `ToolCallFinished { session_id, result }`
- `ApprovalRequested { approval_id, tool, args, reason }`
- `ApprovalResolved { approval_id, decision }`
- `StreamChunk { session_id, delta }`
- `Error { session_id, error }`
- `SessionEnded { session_id }`

Transport: newline-delimited JSON over Unix socket (IPC) and stdio for CLI one-shot. WebSocket optional later.

## Example recipe.yaml

```yaml
name: code-assistant
version: 1
model:
  provider: openai
  model: gpt-4o-mini
  temperature: 0.2
system_prompt: |
  You are a coding assistant. Use tools to read and edit files.
  Ask before running destructive commands.
tools:
  - read_file
  - write_file
  - edit_file
  - bash
  - grep
  - glob
memory:
  kind: file
  path: ~/.kestrel/sessions
guardrails:
  - no_secrets_in_output
  - require_approval_for_bash
channels:
  - terminal
approvals:
  mode: interactive   # interactive | auto-approve | deny-all
  timeout_seconds: 300
truncation:
  max_output_bytes: 32768
  strategy: head_tail  # head | tail | head_tail
```

## Approvals

- Each tool declares a risk level: `safe` (read), `mutating` (write/edit), `dangerous` (bash, network).
- Guardrails and a policy map risk level -> approval requirement.
- On `dangerous` or policy-required calls, the daemon emits `ApprovalRequested` and suspends the tool call on a oneshot channel.
- TUI/desktop render a modal; user approves/denies. Timeout denies by default.
- Auto-approve mode skips the modal but still logs.
- Decisions are recorded in the session log for audit.

## Session storage

- Sessions are stored as append-only JSONL at `~/.kestrel/sessions/<session_id>.jsonl`.
- Each line is a protocol event (Message, ToolCall*, Approval*).
- A sidecar `<session_id>.meta.json` holds recipe hash, created_at, model, title.
- Memory trait impls (file/sqlite) read/write this store.
- Rotation/compaction: summarize older turns when context exceeds budget; keep raw log for audit.

## License plan

- Project license: MIT OR Apache-2.0 (dual).
- Reference code: none may be copied until licenses verified. Treat all references as inspiration only until confirmed.
- Adaptable only after verification:
  - codex (Apache-2.0 upstream): tool schemas, approval UX patterns may be adapted with attribution.
  - rust-sdk (MIT/Apache-2.0): MCP transport and schema types may be adapted with attribution.
- Must only inspire (do not copy): any GPL/AGPL project; any project with unknown license.
- All adapted code must carry an attribution comment naming source file and license.

## Open questions (tracked in OPEN-QUESTIONS.md)

See OPEN-QUESTIONS.md.
