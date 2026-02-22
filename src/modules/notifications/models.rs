use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Notification Type ──────────────────────────────────────

/// The type of a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// A todo was assigned to you.
    TodoAssigned,
    /// You were @mentioned by a member.
    MemberMentioned,
    /// A tag you belong to was @mentioned.
    TagMentioned,
    /// A task you participated in was completed.
    TaskCompleted,
    /// A linked sub-task was completed.
    LinkedTaskCompleted,
    /// Source data table data changed.
    SourceDataChanged,
    /// A scheduled reminder triggered.
    Reminder,
}

impl NotificationType {
    /// Returns the string representation of this notification type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TodoAssigned => "todo_assigned",
            Self::MemberMentioned => "member_mentioned",
            Self::TagMentioned => "tag_mentioned",
            Self::TaskCompleted => "task_completed",
            Self::LinkedTaskCompleted => "linked_task_completed",
            Self::SourceDataChanged => "source_data_changed",
            Self::Reminder => "reminder",
        }
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NotificationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo_assigned" => Ok(Self::TodoAssigned),
            "member_mentioned" => Ok(Self::MemberMentioned),
            "tag_mentioned" => Ok(Self::TagMentioned),
            "task_completed" => Ok(Self::TaskCompleted),
            "linked_task_completed" => Ok(Self::LinkedTaskCompleted),
            "source_data_changed" => Ok(Self::SourceDataChanged),
            "reminder" => Ok(Self::Reminder),
            other => Err(format!("Invalid notification type: {other}")),
        }
    }
}

// ── Reference Type ─────────────────────────────────────────

/// The type of resource a notification references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationReferenceType {
    Task,
    Todo,
    Message,
}

impl NotificationReferenceType {
    /// Returns the string representation of this reference type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Todo => "todo",
            Self::Message => "message",
        }
    }
}

impl std::fmt::Display for NotificationReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NotificationReferenceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task" => Ok(Self::Task),
            "todo" => Ok(Self::Todo),
            "message" => Ok(Self::Message),
            other => Err(format!("Invalid notification reference type: {other}")),
        }
    }
}

// ── Delivery Channel ───────────────────────────────────────

/// A notification delivery channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryChannel {
    InApp,
    WebPush,
    Email,
}

impl DeliveryChannel {
    /// Returns the string representation of this delivery channel.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::WebPush => "web_push",
            Self::Email => "email",
        }
    }
}

impl std::fmt::Display for DeliveryChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Notification (DB Row) ──────────────────────────────────

/// A notification record from the database.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub account_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub body: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub delivered_channels: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ── Notification Preferences (DB Row) ──────────────────────

/// Notification preferences record from the database.
#[derive(Debug, Clone)]
pub struct NotificationPreference {
    pub id: Uuid,
    pub account_id: Uuid,
    pub preferences: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

// ── Web Push Subscription (DB Row) ─────────────────────────

/// A Web Push subscription record from the database.
#[derive(Debug, Clone)]
pub struct WebPushSubscription {
    pub id: Uuid,
    pub account_id: Uuid,
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub device_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ── Scheduled Reminder (DB Row) ────────────────────────────

/// A scheduled reminder record from the database.
#[derive(Debug, Clone)]
pub struct ScheduledReminder {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub reminder_type: String,
    pub trigger_at: DateTime<Utc>,
    pub fired: bool,
    pub fired_at: Option<DateTime<Utc>>,
    pub notification_id: Option<Uuid>,
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Reminder Type ──────────────────────────────────────────

/// The type of a scheduled reminder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReminderType {
    /// Due date is approaching.
    DueDateApproaching,
    /// Due date has passed.
    DueDateOverdue,
    /// Todo has been stale for too long.
    TodoStale,
}

impl ReminderType {
    /// Returns the string representation of this reminder type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DueDateApproaching => "due_date_approaching",
            Self::DueDateOverdue => "due_date_overdue",
            Self::TodoStale => "todo_stale",
        }
    }
}

