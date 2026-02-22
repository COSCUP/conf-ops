use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: save a key-value pair to the user's profile.
pub struct SaveToProfileTool {
    _pool: PgPool,
}

impl SaveToProfileTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for SaveToProfileTool {
    fn name(&self) -> &'static str {
        "saveToProfile"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "saveToProfile".to_string(),
            display_name: Some("Save to Profile".to_string()),
            description: "Save a key-value pair to the user's personal profile".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Profile key"
                    },
                    "value": {
                        "description": "Value to store (any JSON type)"
                    }
                },
                "required": ["key", "value"]
            }),
            category: ToolCategory::Core,
            requires_confirmation: true,
        }
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing key".to_string()))?;

        if params.get("value").is_none_or(serde_json::Value::is_null) {
            return Err(ToolError::ExecutionError("Missing value".to_string()));
        }

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "saved",
                "key": key,
                "message": "Profile save will be connected to AccountService in production"
            }),
            is_error: false,
            duration_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn tool_name_and_definition() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = SaveToProfileTool::new(pool);
        assert_eq!(tool.name(), "saveToProfile");
        assert_eq!(tool.definition().category, ToolCategory::Core);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = SaveToProfileTool::new(pool);
        let ctx = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };
        let result = tool
            .execute(
                serde_json::json!({"key": "nickname", "value": "Alice"}),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn execute_missing_value_returns_error() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = SaveToProfileTool::new(pool);
        let ctx = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };
        let result = tool
            .execute(serde_json::json!({"key": "nickname"}), &ctx)
            .await;
        assert!(result.is_err());
    }
}
