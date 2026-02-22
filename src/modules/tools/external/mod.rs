pub mod mcp_protocol;
pub mod sse;
pub mod stdio;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::models::{McpServerConfig, McpTransport, ToolConfig, ToolResult};

/// Executor for external MCP Server tools (STDIO / SSE).
pub struct ExternalToolExecutor {
    timeout_secs: u64,
}

impl Default for ExternalToolExecutor {
    fn default() -> Self {
        Self { timeout_secs: 30 }
    }
}

impl ExternalToolExecutor {
    /// Create a new external tool executor with a custom timeout.
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    /// Execute an external tool via its configured MCP transport.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidConfig` if the MCP server config is missing or invalid.
    /// Returns transport-specific errors on communication failure.
    pub async fn execute(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
        config: &ToolConfig,
    ) -> Result<ToolResult, ToolError> {
        let mcp_config_value = config
            .mcp_server_config
            .as_ref()
            .ok_or_else(|| ToolError::InvalidConfig("Missing mcp_server_config".to_string()))?;

        let mcp_config: McpServerConfig = serde_json::from_value(mcp_config_value.clone())
            .map_err(|e| ToolError::InvalidConfig(format!("Invalid mcp_server_config: {e}")))?;

        match mcp_config.transport {
            McpTransport::Stdio => {
                stdio::execute_stdio(tool_name, params, &mcp_config, self.timeout_secs).await
            }
            McpTransport::Sse => {
                sse::execute_sse(tool_name, params, &mcp_config, self.timeout_secs).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_executor_constructible() {
        let executor = ExternalToolExecutor::default();
        assert_eq!(executor.timeout_secs, 30);
    }

    #[test]
    fn external_executor_with_custom_timeout() {
        let executor = ExternalToolExecutor::with_timeout(60);
        assert_eq!(executor.timeout_secs, 60);
    }
}
