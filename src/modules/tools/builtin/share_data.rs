use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: share data entries between tasks.
pub struct ShareDataTool {
    _pool: PgPool,
}

impl ShareDataTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for ShareDataTool {
    fn name(&self) -> &'static str {
        "shareDataToTask"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shareDataToTask".to_string(),
            display_name: Some("Share Data to Task".to_string()),
            description: "Share data entries from one task to another task".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "sourceTaskId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Source task ID"
                    },
                    "targetTaskId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Target task ID"
                    },
                    "fieldMappings": {
                        "type": "object",
                        "description": "Mapping of source fields to target fields",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["sourceTaskId", "targetTaskId", "fieldMappings"]
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
        let source_task_id = params
            .get("sourceTaskId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing sourceTaskId".to_string()))?;

        let target_task_id = params
            .get("targetTaskId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing targetTaskId".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "shared",
                "sourceTaskId": source_task_id,
                "targetTaskId": target_task_id,
                "message": "Data sharing will be connected to DataSheetService in production"
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
        let tool = ShareDataTool::new(pool);
        assert_eq!(tool.name(), "shareDataToTask");
        assert_eq!(tool.definition().category, ToolCategory::Core);
        assert!(tool.definition().requires_confirmation);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = ShareDataTool::new(pool);
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
                    "sourceTaskId": Uuid::now_v7().to_string(),
                    "targetTaskId": Uuid::now_v7().to_string(),
                    "fieldMappings": {"name": "companyName"}
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
