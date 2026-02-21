use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "file_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Pending,
    Confirmed,
    Rejected,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileRecord {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub storage_path: String,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub status: FileStatus,
    pub uploaded_by: Uuid,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A key-value metadata entry attached to a file.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileMetadata {
    pub id: Uuid,
    pub file_id: Uuid,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}