impl std::fmt::Display for ReminderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ReminderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "due_date_approaching" => Ok(Self::DueDateApproaching),
            "due_date_overdue" => Ok(Self::DueDateOverdue),
            "todo_stale" => Ok(Self::TodoStale),
            other => Err(format!("Invalid reminder type: {other}")),
        }
    }
}

// ── Create Params ──────────────────────────────────────────

/// Parameters for creating a notification.
pub struct CreateNotificationParams {
    pub id: Uuid,
    pub account_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: Option<String>,
    pub reference_type: Option<NotificationReferenceType>,
    pub reference_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub delivered_channels: Vec<DeliveryChannel>,
}

/// Parameters for creating a web push subscription.
pub struct CreateWebPushSubscriptionParams {
    pub id: Uuid,
    pub account_id: Uuid,
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub device_name: Option<String>,
}

/// Parameters for creating a scheduled reminder.
pub struct CreateScheduledReminderParams {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub reminder_type: ReminderType,
    pub trigger_at: DateTime<Utc>,
    pub config: Option<serde_json::Value>,
}

// ── Notification Category Mapping ──────────────────────────

/// Maps a notification type to its preference category key.
pub fn notification_type_to_category(notification_type: &NotificationType) -> &'static str {
    match notification_type {
        NotificationType::TaskCompleted
        | NotificationType::LinkedTaskCompleted
        | NotificationType::SourceDataChanged => "taskUpdates",
        NotificationType::TodoAssigned => "todoAssignments",
        NotificationType::MemberMentioned | NotificationType::TagMentioned => "mentions",
        NotificationType::Reminder => "systemAnnouncements",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_type_serde_roundtrip() {
        let variants = vec![
            (NotificationType::TodoAssigned, "\"todo_assigned\""),
            (NotificationType::MemberMentioned, "\"member_mentioned\""),
            (NotificationType::TagMentioned, "\"tag_mentioned\""),
            (NotificationType::TaskCompleted, "\"task_completed\""),
            (
                NotificationType::LinkedTaskCompleted,
                "\"linked_task_completed\"",
            ),
            (
                NotificationType::SourceDataChanged,
                "\"source_data_changed\"",
            ),
            (NotificationType::Reminder, "\"reminder\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: NotificationType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn notification_type_from_str() {
        assert_eq!(
            "todo_assigned".parse::<NotificationType>().unwrap(),
            NotificationType::TodoAssigned
        );
        assert!("invalid".parse::<NotificationType>().is_err());
    }

    #[test]
    fn reference_type_serde_roundtrip() {
        let variants = vec![
            (NotificationReferenceType::Task, "\"task\""),
            (NotificationReferenceType::Todo, "\"todo\""),
            (NotificationReferenceType::Message, "\"message\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: NotificationReferenceType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn delivery_channel_display() {
        assert_eq!(DeliveryChannel::InApp.to_string(), "in_app");
        assert_eq!(DeliveryChannel::WebPush.to_string(), "web_push");
        assert_eq!(DeliveryChannel::Email.to_string(), "email");
    }

    #[test]
    fn reminder_type_serde_roundtrip() {
        let variants = vec![
            (ReminderType::DueDateApproaching, "\"due_date_approaching\""),
            (ReminderType::DueDateOverdue, "\"due_date_overdue\""),
            (ReminderType::TodoStale, "\"todo_stale\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: ReminderType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn notification_type_to_category_mapping() {
        assert_eq!(
            notification_type_to_category(&NotificationType::TaskCompleted),
            "taskUpdates"
        );
        assert_eq!(
            notification_type_to_category(&NotificationType::TodoAssigned),
            "todoAssignments"
        );
        assert_eq!(
            notification_type_to_category(&NotificationType::MemberMentioned),
            "mentions"
        );
        assert_eq!(
            notification_type_to_category(&NotificationType::Reminder),
            "systemAnnouncements"
        );
    }
}
