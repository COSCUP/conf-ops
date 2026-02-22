//! MCP (Model Context Protocol) JSON-RPC message types.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC 2.0 request.
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }

    /// Create an `initialize` request.
    pub fn initialize(id: u64) -> Self {
        Self::new(
            id,
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "conf-ops",
                    "version": "0.1.0"
                }
            })),
        )
    }

    /// Create a `tools/list` request.
    pub fn tools_list(id: u64) -> Self {
        Self::new(id, "tools/list", None)
    }

    /// Create a `tools/call` request.
    pub fn tools_call(id: u64, name: &str, arguments: &serde_json::Value) -> Self {
        Self::new(
            id,
            "tools/call",
            Some(serde_json::json!({
                "name": name,
                "arguments": arguments
            })),
        )
    }
}

/// A JSON-RPC 2.0 response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP tool content returned from a `tools/call` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

/// MCP `tools/call` result containing content and error flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    pub content: Vec<McpToolContent>,
    #[serde(default)]
    pub is_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_rpc_request_initialize() {
        let req = JsonRpcRequest::initialize(1);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "initialize");
        assert!(req.params.is_some());
    }

    #[test]
    fn json_rpc_request_tools_list() {
        let req = JsonRpcRequest::tools_list(2);
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_none());
    }

    #[test]
    fn json_rpc_request_tools_call() {
        let req = JsonRpcRequest::tools_call(3, "myTool", &serde_json::json!({"key": "val"}));
        assert_eq!(req.method, "tools/call");
        let params = req.params.expect("should have params");
        assert_eq!(params["name"], "myTool");
        assert_eq!(params["arguments"]["key"], "val");
    }

    #[test]
    fn json_rpc_response_with_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"hello"}],"isError":false}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn json_rpc_response_with_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).expect("should parse");
        assert!(resp.error.is_some());
        let error = resp.error.expect("should have error");
        assert_eq!(error.code, -32600);
    }

    #[test]
    fn mcp_tool_result_deserialization() {
        let json = r#"{"content":[{"type":"text","text":"result data"}],"isError":false}"#;
        let result: McpToolResult = serde_json::from_str(json).expect("should parse");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text.as_deref(), Some("result data"));
    }

    #[test]
    fn mcp_tool_result_error() {
        let json = r#"{"content":[{"type":"text","text":"something went wrong"}],"isError":true}"#;
        let result: McpToolResult = serde_json::from_str(json).expect("should parse");
        assert!(result.is_error);
    }

    #[test]
    fn json_rpc_request_serialization_roundtrip() {
        let req = JsonRpcRequest::tools_call(5, "testTool", &serde_json::json!({"param": 42}));
        let serialized = serde_json::to_string(&req).expect("should serialize");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("should deserialize");
        assert_eq!(deserialized.id, 5);
        assert_eq!(deserialized.method, "tools/call");
    }
}
