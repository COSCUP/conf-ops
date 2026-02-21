use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "message_source_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MessageSourceType {
    Member,
    AiSuggestion,
    ToolExecution,
    System,
    EmailInbound,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub task_id: Uuid,
    pub source_type: MessageSourceType,
    pub source_id: Option<Uuid>,
    pub content: serde_json::Value,
    pub attachments: Option<serde_json::Value>,
    pub action_result: Option<serde_json::Value>,
    pub last_seen_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConversationState {
    pub id: Uuid,
    pub task_id: Uuid,
    pub account_id: Uuid,
    pub last_read_message_id: Option<Uuid>,
    pub unread_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LastSeenPosition {
    pub id: Uuid,
    pub task_id: Uuid,
    pub account_id: Uuid,
    pub y_clock: i64,
    pub updated_at: DateTime<Utc>,
}

// ── Attachment structure ─────────────────────────────────────

/// A file attachment referenced in a conversation message.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachment {
    pub file_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub storage_path: String,
}

// ── Content type structures ──────────────────────────────────

/// Content for `source_type` = `member`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberContent {
    pub text: String,
    #[serde(default)]
    pub mentions: Vec<Mention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    #[serde(rename = "type")]
    pub mention_type: MentionType,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionType {
    Member,
    Tag,
}

/// Content for `source_type` = `system`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContent {
    pub event: String,
    pub details: serde_json::Value,
}

/// Content for `source_type` = `tool_execution`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionContent {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: serde_json::Value,
    pub status: ToolExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Success,
    Error,
}
