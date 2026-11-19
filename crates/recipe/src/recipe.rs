//! Recipe format and validation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("yaml parse error: {0}")]
    Parse(String),
    #[error("invalid field {path}: {message}")]
    Field { path: String, message: String },
}

/// Top-level recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// Name.
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Schema version.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Model config.
    pub model: ModelConfig,
    /// Prompt config.
    #[serde(default)]
    pub prompt: PromptConfig,
    /// Tools.
    #[serde(default)]
    pub tools: Vec<ToolEntry>,
    /// Memory module id.
    #[serde(default = "default_memory")]
    pub memory: String,
    /// Retriever module id.
    #[serde(default)]
    pub retriever: Option<String>,
    /// Guardrail module ids.
    #[serde(default)]
    pub guardrails: Vec<String>,
    /// Channel module ids.
    #[serde(default)]
    pub channels: Vec<String>,
    /// Limits.
    #[serde(default)]
    pub limits: Limits,
}

fn default_version() -> u32 {
    1
}
fn default_memory() -> String {
    "none".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Provider module id.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Temperature.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Max tokens.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Fallback model ids.
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Inline text.
    #[serde(default)]
    pub text: Option<String>,
    /// File path.
    #[serde(default)]
    pub file: Option<String>,
    /// Variables for minijinja substitution.
    #[serde(default)]
    pub vars: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Tool module id.
    pub id: String,
    /// Per-tool config.
    #[serde(default)]
    pub config: serde_json::Value,
    /// Permission level.
    #[serde(default = "default_perm")]
    pub permission: Permission,
}

fn default_perm() -> Permission {
    Permission::Ask
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default = "default_turns")]
    pub max_turns: usize,
    #[serde(default)]
    pub spend_cap_usd: Option<f64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_turns: 16,
            spend_cap_usd: None,
            timeout_seconds: None,
        }
    }
}

fn default_turns() -> usize {
    16
}

impl Recipe {
    /// Parse from YAML string.
    pub fn from_yaml(s: &str) -> Result<Self, RecipeError> {
        serde_yaml::from_str(s).map_err(|e| RecipeError::Parse(e.to_string()))
    }

    /// Validate, returning field-path errors.
    pub fn validate(&self) -> Result<(), RecipeError> {
        if self.name.trim().is_empty() {
            return Err(RecipeError::Field { path: "name".into(), message: "must not be empty".into() });
        }
        if self.model.model.trim().is_empty() {
            return Err(RecipeError::Field { path: "model.model".into(), message: "must not be empty".into() });
        }
        if self.model.provider.trim().is_empty() {
            return Err(RecipeError::Field { path: "model.provider".into(), message: "must not be empty".into() });
        }
        for (i, t) in self.tools.iter().enumerate() {
            if t.id.trim().is_empty() {
                return Err(RecipeError::Field { path: format!("tools[{i}].id"), message: "must not be empty".into() });
            }
        }
        if self.limits.max_turns == 0 {
            return Err(RecipeError::Field { path: "limits.max_turns".into(), message: "must be > 0".into() });
        }
        Ok(())
    }

    /// Render prompt text with minijinja, resolving file path if set.
    pub fn render_prompt(&self) -> Result<String, RecipeError> {
        let raw = if let Some(text) = &self.prompt.text {
            text.clone()
        } else if let Some(file) = &self.prompt.file {
            std::fs::read_to_string(file).map_err(|e| RecipeError::Field {
                path: "prompt.file".into(),
                message: e.to_string(),
            })?
        } else {
            String::new()
        };
        let env = minijinja::Environment::new();
        let tmpl = env.template_from_str(&raw).map_err(|e| RecipeError::Field {
            path: "prompt.text".into(),
            message: e.to_string(),
        })?;
        let ctx = minijinja::Value::from_serialize(self.prompt.vars.clone());
        let rendered = tmpl.render(ctx).map_err(|e| RecipeError::Field {
            path: "prompt.text".into(),
            message: e.to_string(),
        })?;
        Ok(rendered)
    }

    /// Hash content for versioning.
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let s = serde_yaml::to_string(self).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        format!("{:x}", h.finalize())
    }
}
