//! Agent loop for `kestrel-core`.

use crate::error::AgentError;
use crate::traits::{Provider, Tool, tool_schema};
use kestrel_protocol::{Chunk, Event, FinishReason, Message, Role, ToolCall, ToolResult, ToolSchema};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Configuration for an agent run.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model id.
    pub model: String,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Max output tokens per completion.
    pub max_tokens: Option<u32>,
    /// Max agent turns.
    pub max_turns: usize,
    /// Session id.
    pub session_id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            temperature: None,
            max_tokens: None,
            max_turns: 16,
            session_id: String::new(),
        }
    }
}

/// Run the agent loop. Streams [`Event`]s into `events_tx`.
pub async fn run_agent(
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    system_prompt: Option<String>,
    user_prompt: String,
    config: AgentConfig,
    events_tx: mpsc::Sender<Event>,
    cancel: CancellationToken,
) -> Result<Vec<Message>, AgentError> {
    let tool_map: BTreeMap<String, Arc<dyn Tool>> =
        tools.iter().map(|t| (t.name().to_string(), t.clone())).collect();
    let schemas: Vec<ToolSchema> = tools.iter().map(|t| tool_schema(t.as_ref())).collect();

    let mut messages: Vec<Message> = Vec::new();
    if let Some(sp) = system_prompt {
        messages.push(Message::system(sp));
    }
    messages.push(Message::user(user_prompt));

    let _ = events_tx
        .send(Event::SessionStarted {
            session_id: config.session_id.clone(),
            protocol_version: kestrel_protocol::PROTOCOL_VERSION,
        })
        .await;

    for _turn in 0..config.max_turns {
        if cancel.is_cancelled() {
            let _ = events_tx
                .send(Event::Error {
                    session_id: Some(config.session_id.clone()),
                    message: "cancelled".into(),
                })
                .await;
            return Err(AgentError::Cancelled);
        }

        // Call provider with retry.
        let mut stream = retry_stream(
            provider.clone(),
            &messages,
            &schemas,
            &config,
            &cancel,
        )
        .await?;

        // Accumulate.
        let mut text = String::new();
        // tool call accumulators: index -> (id, name, args_json)
        let mut tool_acc: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
        let mut finish = FinishReason::Stop;
        let mut _usage = kestrel_protocol::Usage::default();

        use futures::StreamExt;
        while let Some(item) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(AgentError::Cancelled);
            }
            match item.map_err(AgentError::Provider)? {
                Chunk::TextDelta { delta } => {
                    text.push_str(&delta);
                    let _ = events_tx
                        .send(Event::MessageDelta {
                            session_id: config.session_id.clone(),
                            delta,
                        })
                        .await;
                }
                Chunk::ToolCallDelta {
                    index,
                    id,
                    name,
                    arguments_delta,
                } => {
                    let entry = tool_acc.entry(index).or_insert_with(|| (String::new(), String::new(), String::new()));
                    if let Some(i) = id {
                        if entry.0.is_empty() {
                            entry.0 = i;
                        }
                    }
                    if let Some(n) = name {
                        if entry.1.is_empty() {
                            entry.1 = n;
                        }
                    }
                    entry.2.push_str(&arguments_delta);
                }
                Chunk::Usage { usage: u } => {
                    _usage = u;
                    let _ = events_tx
                        .send(Event::UsageUpdate {
                            session_id: config.session_id.clone(),
                            usage: u,
                        })
                        .await;
                }
                Chunk::Finish { reason } => {
                    finish = reason;
                }
            }
        }

        // Build assistant message.
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        for (_idx, (id, name, args_json)) in &tool_acc {
            let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::Value::Null);
            tool_calls.push(ToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: args,
            });
        }

        let assistant_msg = Message {
            role: Role::Assistant,
            content: text.clone(),
            tool_calls: tool_calls.clone(),
            tool_results: Vec::new(),
        };
        messages.push(assistant_msg.clone());

        let _ = events_tx
            .send(Event::MessageComplete {
                session_id: config.session_id.clone(),
                message: assistant_msg,
            })
            .await;

        // If no tool calls or stop, done.
        if tool_calls.is_empty() {
            let _ = events_tx
                .send(Event::SessionEnded {
                    session_id: config.session_id.clone(),
                })
                .await;
            return Ok(messages);
        }

        // Execute tool calls (parallel via JoinSet).
        let ctx = kestrel_protocol::ToolContext {
            session_id: config.session_id.clone(),
            cwd: std::env::current_dir().unwrap_or_default(),
            // approval_tx placeholder; modules layer wires real channel
            approval_tx: tokio::sync::mpsc::channel(1).0,
        };
        // NOTE: ToolContext.approval_tx is a placeholder here; real wiring happens in modules/daemon.

        let mut join_set = tokio::task::JoinSet::new();
        for call in tool_calls.clone() {
            let tool = tool_map.get(&call.name).cloned();
            let call_clone = call.clone();
            let ctx_clone = ctx.clone();
            let _ = events_tx
                .send(Event::ToolCallStarted {
                    session_id: config.session_id.clone(),
                    call: call_clone.clone(),
                })
                .await;
            join_set.spawn(async move {
                let (result, is_error) = match tool {
                    Some(t) => match t.execute(call_clone.arguments.clone(), &ctx_clone).await {
                        Ok(out) => (out.content, out.is_error),
                        Err(e) => (format!("error: {e}"), true),
                    },
                    None => (format!("unknown tool: {}", call_clone.name), true),
                };
                (call_clone, result, is_error)
            });
        }

        // Collect results in completion order, then append in original order.
        let mut results_map: BTreeMap<String, (String, bool)> = BTreeMap::new();
        while let Some(res) = join_set.join_next().await {
            if let Ok((call, content, is_error)) = res {
                let _ = events_tx
                    .send(Event::ToolCallFinished {
                        session_id: config.session_id.clone(),
                        result: ToolResult {
                            tool_call_id: call.id.clone(),
                            content: content.clone(),
                            is_error,
                            truncated: false,
                        },
                    })
                    .await;
                results_map.insert(call.id.clone(), (content, is_error));
            }
        }

        // Append tool results message preserving call order.
        let mut tool_results: Vec<ToolResult> = Vec::new();
        for call in &tool_calls {
            if let Some((content, is_error)) = results_map.get(&call.id) {
                tool_results.push(ToolResult {
                    tool_call_id: call.id.clone(),
                    content: content.clone(),
                    is_error: *is_error,
                    truncated: false,
                });
            }
        }
        messages.push(Message {
            role: Role::Tool,
            content: String::new(),
            tool_calls: Vec::new(),
            tool_results,
        });
    }

    let _ = events_tx
        .send(Event::SessionEnded {
            session_id: config.session_id.clone(),
        })
        .await;
    Err(AgentError::MaxTurns(config.max_turns))
}

/// Retry provider stream creation with exponential backoff.
async fn retry_stream(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    schemas: &[ToolSchema],
    config: &AgentConfig,
    cancel: &CancellationToken,
) -> Result<
    Pin<Box<dyn futures::Stream<Item = Result<Chunk, crate::error::ProviderError>> + Send>>,
    AgentError,
> {
    let mut attempt = 0u32;
    let max_attempts = 5u32;
    loop {
        if cancel.is_cancelled() {
            return Err(AgentError::Cancelled);
        }
        let opts = crate::traits::CompletionOpts {
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        };
        match provider.complete_stream(messages, schemas, opts).await {
            Ok(s) => return Ok(s),
            Err(e) => {
                if !e.is_transient() || attempt + 1 >= max_attempts {
                    return Err(AgentError::Provider(e));
                }
                attempt += 1;
                let delay = std::time::Duration::from_millis(200 * (1 << attempt.min(6)));
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {}
                    _ = cancel.cancelled() => return Err(AgentError::Cancelled),
                }
            }
        }
    }
}
