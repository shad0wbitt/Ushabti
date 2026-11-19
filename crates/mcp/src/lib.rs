//! MCP client integration.

use anyhow::Result;
use rmcp::{model::CallToolRequestParams, service::ServiceExt, transport::{ConfigureCommandExt, TokioChildProcess}};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct McpClient {
    inner: rmcp::service::RunningService<rmcp::RoleClient, ()>,
}

impl McpClient {
    pub async fn connect(cfg: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&cfg.command);
        for a in &cfg.args { cmd.arg(a); }
        let transport = TokioChildProcess::new(cmd.configure(|_| {}))
            .map_err(|e| anyhow::anyhow!("transport: {e}"))?;
        let inner = ().serve(transport).await.map_err(|e| anyhow::anyhow!("serve: {e}"))?;
        Ok(Self { inner })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDef>> {
        let tools = self.inner.list_tools(Default::default()).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(tools.tools.into_iter().map(|t| McpToolDef {
            name: t.name.to_string(),
            description: t.description.map(|d| d.to_string()).unwrap_or_default(),
            input_schema: serde_json::to_value(&*t.input_schema).unwrap_or(serde_json::json!({})),
        }).collect())
    }

    pub async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<String> {
        let name_owned = name.to_string();
        let params = CallToolRequestParams::new(name_owned).with_arguments(
            args.as_object().cloned().unwrap_or_default(),
        );
        let res = self.inner.call_tool(params).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(format!("{:?}", res))
    }
}
