use async_trait::async_trait;

use crate::modules::tools::error::ToolError;
use crate::modules::tools::executor::BuiltinToolExecutor;
use crate::modules::tools::models::{ToolCategory, ToolContext, ToolDefinition, ToolResult};

/// Configurable builtin tool: create a Google Meet meeting.
#[derive(Default)]
pub struct GoogleMeetTool;

#[async_trait]
impl BuiltinToolExecutor for GoogleMeetTool {
    fn name(&self) -> &'static str {
        "googleMeet/createMeeting"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "googleMeet/createMeeting".to_string(),
            display_name: Some("Create Google Meet".to_string()),
            description: "Create a Google Meet meeting via Google Calendar API".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Meeting title"
                    },
                    "startTime": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Meeting start time (ISO 8601)"
                    },
                    "endTime": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Meeting end time (ISO 8601)"
                    },
                    "attendees": {
                        "type": "array",
                        "items": { "type": "string", "format": "email" },
                        "description": "Attendee email addresses"
                    }
                },
                "required": ["title", "startTime", "endTime"]
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

        let start_time = params
            .get("startTime")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing startTime".to_string()))?;

        let end_time = params
            .get("endTime")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing endTime".to_string()))?;

        Ok(ToolResult {
            content: serde_json::json!({
                "status": "created",
                "title": title,
                "startTime": start_time,
                "endTime": end_time,
                "message": "Google Meet creation will be connected to Google Calendar API in production"
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
        let tool = GoogleMeetTool::default();
        assert_eq!(tool.name(), "googleMeet/createMeeting");
        assert_eq!(tool.definition().category, ToolCategory::Configurable);
    }

    #[tokio::test]
    async fn execute_with_valid_params() {
        let tool = GoogleMeetTool::default();
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
                    "title": "Sprint Planning",
                    "startTime": "2026-01-15T10:00:00Z",
                    "endTime": "2026-01-15T11:00:00Z"
                }),
                &ctx,
            )
            .await
            .expect("should succeed");
        assert!(!result.is_error);
    }
}
