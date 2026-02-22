//! STDIO transport for external MCP Server communication.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::models::{McpServerConfig, ToolResult};

use super::mcp_protocol::{JsonRpcRequest, JsonRpcResponse, McpToolResult};

/// Execute a tool via STDIO MCP Server transport.
///
/// Spawns a child process, sends initialize + tools/call, and reads the response.
///
/// # Errors
///
/// Returns `ToolError::McpConnectionFailed` if the process cannot be spawned.
/// Returns `ToolError::McpTimeout` if execution exceeds the timeout.
/// Returns `ToolError::McpInvalidResponse` if the response is not valid JSON-RPC.
/// Returns `ToolError::McpProcessCrashed` if the process exits unexpectedly.
pub async fn execute_stdio(
    tool_name: &str,
    params: &serde_json::Value,
    config: &McpServerConfig,
    timeout_secs: u64,
) -> Result<ToolResult, ToolError> {
    let command = config.command.as_deref().ok_or_else(|| {
        ToolError::InvalidConfig("Missing command for stdio transport".to_string())
    })?;

    let args: Vec<&str> = config
        .args
        .as_ref()
        .map_or_else(Vec::new, |a| a.iter().map(String::as_str).collect());

    let start = std::time::Instant::now();

    let mut child = Command::new(command)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to spawn process: {e}")))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ToolError::McpConnectionFailed("Failed to open stdin".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ToolError::McpConnectionFailed("Failed to open stdout".to_string()))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    let result = tokio::time::timeout(timeout, async {
        communicate(stdin, stdout, tool_name, params).await
    })
    .await
    .map_err(|_| ToolError::McpTimeout)?;

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // Kill the child process
    let _ = child.kill().await;

    result.map(|mut r| {
        r.duration_ms = duration_ms;
        r
    })
}

async fn communicate(
    mut stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    tool_name: &str,
    params: &serde_json::Value,
) -> Result<ToolResult, ToolError> {
    let mut reader = BufReader::new(stdout);

    // Step 1: Initialize
    let init_req = JsonRpcRequest::initialize(1);
    send_request(&mut stdin, &init_req).await?;
    let _init_resp = read_response(&mut reader).await?;

    // Step 2: Call tool
    let call_req = JsonRpcRequest::tools_call(2, tool_name, params);
    send_request(&mut stdin, &call_req).await?;
    let call_resp = read_response(&mut reader).await?;

    // Parse result
    if let Some(error) = call_resp.error {
        return Err(ToolError::ExecutionError(error.message));
    }

    let result_value = call_resp
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
        duration_ms: 0,
    })
}

async fn send_request(
    stdin: &mut tokio::process::ChildStdin,
    request: &JsonRpcRequest,
) -> Result<(), ToolError> {
    let serialized = serde_json::to_string(request)
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to serialize request: {e}")))?;

    stdin
        .write_all(serialized.as_bytes())
        .await
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to write to stdin: {e}")))?;

    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to write newline: {e}")))?;

    stdin
        .flush()
        .await
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to flush stdin: {e}")))?;

    Ok(())
}

async fn read_response(
    reader: &mut BufReader<tokio::process::ChildStdout>,
) -> Result<JsonRpcResponse, ToolError> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ToolError::McpConnectionFailed(format!("Failed to read from stdout: {e}")))?;

    if line.is_empty() {
        return Err(ToolError::McpProcessCrashed(
            "Process closed stdout unexpectedly".to_string(),
        ));
    }

    serde_json::from_str(&line)
        .map_err(|e| ToolError::McpInvalidResponse(format!("Invalid JSON-RPC response: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_request_serialization() {
        let req = JsonRpcRequest::tools_call(1, "test", &serde_json::json!({}));
        let serialized = serde_json::to_string(&req).expect("should serialize");
        assert!(serialized.contains("tools/call"));
    }
}
