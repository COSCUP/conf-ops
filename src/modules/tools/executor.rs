use async_trait::async_trait;

use super::error::ToolError;
use super::models::{ToolContext, ToolDefinition, ToolResult};

/// Trait for executing a builtin tool.
#[async_trait]
pub trait BuiltinToolExecutor: Send + Sync {
    /// Returns the tool identifier name.
    fn name(&self) -> &'static str;

    /// Returns the MCP tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given parameters and context.
    ///
    /// # Errors
    ///
    /// Returns `ToolError` on execution failure.
    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::tools::models::ToolCategory;
    use uuid::Uuid;

    struct MockExecutor;

    #[async_trait]
    impl BuiltinToolExecutor for MockExecutor {
        fn name(&self) -> &'static str {
            "mockTool"
        }

        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "mockTool".to_string(),
                display_name: Some("Mock Tool".to_string()),
                description: "A mock tool for testing".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                category: ToolCategory::Core,
                requires_confirmation: false,
            }
        }

        async fn execute(
            &self,
            _params: serde_json::Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult {
                content: serde_json::json!({"result": "ok"}),
                is_error: false,
                duration_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn mock_executor_works() {
        let executor = MockExecutor;
        let context = ToolContext {
            task_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            actor_id: Uuid::now_v7(),
            actor_tags: vec![],
        };

        let result = executor
            .execute(serde_json::json!({}), &context)
            .await
            .expect("should succeed");

        assert!(!result.is_error);
        assert_eq!(executor.name(), "mockTool");
        assert_eq!(executor.definition().category, ToolCategory::Core);
    }
}
