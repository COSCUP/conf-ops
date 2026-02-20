use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct MemberTag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub external_task_creation: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MemberTagAssignment {
    pub id: Uuid,
    pub tag_id: Uuid,
    pub member_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub project_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Read-model projection: tag with assignment counts.
#[derive(Debug, Clone, FromRow)]
pub struct MemberTagListItem {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub member_count: i64,
    pub contact_count: i64,
}

/// Read-model projection: assigned member with account info (JOIN accounts).
#[derive(Debug, Clone, FromRow)]
pub struct TagAssignedMember {
    pub assignment_id: Uuid,
    pub member_id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

/// Read-model projection: assigned contact.
#[derive(Debug, Clone, FromRow)]
pub struct TagAssignedContact {
    pub assignment_id: Uuid,
    pub contact_id: Uuid,
    pub name: String,
    pub email: String,
}
