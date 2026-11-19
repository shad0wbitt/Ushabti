//! Module system.

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Module kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    /// Model provider.
    Provider,
    /// Tool.
    Tool,
    /// Memory store.
    Memory,
    /// Retriever.
    Retriever,
    /// Guardrail.
    Guardrail,
    /// Channel.
    Channel,
}

/// Manifest describing a module.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    /// Module id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Version.
    pub version: String,
    /// Kind.
    pub kind: ModuleKind,
    /// Config JSON Schema.
    pub config_schema: Value,
    /// Required permissions (e.g. "fs", "net", "shell").
    #[serde(default)]
    pub required_permissions: Vec<String>,
    /// Capabilities (e.g. "streaming").
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Trait implemented by all modules.
#[async_trait]
pub trait Module: Send + Sync {
    /// Module id.
    fn id(&self) -> &str;
    /// Kind.
    fn kind(&self) -> ModuleKind;
    /// Manifest.
    fn manifest(&self) -> Manifest;
    /// Build from config.
    async fn build(&self, config: Value) -> Result<Box<dyn std::any::Any + Send>, ModuleError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("build failed: {0}")]
    Build(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("incompatible: {0}")]
    Incompatible(String),
}

/// Registry of modules.
pub struct ModuleRegistry {
    modules: BTreeMap<String, Box<dyn Module>>,
}

impl ModuleRegistry {
    /// Create empty.
    pub fn new() -> Self {
        Self { modules: BTreeMap::new() }
    }
    /// Register a module.
    pub fn register(&mut self, m: Box<dyn Module>) {
        self.modules.insert(m.id().to_string(), m);
    }
    /// Get a module.
    pub fn get(&self, id: &str) -> Option<&dyn Module> {
        self.modules.get(id).map(|b| b.as_ref())
    }
    /// List ids.
    pub fn ids(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }
    /// All manifests.
    pub fn manifests(&self) -> Vec<Manifest> {
        self.modules.values().map(|m| m.manifest()).collect()
    }
    /// Compatibility warnings for a set of chosen module ids.
    pub fn compatibility_warnings(&self, chosen: &[String]) -> Vec<String> {
        let mut warnings = Vec::new();
        let has_streaming_provider = chosen.iter().any(|id| {
            self.get(id).map(|m| m.manifest().capabilities.contains(&"streaming".to_string())).unwrap_or(false)
        });
        for id in chosen {
            if let Some(m) = self.get(id) {
                let man = m.manifest();
                if man.capabilities.contains(&"needs_streaming".to_string()) && !has_streaming_provider {
                    warnings.push(format!("{} requires a streaming provider", id));
                }
            }
        }
        warnings
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
