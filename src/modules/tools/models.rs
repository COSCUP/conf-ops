use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Tool Scope Type ─────────────────────────────────────────

/// The scope at which a tool configuration applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolScopeType {
    Organization,
    Project,
}

impl ToolScopeType {
    /// Returns the string representation of this scope type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::Project => "project",
        }
    }
}

impl std::fmt::Display for ToolScopeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ToolScopeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "organization" => Ok(Self::Organization),
            "project" => Ok(Self::Project),
            other => Err(format!("Invalid tool scope type: {other}")),
        }
    }
}

// ── Tool Type ───────────────────────────────────────────────

/// The type of a tool: builtin or external (MCP Server).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Builtin,
    External,
}

impl ToolType {
    /// Returns the string representation of this tool type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::External => "external",
        }
    }
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ToolType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "builtin" => Ok(Self::Builtin),
            "external" => Ok(Self::External),
            other => Err(format!("Invalid tool type: {other}")),
        }
    }
}

// ── Tool Category ───────────────────────────────────────────

/// The category of a tool in the MCP runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// Core tools that are always available.
    Core,
    /// Configurable builtin tools that need configuration.
    Configurable,
    /// External MCP Server tools.
    External,
}

impl ToolCategory {
    /// Returns the string representation of this category.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Configurable => "configurable",
            Self::External => "external",
        }
    }
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Tool Execution Status ───────────────────────────────────

/// The status of a tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Success,
    Error,
}

impl ToolExecutionStatus {
    /// Returns the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for ToolExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ToolExecutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "error" => Ok(Self::Error),
            other => Err(format!("Invalid tool execution status: {other}")),
        }
    }
}

// ── MCP Transport ───────────────────────────────────────────

/// The transport type for an external MCP Server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Sse,
}

// ── MCP Server Config ───────────────────────────────────────

/// MCP Server connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// Transport type: stdio or sse.
    pub transport: McpTransport,
    /// Command to start the MCP server (stdio mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// URL to connect to the MCP server (sse mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Arguments for the MCP server command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Environment variables for the MCP server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
    /// Auth header for SSE mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<String>,
}

// ── Tool Config (DB Row) ────────────────────────────────────

/// A tool configuration stored in the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ToolConfig {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub tool_type: String,
    pub tool_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub mcp_server_config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// ── Tool Execution (DB Row) ─────────────────────────────────

/// A tool execution record stored in the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ToolExecution {
    pub id: Uuid,
    pub task_id: Uuid,
    pub suggestion_id: Option<Uuid>,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: serde_json::Value,
    pub status: String,
    pub executed_by: Uuid,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: i32,
}

// ── Tool Definition (MCP format) ────────────────────────────

/// An MCP tool definition exposed to clients.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// Tool identifier name.
    pub name: String,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Tool description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// Tool category.
    pub category: ToolCategory,
    /// Whether the tool requires human confirmation before execution.
    pub requires_confirmation: bool,
}

// ── Tool Result ─────────────────────────────────────────────

/// The result of a tool execution (MCP format).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Result content.
    pub content: serde_json::Value,
    /// Whether the execution resulted in an error.
    pub is_error: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

// ── Tool Context ────────────────────────────────────────────

/// Context for tool execution containing identifiers and authorization data.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub organization_id: Uuid,
    pub actor_id: Uuid,
    pub actor_tags: Vec<Uuid>,
}

// ── Create/Update Params ────────────────────────────────────

/// Parameters for creating a tool configuration.
pub struct CreateToolConfigParams {
    pub id: Uuid,
    pub scope_type: ToolScopeType,
    pub scope_id: Uuid,
    pub tool_type: ToolType,
    pub tool_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub mcp_server_config: Option<serde_json::Value>,
}

/// Parameters for updating a tool configuration.
pub struct UpdateToolConfigParams {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
    pub mcp_server_config: Option<serde_json::Value>,
}

/// Parameters for recording a tool execution.
pub struct RecordToolExecutionParams {
    pub id: Uuid,
    pub task_id: Uuid,
    pub suggestion_id: Option<Uuid>,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: serde_json::Value,
    pub status: ToolExecutionStatus,
    pub executed_by: Uuid,
    pub duration_ms: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_scope_type_serde_roundtrip() {
        let variants = vec![
            (ToolScopeType::Organization, "\"organization\""),
            (ToolScopeType::Project, "\"project\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: ToolScopeType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn tool_type_serde_roundtrip() {
        let variants = vec![
            (ToolType::Builtin, "\"builtin\""),
            (ToolType::External, "\"external\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: ToolType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn tool_execution_status_display_and_from_str() {
        let variants = vec![
            (ToolExecutionStatus::Success, "success"),
            (ToolExecutionStatus::Error, "error"),
        ];

        for (variant, expected_str) in variants {
            assert_eq!(variant.to_string(), expected_str);
            let parsed: ToolExecutionStatus = expected_str.parse().expect("should parse");
            assert_eq!(parsed, variant);
        }

        let err = "invalid".parse::<ToolExecutionStatus>();
        assert!(err.is_err());
    }

    #[test]
    fn tool_definition_serde_roundtrip() {
        let def = ToolDefinition {
            name: "createTask".to_string(),
            display_name: Some("Create Task".to_string()),
            description: "Create a new task from a template".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "templateId": { "type": "string", "format": "uuid" }
                },
                "required": ["templateId"]
            }),
            category: ToolCategory::Core,
            requires_confirmation: true,
        };

        let serialized = serde_json::to_string(&def).expect("should serialize");
        let deserialized: ToolDefinition =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(deserialized.name, "createTask");
        assert_eq!(deserialized.category, ToolCategory::Core);
        assert!(deserialized.requires_confirmation);
    }

    #[test]
    fn tool_result_serde_roundtrip() {
        let result = ToolResult {
            content: serde_json::json!({"taskId": "abc-123"}),
            is_error: false,
            duration_ms: 42,
        };

        let serialized = serde_json::to_string(&result).expect("should serialize");
        let deserialized: ToolResult =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert!(!deserialized.is_error);
        assert_eq!(deserialized.duration_ms, 42);
    }

    #[test]
    fn mcp_server_config_serde_stdio() {
        let config = McpServerConfig {
            transport: McpTransport::Stdio,
            command: Some("/usr/bin/mcp-server".to_string()),
            url: None,
            args: Some(vec!["--flag".to_string()]),
            env: Some(serde_json::json!({"API_KEY": "test"})),
            auth_header: None,
        };

        let serialized = serde_json::to_string(&config).expect("should serialize");
        let deserialized: McpServerConfig =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(deserialized.transport, McpTransport::Stdio);
        assert_eq!(deserialized.command.as_deref(), Some("/usr/bin/mcp-server"));
    }

    #[test]
    fn mcp_server_config_serde_sse() {
        let config = McpServerConfig {
            transport: McpTransport::Sse,
            command: None,
            url: Some("https://mcp.example.com/sse".to_string()),
            args: None,
            env: None,
            auth_header: Some("Bearer token123".to_string()),
        };

        let serialized = serde_json::to_string(&config).expect("should serialize");
        let deserialized: McpServerConfig =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(deserialized.transport, McpTransport::Sse);
        assert_eq!(
            deserialized.url.as_deref(),
            Some("https://mcp.example.com/sse")
        );
    }
}
