use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrdtOperation {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub operation: Vec<u8>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}
