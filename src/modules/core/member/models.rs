use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "member_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    TagAdmin,
    Member,
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner => write!(f, "owner"),
            Self::TagAdmin => write!(f, "tag_admin"),
            Self::Member => write!(f, "member"),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Member {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: Uuid,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Read-model projection: member with account info (JOIN accounts).
#[derive(Debug, Clone, FromRow)]
pub struct MemberWithAccount {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: Uuid,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}
