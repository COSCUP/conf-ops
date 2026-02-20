use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DataEntry {
    pub id: Uuid,
    pub task_id: Uuid,
    pub data_schema_id: Uuid,
    pub values: serde_json::Value,
    pub source_links: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLink {
    pub source_task_id: Uuid,
    pub source_schema_id: Uuid,
    pub field_keys: Vec<String>,
    pub shared_at: DateTime<Utc>,
    pub shared_by: Uuid,
}
