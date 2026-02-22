use async_trait::async_trait;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Configurable builtin tool: create a `HackMD` document.
#[derive(Default)]
pub struct HackmdTool;

#[async_trait]
impl BuiltinToolExecutor for HackmdTool {
    fn name(&self) -> &'static str {
        "hackmd/createDocument"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "hackmd/createDocument".to_string(),
            display_name: Some("Create HackMD Document".to_string()),
            description: "Create a new HackMD collaborative document".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Document title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Initial document content (Markdown)"
                    },
                    "permission": {
                        "type": "string",
                        "enum": ["freely", "editable", "limited", "locked", "protected", "private"],
                        "description": "Document permission level"
                    }
                },
                "required": ["title"]
            }),
            category: ToolCategory::Configurable,
            requires_confirmation: true,
        }
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing title".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "created",
                "title": title,
                "message": "HackMD document creation will be connected to HackMD API in production"
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

    #[test]
    fn tool_name_and_definition() {
        let tool = HackmdTool::default();
        assert_eq!(tool.name(), "hackmd/createDocument");
        assert_eq!(tool.definition().category, ToolCategory::Configurable);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let tool = HackmdTool::default();
        let ctx = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };
        let result = tool
            .execute(serde_json::json!({"title": "Meeting Notes"}), &ctx)
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
