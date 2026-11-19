//! Bash tool: timeout, merged stdout/stderr, output cap.

use crate::paths::safe_resolve;
use crate::truncate::truncate_output;
use async_trait::async_trait;
use kestrel_core::{Tool, ToolContext, ToolError, ToolOutput};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Bash tool.
pub struct BashTool {
    root: PathBuf,
    max_bytes: usize,
    default_timeout: Duration,
}

impl BashTool {
    /// Create with workspace root.
    pub fn new(root: PathBuf, max_bytes: usize, default_timeout: Duration) -> Self {
        Self { root, max_bytes, default_timeout }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn description(&self) -> &str {
        "Run a shell command in the workspace root. Merges stdout/stderr. Enforces timeout and output cap."
    }
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "timeout_ms": { "type": "integer", "minimum": 1 }
            },
            "required": ["command"]
        })
    }
    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let cmd = args.get("command").and_then(|v| v.as_str()).ok_or_else(|| ToolError::InvalidArgs("missing command".into()))?;
        let to = args
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis)
            .unwrap_or(self.default_timeout);

        // Resolve cwd inside root (root itself).
        let cwd = safe_resolve(&self.root, ".").map_err(ToolError::InvalidArgs)?;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let run = async {
            let (o, e) = tokio::join!(
                async { if let Some(mut s) = stdout { use tokio::io::AsyncReadExt; let mut b = Vec::new(); let _ = s.read_to_end(&mut b).await; b } else { Vec::new() } },
                async { if let Some(mut s) = stderr { use tokio::io::AsyncReadExt; let mut b = Vec::new(); let _ = s.read_to_end(&mut b).await; b } else { Vec::new() } },
            );
            let status = child.wait().await;
            (o, e, status)
        };

        match timeout(to, run).await {
            Ok((o, e, status)) => {
                let mut combined = String::from_utf8_lossy(&o).to_string();
                if !e.is_empty() {
                    combined.push_str("\n--- stderr ---\n");
                    combined.push_str(&String::from_utf8_lossy(&e));
                }
                let (out, truncated) = truncate_output(&combined, self.max_bytes);
                let mut content = out;
                if truncated {
                    content.push_str("\n(truncated)");
                }
                let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
                if code != 0 {
                    content = format!("exit code {code}\n{content}");
                }
                Ok(ToolOutput { content, is_error: code != 0 })
            }
            Err(_) => {
                let _ = child.kill().await;
                Ok(ToolOutput { content: "command timed out".into(), is_error: true })
            }
        }
    }
}
EOF