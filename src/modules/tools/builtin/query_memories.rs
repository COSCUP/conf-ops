use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Core tool: query memories at a given scope.
pub struct QueryMemoriesTool {
    _pool: PgPool,
}

impl QueryMemoriesTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for QueryMemoriesTool {
    fn name(&self) -> &'static str {
        "queryMemories"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "queryMemories".to_string(),
            display_name: Some("Query Memories".to_string()),
            description: "Search and retrieve memory entries at the specified scope".to_string(),
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
                    "search": {
                        "type": "string",
                        "description": "Optional search query to filter memories"
                    }
                },
                "required": ["scopeType", "scopeId"]
            }),
            category: ToolCategory::Core,
            requires_confirmation: false,
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

        Ok(ToolResult {
            content: serde_json::json!({
                "scopeType": scope_type,
                "scopeId": scope_id,
                "memories": [],
                "message": "Memory query will be connected to MemoryService in production"
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
        let tool = QueryMemoriesTool::new(pool);
        assert_eq!(tool.name(), "queryMemories");
        assert_eq!(tool.definition().category, ToolCategory::Core);
        assert!(!tool.definition().requires_confirmation);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = QueryMemoriesTool::new(pool);
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
                    "scopeType": "project",
                    "scopeId": Uuid::now_v7().to_string()
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
