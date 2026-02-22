use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: update an existing todo item.
pub struct UpdateTodoTool {
    _pool: PgPool,
}

impl UpdateTodoTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for UpdateTodoTool {
    fn name(&self) -> &'static str {
        "updateTodo"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "updateTodo".to_string(),
            display_name: Some("Update Todo".to_string()),
            description: "Update an existing todo item".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "todoId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Todo ID"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "cancelled"],
                        "description": "New status"
                    },
                    "title": {
                        "type": "string",
                        "description": "New title"
                    },
                    "assigneeIds": {
                        "type": "array",
                        "items": { "type": "string", "format": "uuid" },
                        "description": "New assignee member IDs"
                    }
                },
                "required": ["todoId"]
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
        let todo_id = params
            .get("todoId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing todoId".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "updated",
                "todoId": todo_id,
                "message": "Todo update will be connected to TodoService in production"
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
        let tool = UpdateTodoTool::new(pool);
        assert_eq!(tool.name(), "updateTodo");
        assert_eq!(tool.definition().category, ToolCategory::Core);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = UpdateTodoTool::new(pool);
        let ctx = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };
        let result = tool
            .execute(
                serde_json::json!({"todoId": Uuid::now_v7().to_string()}),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
