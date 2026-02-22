use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Email Direction ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailDirection {
    Inbound,
    Outbound,
}

impl EmailDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }
}

impl std::fmt::Display for EmailDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EmailDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbound" => Ok(Self::Inbound),
            "outbound" => Ok(Self::Outbound),
            other => Err(format!("Invalid email direction: {other}")),
        }
    }
}

// ── Send Status ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendStatus {
    Sent,
    Failed,
    Pending,
}

impl SendStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::Pending => "pending",
        }
    }
}

impl std::fmt::Display for SendStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SendStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sent" => Ok(Self::Sent),
            "failed" => Ok(Self::Failed),
            "pending" => Ok(Self::Pending),
            other => Err(format!("Invalid send status: {other}")),
        }
    }
}

// ── Email Thread ─────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailThread {
    pub id: Uuid,
    pub task_id: Uuid,
    pub subject: String,
    pub participants: serde_json::Value,
    pub message_ids: serde_json::Value,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateEmailThreadParams {
    pub id: Uuid,
    pub task_id: Uuid,
    pub subject: String,
    pub participants: serde_json::Value,
}

pub struct UpdateEmailThreadParams {
    pub subject: Option<String>,
    pub participants: Option<serde_json::Value>,
}

// ── Email Message ────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailMessage {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub from_address: String,
    pub to_addresses: serde_json::Value,
    pub cc_addresses: serde_json::Value,
    pub subject: String,
    pub direction: String,
    pub conversation_message_id: Option<Uuid>,
    pub raw_headers: Option<serde_json::Value>,
    pub send_status: String,
    pub retry_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct CreateEmailMessageParams {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub from_address: String,
    pub to_addresses: serde_json::Value,
    pub cc_addresses: serde_json::Value,
    pub subject: String,
    pub direction: EmailDirection,
    pub conversation_message_id: Option<Uuid>,
    pub raw_headers: Option<serde_json::Value>,
    pub send_status: SendStatus,
}

// ── Unassigned Email ─────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UnassignedEmail {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_name: Option<String>,
    pub from_address: String,
    pub subject: String,
    pub snippet: Option<String>,
    pub raw_mime: Vec<u8>,
    pub has_attachments: bool,
    pub received_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub assigned_task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

pub struct CreateUnassignedEmailParams {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_name: Option<String>,
    pub from_address: String,
    pub subject: String,
    pub snippet: Option<String>,
    pub raw_mime: Vec<u8>,
    pub has_attachments: bool,
}
