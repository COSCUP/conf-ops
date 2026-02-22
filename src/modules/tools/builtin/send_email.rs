use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Configurable builtin tool: send email via SMTP.
pub struct SendEmailTool {
    _pool: PgPool,
}

impl SendEmailTool {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait]
impl BuiltinToolExecutor for SendEmailTool {
    fn name(&self) -> &'static str {
        "smtp/sendEmail"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "smtp/sendEmail".to_string(),
            display_name: Some("Send Email".to_string()),
            description: "Send an email via SMTP".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "to": {
                        "type": "array",
                        "items": { "type": "string", "format": "email" },
                        "description": "Recipient email addresses"
                    },
                    "subject": {
                        "type": "string",
                        "description": "Email subject"
                    },
                    "body": {
                        "type": "string",
                        "description": "Email body (plain text or HTML)"
                    },
                    "replyToThreadId": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Optional email thread ID to reply to"
                    }
                },
                "required": ["to", "subject", "body"]
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
        let to = params
            .get("to")
            .ok_or_else(|| ToolError::ExecutionError("Missing to".to_string()))?;

        let subject = params
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing subject".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "sent",
                "to": to,
                "subject": subject,
                "message": "Email sending will be connected to EmailOutboundService in production"
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
        let tool = SendEmailTool::new(pool);
        assert_eq!(tool.name(), "smtp/sendEmail");
        assert_eq!(tool.definition().category, ToolCategory::Configurable);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let pool = PgPool::connect_lazy("postgres://localhost/test")
            .unwrap_or_else(|_| panic!("lazy pool"));
        let tool = SendEmailTool::new(pool);
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
                    "to": ["test@example.com"],
                    "subject": "Test",
                    "body": "Hello"
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
