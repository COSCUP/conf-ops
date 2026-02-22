use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

/// Errors that can occur in the tools module.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool config not found")]
    ConfigNotFound,

    #[error("Tool is disabled: {0}")]
    Disabled(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Placeholder resolution failed: {0}")]
    PlaceholderResolutionFailed(String),

    #[error("MCP connection failed: {0}")]
    McpConnectionFailed(String),

    #[error("MCP timeout")]
    McpTimeout,

    #[error("MCP invalid response: {0}")]
    McpInvalidResponse(String),

    #[error("Tool execution error: {0}")]
    ExecutionError(String),

    #[error("MCP process crashed: {0}")]
    McpProcessCrashed(String),

    #[error("Invalid tool configuration: {0}")]
    InvalidConfig(String),

    #[error("Duplicate tool config")]
    DuplicateConfig,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<ToolError> for ProblemDetails {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::NotFound(_) | ToolError::ConfigNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            ToolError::Disabled(_) => {
                Self::new(StatusCode::FORBIDDEN, "Tool Disabled").with_detail(err.to_string())
            }
            ToolError::PermissionDenied(_) => {
                Self::new(StatusCode::FORBIDDEN, "Permission Denied").with_detail(err.to_string())
            }
            ToolError::PlaceholderResolutionFailed(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Placeholder Resolution Failed")
                    .with_detail(err.to_string())
            }
            ToolError::McpConnectionFailed(_) | ToolError::McpProcessCrashed(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "MCP Server Error").with_detail(err.to_string())
            }
            ToolError::McpTimeout => {
                Self::new(StatusCode::BAD_GATEWAY, "MCP Timeout").with_detail(err.to_string())
            }
            ToolError::McpInvalidResponse(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "MCP Invalid Response")
                    .with_detail(err.to_string())
            }
            ToolError::ExecutionError(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Tool Execution Error")
                    .with_detail(err.to_string())
            }
            ToolError::InvalidConfig(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Tool Configuration")
                    .with_detail(err.to_string())
            }
            ToolError::DuplicateConfig => {
                Self::new(StatusCode::CONFLICT, "Tool Config Already Exists")
                    .with_detail("A tool configuration with this name already exists in this scope")
            }
            ToolError::Database(_) => Self::internal_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_not_found_maps_to_404() {
        let err = ToolError::NotFound("createTask".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
    }

    #[test]
    fn permission_denied_maps_to_403() {
        let err = ToolError::PermissionDenied("missing tag".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 403);
    }

    #[test]
    fn mcp_timeout_maps_to_502() {
        let err = ToolError::McpTimeout;
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 502);
    }

    #[test]
    fn duplicate_config_maps_to_409() {
        let err = ToolError::DuplicateConfig;
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 409);
    }
}
