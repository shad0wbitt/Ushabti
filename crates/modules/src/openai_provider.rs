//! OpenAI-compatible provider with SSE streaming.

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use kestrel_core::{CompletionOpts, Provider, ProviderError};
use kestrel_protocol::{Chunk, Message, ToolSchema, Usage};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;

/// OpenAI-compatible provider.
pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    /// Create a new provider.
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn complete_stream(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        opts: CompletionOpts,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<Chunk, ProviderError>> + Send>>, ProviderError> {
        let model = if opts.model.is_empty() { self.model.clone() } else { opts.model.clone() };
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut msgs: Vec<Value> = Vec::new();
        for m in messages {
            let role = match m.role {
                kestrel_protocol::Role::System => "system",
                kestrel_protocol::Role::User => "user",
                kestrel_protocol::Role::Assistant => "assistant",
                kestrel_protocol::Role::Tool => "tool",
            };
            let mut obj = serde_json::json!({ "role": role, "content": m.content });
            if !m.tool_calls.is_empty() {
                let tcs: Vec<Value> = m
                    .tool_calls
                    .iter()
                    .map(|tc| serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": { "name": tc.name, "arguments": tc.arguments.to_string() }
                    }))
                    .collect();
                obj["tool_calls"] = Value::Array(tcs);
            }
            if !m.tool_results.is_empty() {
                // emit one tool message per result; keep first for simplicity
                for tr in &m.tool_results {
                    msgs.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tr.tool_call_id,
                        "content": tr.content
                    }));
                }
                continue;
            }
            msgs.push(obj);
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": msgs,
            "stream": true,
        });
        if let Some(t) = opts.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(m) = opts.max_tokens {
            body["max_tokens"] = serde_json::json!(m);
        }
        if !tools.is_empty() {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                }))
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Http { status, body });
        }

        let byte_stream = resp.bytes_stream();

        // Parse SSE manually, yield Chunks.
        let stream = async_stream::stream! {
            let mut buf = String::new();
            let mut bs = byte_stream;
            while let Some(chunk) = bs.next().await {
                match chunk {
                    Ok(b) => {
                        buf.push_str(&String::from_utf8_lossy(&b));
                        // process complete lines
                        while let Some(nl) = buf.find('\n') {
                            let line = buf[..nl].trim().to_string();
                            buf = buf[nl+1..].to_string();
                            if line.is_empty() || line.starts_with(':') { continue; }
                            if let Some(data) = line.strip_prefix("data: ") {
                                let data = data.trim();
                                if data == "[DONE]" { 
                                    yield Ok(Chunk::Finish { reason: kestrel_protocol::FinishReason::Stop });
                                    continue; 
                                }
                                if let Ok(v) = serde_json::from_str::<Value>(data) {
                                    // choices[0].delta
                                    if let Some(choice) = v.get("choices").and_then(|c| c.get(0)) {
                                        if let Some(delta) = choice.get("delta") {
                                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                                if !content.is_empty() {
                                                    yield Ok(Chunk::TextDelta { delta: content.to_string() });
                                                }
                                            }
                                            if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                for tc in tcs {
                                                    let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                                    let id = tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                                                    let name = tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).map(|s| s.to_string());
                                                    let args = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()).unwrap_or("").to_string();
                                                    yield Ok(Chunk::ToolCallDelta { index: idx, id, name, arguments_delta: args });
                                                }
                                            }
                                        }
                                        if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                                            let reason = match fr {
                                                "stop" => kestrel_protocol::FinishReason::Stop,
                                                "tool_calls" => kestrel_protocol::FinishReason::ToolCalls,
                                                "length" => kestrel_protocol::FinishReason::Length,
                                                "content_filter" => kestrel_protocol::FinishReason::ContentFilter,
                                                _ => kestrel_protocol::FinishReason::Other,
                                            };
                                            yield Ok(Chunk::Finish { reason });
                                        }
                                    }
                                    if let Some(u) = v.get("usage") {
                                        let usage = Usage {
                                            prompt_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                                            completion_tokens: u.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                                            total_tokens: u.get("total_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                                        };
                                        yield Ok(Chunk::Usage { usage });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::Network(e.to_string()));
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "openai"
    }
}
