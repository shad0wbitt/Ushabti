//! Edit tool: exact string replace, fails if match not unique.

use crate::paths::safe_resolve;
use async_trait::async_trait;
use kestrel_core::{Tool, ToolContext, ToolError, ToolOutput};
use serde_json::Value;
use std::path::PathBuf;

/// Edit tool.
pub struct EditTool {
    root: PathBuf,
}

impl EditTool {
    /// Create with workspace root.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit_file"
    }
    fn description(&self) -> &str {
        "Replace an exact string in a file. Fails if old_string occurs zero or more than once."
    }
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| ToolError::InvalidArgs("missing path".into()))?;
        let old = args.get("old_string").and_then(|v| v.as_str()).ok_or_else(|| ToolError::InvalidArgs("missing old_string".into()))?;
        let new = args.get("new_string").and_then(|v| v.as_str()).ok_or_else(|| ToolError::InvalidArgs("missing new_string".into()))?;
        let resolved = safe_resolve(&self.root, path).map_err(ToolError::InvalidArgs)?;
        let content = tokio::fs::read_to_string(&resolved).await.map_err(|e| ToolError::Execution(e.to_string()))?;
        let count = content.matches(old).count();
        if count == 0 {
            return Ok(ToolOutput { content: "old_string not found".into(), is_error: true });
        }
        if count > 1 {
            return Ok(ToolOutput { content: format!("old_string occurs {count} times; must be unique"), is_error: true });
        }
        let updated = content.replacen(old, new, 1);
        tokio::fs::write(&resolved, updated).await.map_err(|e| ToolError::Execution(e.to_string()))?;
        Ok(ToolOutput { content: "ok".into(), is_error: false })
    }
}
EOF