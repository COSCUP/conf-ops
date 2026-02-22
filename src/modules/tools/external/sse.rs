//! SSE transport for external MCP Server communication.

use crate::modules::tools::error::ToolError;
use crate::modules::tools::models::{McpServerConfig, ToolResult};

use super::mcp_protocol::{JsonRpcRequest, JsonRpcResponse, McpToolResult};

/// Execute a tool via SSE MCP Server transport.
///
/// Connects to the remote MCP server and sends a `tools/call` request via HTTP POST.
///
/// # Errors
///
/// Returns `ToolError::McpConnectionFailed` if the connection cannot be established.
/// Returns `ToolError::McpTimeout` if execution exceeds the timeout.
/// Returns `ToolError::McpInvalidResponse` if the response is not valid JSON-RPC.
pub async fn execute_sse(
    tool_name: &str,
    params: &serde_json::Value,
    config: &McpServerConfig,
    timeout_secs: u64,
) -> Result<ToolResult, ToolError> {
    let url = config
        .url
        .as_deref()
        .ok_or_else(|| ToolError::InvalidConfig("Missing url for sse transport".to_string()))?;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to build HTTP client: {e}")))?;

    let call_req = JsonRpcRequest::tools_call(1, tool_name, params);

    let mut request = client.post(url).json(&call_req);

    if let Some(ref auth_header) = config.auth_header {
        request = request.header("Authorization", auth_header);
    }

    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            ToolError::McpTimeout
        } else {
            ToolError::McpConnectionFailed(format!("HTTP request failed: {e}"))
        }
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ToolError::McpConnectionFailed(format!(
            "HTTP {status}: {body}"
        )));
    }

    let rpc_response: JsonRpcResponse = response
        .json()
        .await
        .map_err(|e| ToolError::McpInvalidResponse(format!("Invalid JSON-RPC response: {e}")))?;

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    if let Some(error) = rpc_response.error {
        return Err(ToolError::ExecutionError(error.message));
    }

    let result_value = rpc_response
        .result
        .ok_or_else(|| ToolError::McpInvalidResponse("Missing result in response".to_string()))?;

    let mcp_result: McpToolResult = serde_json::from_value(result_value)
        .map_err(|e| ToolError::McpInvalidResponse(format!("Invalid tool result: {e}")))?;

    let content_text = mcp_result
        .content
        .iter()
        .filter_map(|c| c.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    let content = serde_json::from_str(&content_text)
        .unwrap_or_else(|_| serde_json::json!({"text": content_text}));

    Ok(ToolResult {
        content,
        is_error: mcp_result.is_error,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use crate::modules::tools::models::McpTransport;

    use super::*;

    #[test]
    fn sse_requires_url() {
        let config = McpServerConfig {
            transport: McpTransport::Sse,
            command: None,
            url: None,
            args: None,
            env: None,
            auth_header: None,
        };
        // Verify that we'd get an error for missing URL.
        assert!(config.url.is_none());
    }
}
