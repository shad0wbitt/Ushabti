use async_trait::async_trait;
use kestrel_core::traits::Tool;
use kestrel_core::error::ToolError;
use kestrel_protocol::{ToolContext, ToolOutput};
use kestrel_mcp::{McpClient, McpToolDef};
use std::sync::Arc;

pub struct McpTool {
    client: Arc<McpClient>,
    def: McpToolDef,
}

impl McpTool {
    pub fn new(client: Arc<McpClient>, def: McpToolDef) -> Self { Self { client, def } }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str { &self.def.name }
    fn description(&self) -> &str { &self.def.description }
    fn schema(&self) -> serde_json::Value { self.def.input_schema.clone() }
    async fn execute(&self, args: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        match self.client.call_tool(&self.def.name, args).await {
            Ok(content) => Ok(ToolOutput { content, is_error: false }),
            Err(e) => Ok(ToolOutput { content: e.to_string(), is_error: true }),
        }
    }
}
