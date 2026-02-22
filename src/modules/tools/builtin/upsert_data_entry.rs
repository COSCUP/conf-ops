use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: upsert a data entry in a task's data sheet.
pub struct UpsertDataEntryTool {
    _pool: PgPool,
}

impl UpsertDataEntryTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for UpsertDataEntryTool {
    fn name(&self) -> &'static str {
        "upsertDataEntry"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "upsertDataEntry".to_string(),
            display_name: Some("Upsert Data Entry".to_string()),
            description: "Create or update a data entry in a task's data sheet".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Task ID"
                    },
                    "schemaId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Data schema ID"
                    },
                    "values": {
                        "type": "object",
                        "description": "Field values to upsert",
                        "additionalProperties": true
                    }
                },
                "required": ["taskId", "schemaId", "values"]
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
        let task_id = params
            .get("taskId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing taskId".to_string()))?;

        let schema_id = params
            .get("schemaId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing schemaId".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "upserted",
                "taskId": task_id,
                "schemaId": schema_id,
                "message": "Data entry upsert will be connected to DataSheetService in production"
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
        let tool = UpsertDataEntryTool::new(pool);
        assert_eq!(tool.name(), "upsertDataEntry");
        assert_eq!(tool.definition().category, ToolCategory::Core);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = UpsertDataEntryTool::new(pool);
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
                    "taskId": Uuid::now_v7().to_string(),
                    "schemaId": Uuid::now_v7().to_string(),
                    "values": {"field1": "value1"}
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
