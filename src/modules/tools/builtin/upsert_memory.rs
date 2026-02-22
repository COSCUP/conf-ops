use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: upsert a memory entry.
pub struct UpsertMemoryTool {
    _pool: PgPool,
}

impl UpsertMemoryTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for UpsertMemoryTool {
    fn name(&self) -> &'static str {
        "upsertMemory"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "upsertMemory".to_string(),
            display_name: Some("Upsert Memory".to_string()),
            description: "Create or update a memory entry at the specified scope".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "scopeType": {
                        "type": "string",
                        "enum": ["account", "organization", "project", "member_tag", "task_template", "task"],
                        "description": "Memory scope type"
                    },
                    "scopeId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Scope ID"
                    },
                    "content": {
                        "type": "string",
                        "description": "Memory content"
                    }
                },
                "required": ["scopeType", "scopeId", "content"]
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
        let scope_type = params
            .get("scopeType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing scopeType".to_string()))?;

        let scope_id = params
            .get("scopeId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing scopeId".to_string()))?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing content".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "upserted",
                "scopeType": scope_type,
                "scopeId": scope_id,
                "contentLength": content.len(),
                "message": "Memory upsert will be connected to MemoryService in production"
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
        let tool = UpsertMemoryTool::new(pool);
        assert_eq!(tool.name(), "upsertMemory");
        assert_eq!(tool.definition().category, ToolCategory::Core);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = UpsertMemoryTool::new(pool);
        let ctx = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };
        let result = tool
            .execute(
                serde_json::json!({
                    "scopeType": "task",
                    "scopeId": Uuid::now_v7().to_string(),
                    "content": "Important context"
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
