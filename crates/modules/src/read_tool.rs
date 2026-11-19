//! Read tool: read file with optional line range.

use crate::paths::safe_resolve;
use crate::truncate::truncate_output;
use async_trait::async_trait;
use kestrel_core::{Tool, ToolContext, ToolError, ToolOutput};
use kestrel_protocol::ToolSchema;
use serde_json::Value;
use std::path::PathBuf;

/// Read tool.
pub struct ReadTool {
    root: PathBuf,
    max_bytes: usize,
}

impl ReadTool {
    /// Create with workspace root.
    pub fn new(root: PathBuf, max_bytes: usize) -> Self {
        Self { root, max_bytes }
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a file within the workspace root. Supports optional start_line/end_line (1-based, inclusive)."
    }
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "start_line": { "type": "integer", "minimum": 1 },
                "end_line": { "type": "integer", "minimum": 1 }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing path".into()))?;
        let resolved = safe_resolve(&self.root, path).map_err(ToolError::InvalidArgs)?;
        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        let lines: Vec<&str> = content.lines().collect();
        let start = args.get("start_line").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(1);
        let end = args.get("end_line").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(lines.len());
        let start = start.max(1).min(lines.len().max(1));
        let end = end.min(lines.len()).max(start);
        let selected: String = if lines.is_empty() {
            String::new()
        } else {
            lines[start - 1..end].join("\n")
        };
        let (out, truncated) = truncate_output(&selected, self.max_bytes);
        let mut content = out;
        if truncated {
            content.push_str("\n(truncated)");
        }
        let _ = truncated;
        Ok(ToolOutput {
            content,
            is_error: false,
        })
    }
}

// keep schema import used
#[allow(dead_code)]
fn _schema(_s: ToolSchema) {}
