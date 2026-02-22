use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Supported webhook event types.
pub const SUPPORTED_EVENT_TYPES: &[&str] = &[
    "task.created",
    "task.updated",
    "task.completed",
    "task.deleted",
    "todo.created",
    "todo.completed",
    "todo.updated",
    "data_entry.created",
    "data_entry.updated",
    "data_entry.deleted",
    "message.created",
    "member.added",
    "member.removed",
    "test",
];

/// Webhook event delivery status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventStatus {
    Pending,
    Success,
    Failed,
}

impl WebhookEventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for WebhookEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for WebhookEventStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            other => Err(format!("Invalid webhook event status: {other}")),
        }
    }
}

/// A webhook configuration record from the database.
#[derive(Debug, Clone)]
pub struct Webhook {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: serde_json::Value,
    pub enabled: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A webhook event log record from the database.
#[derive(Debug, Clone)]
pub struct WebhookEventLog {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Parameters for creating a webhook.
pub struct CreateWebhookParams {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: serde_json::Value,
    pub created_by: Uuid,
}

/// Parameters for updating a webhook.
pub struct UpdateWebhookParams {
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Parameters for creating a webhook event log entry.
pub struct CreateWebhookEventLogParams {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Check whether a given event type is valid.
pub fn is_valid_event_type(event_type: &str) -> bool {
    SUPPORTED_EVENT_TYPES.contains(&event_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_event_status_roundtrip() {
        let variants = vec![
            (WebhookEventStatus::Pending, "pending"),
            (WebhookEventStatus::Success, "success"),
            (WebhookEventStatus::Failed, "failed"),
        ];

        for (variant, expected) in variants {
            assert_eq!(variant.as_str(), expected);
            assert_eq!(variant.to_string(), expected);
            let parsed: WebhookEventStatus = expected.parse().unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn invalid_status_returns_error() {
        assert!("invalid".parse::<WebhookEventStatus>().is_err());
    }

    #[test]
    fn supported_event_types_contains_test() {
        assert!(is_valid_event_type("test"));
        assert!(is_valid_event_type("task.created"));
        assert!(!is_valid_event_type("invalid.event"));
    }
}
