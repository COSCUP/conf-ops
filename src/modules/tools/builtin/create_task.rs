use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: create a task from a template.
pub struct CreateTaskTool {
    _pool: PgPool,
}

impl CreateTaskTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for CreateTaskTool {
    fn name(&self) -> &'static str {
        "createTask"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "createTask".to_string(),
            display_name: Some("Create Task".to_string()),
            description: "Create a new task from a task template".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "templateId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Task template ID"
                    },
                    "ownerTagId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Owner member tag ID"
                    },
                    "name": {
                        "type": "string",
                        "description": "Task name (optional, uses template name if omitted)"
                    }
                },
                "required": ["templateId", "ownerTagId"]
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
        let template_id = params
            .get("templateId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing templateId".to_string()))?;

        let owner_tag_id = params
            .get("ownerTagId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing ownerTagId".to_string()))?;

        // Placeholder: In production, this would call TaskService to create the task.
        // For now, return a stub result indicating success.
        Ok(ToolResult {
            content: serde_json::json!({
                "status": "created",
                "templateId": template_id,
                "ownerTagId": owner_tag_id,
                "name": params.get("name").and_then(|v| v.as_str()).unwrap_or("New Task"),
                "message": "Task creation will be connected to TaskService in production"
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

    fn test_context() -> ToolContext {
        ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        }
    }

    #[tokio::test]
    async fn tool_name() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap_or_else(|_| {
            panic!("lazy pool should work even without DB");
        });
        let tool = CreateTaskTool::new(pool);
        assert_eq!(tool.name(), "createTask");
    }

    #[tokio::test]
    async fn tool_definition_has_required_fields() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap_or_else(|_| {
            panic!("lazy pool");
        });
        let tool = CreateTaskTool::new(pool);
        let def = tool.definition();
        assert_eq!(def.name, "createTask");
        assert_eq!(def.category, ToolCategory::Core);
        assert!(def.requires_confirmation);
    }

    #[tokio::test]
    async fn execute_missing_template_id_returns_error() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap_or_else(|_| {
            panic!("lazy pool");
        });
        let tool = CreateTaskTool::new(pool);
        let result = tool.execute(serde_json::json!({}), &test_context()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_with_valid_params_returns_result() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap_or_else(|_| {
            panic!("lazy pool");
        });
        let tool = CreateTaskTool::new(pool);
        let result = tool
            .execute(
                serde_json::json!({
                    "templateId": Uuid::now_v7().to_string(),
                    "ownerTagId": Uuid::now_v7().to_string()
                }),
                &test_context(),
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
