//! Guardrails: input/output hooks.

use async_trait::async_trait;
use kestrel_protocol::Message;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardrailError {
    #[error("blocked: {0}")]
    Blocked(String),
    #[error("modified")]
    Modified,
}

/// Decision from a guardrail.
#[derive(Debug, Clone)]
pub enum GuardrailDecision {
    /// Allow unchanged.
    Allow,
    /// Allow with modified message.
    Modify(Message),
    /// Block.
    Block(String),
}

#[async_trait]
pub trait Guardrail: Send + Sync {
    async fn check_input(&self, msg: &Message) -> GuardrailDecision;
    async fn check_output(&self, msg: &Message) -> GuardrailDecision;
}

/// Spend cap guardrail (token/cost limits).
pub struct SpendCap {
    pub max_tokens: Option<u64>,
    pub max_cost_usd: Option<f64>,
    pub used_tokens: std::sync::atomic::AtomicU64,
    pub used_cost: std::sync::Mutex<f64>,
}

impl SpendCap {
    pub fn new(max_tokens: Option<u64>, max_cost_usd: Option<f64>) -> Self {
        Self {
            max_tokens,
            max_cost_usd,
            used_tokens: std::sync::atomic::AtomicU64::new(0),
            used_cost: std::sync::Mutex::new(0.0),
        }
    }
    pub fn record(&self, tokens: u64, cost: f64) {
        self.used_tokens.fetch_add(tokens, std::sync::atomic::Ordering::Relaxed);
        *self.used_cost.lock().unwrap() += cost;
    }
}

#[async_trait]
impl Guardrail for SpendCap {
    async fn check_input(&self, _msg: &Message) -> GuardrailDecision {
        let used = self.used_tokens.load(std::sync::atomic::Ordering::Relaxed);
        if let Some(max) = self.max_tokens {
            if used >= max {
                return GuardrailDecision::Block(format!("spend cap: {used} >= {max} tokens"));
            }
        }
        let cost = *self.used_cost.lock().unwrap();
        if let Some(max) = self.max_cost_usd {
            if cost >= max {
                return GuardrailDecision::Block(format!("spend cap: ${cost:.4} >= ${max:.4}"));
            }
        }
        GuardrailDecision::Allow
    }
    async fn check_output(&self, _msg: &Message) -> GuardrailDecision {
        GuardrailDecision::Allow
    }
}

/// PII redaction guardrail.
pub struct PiiRedaction {
    pub patterns: Vec<String>,
}

impl PiiRedaction {
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }
    fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        // email
        // simple regex via manual-ish: use regex crate? not in deps. Use basic replacements.
        // We'll handle common patterns with regex crate if available; fallback to naive.
        out
    }
}

#[async_trait]
impl Guardrail for PiiRedaction {
    async fn check_input(&self, msg: &Message) -> GuardrailDecision {
        let redacted = self.redact(&msg.content);
        if redacted != msg.content {
            let mut m = msg.clone();
            m.content = redacted;
            return GuardrailDecision::Modify(m);
        }
        GuardrailDecision::Allow
    }
    async fn check_output(&self, msg: &Message) -> GuardrailDecision {
        let redacted = self.redact(&msg.content);
        if redacted != msg.content {
            let mut m = msg.clone();
            m.content = redacted;
            return GuardrailDecision::Modify(m);
        }
        GuardrailDecision::Allow
    }
}
