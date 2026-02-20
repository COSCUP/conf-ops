use sqlx::PgPool;
use uuid::Uuid;

use super::error::DataSheetError;
use super::models::DataEntry;

pub struct DataEntryRepository;

impl DataEntryRepository {
    /// Upsert a data entry (insert or update on conflict).
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn upsert(
        pool: &PgPool,
        id: Uuid,
        task_id: Uuid,
        data_schema_id: Uuid,
        values: &serde_json::Value,
    ) -> Result<DataEntry, DataSheetError> {
        sqlx::query_as!(
            DataEntry,
            r#"INSERT INTO data_entries (id, task_id, data_schema_id, values)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (task_id, data_schema_id)
             DO UPDATE SET values = $4, updated_at = NOW()
             RETURNING id, task_id, data_schema_id, values,
                       source_links, created_at, updated_at, deleted_at"#,
            id,
            task_id,
            data_schema_id,
            values,
        )
        .fetch_one(pool)
        .await
        .map_err(DataSheetError::from)
    }

    /// Get a data entry by task ID and schema ID.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::NotFound` if the entry does not exist.
    pub async fn get_by_task_and_schema(
        pool: &PgPool,
        task_id: Uuid,
        data_schema_id: Uuid,
    ) -> Result<DataEntry, DataSheetError> {
        sqlx::query_as!(
            DataEntry,
            r#"SELECT id, task_id, data_schema_id, values,
                      source_links, created_at, updated_at, deleted_at
             FROM data_entries
             WHERE task_id = $1 AND data_schema_id = $2 AND deleted_at IS NULL"#,
            task_id,
            data_schema_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(DataSheetError::NotFound)
    }

    /// List all data entries for a task.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn list_by_task(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Vec<DataEntry>, DataSheetError> {
        let entries = sqlx::query_as!(
            DataEntry,
            r#"SELECT id, task_id, data_schema_id, values,
                      source_links, created_at, updated_at, deleted_at
             FROM data_entries
             WHERE task_id = $1 AND deleted_at IS NULL
             ORDER BY created_at ASC"#,
            task_id,
        )
        .fetch_all(pool)
        .await?;
        Ok(entries)
    }

    /// List all data entries for a schema (aggregation view).
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn list_by_schema(
        pool: &PgPool,
        data_schema_id: Uuid,
    ) -> Result<Vec<DataEntry>, DataSheetError> {
        let entries = sqlx::query_as!(
            DataEntry,
            r#"SELECT id, task_id, data_schema_id, values,
                      source_links, created_at, updated_at, deleted_at
             FROM data_entries
             WHERE data_schema_id = $1 AND deleted_at IS NULL
             ORDER BY created_at ASC"#,
            data_schema_id,
        )
        .fetch_all(pool)
        .await?;
        Ok(entries)
    }

    /// Soft-delete a data entry.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::NotFound` if the entry does not exist.
    pub async fn soft_delete(
        pool: &PgPool,
        task_id: Uuid,
        data_schema_id: Uuid,
    ) -> Result<(), DataSheetError> {
        let result = sqlx::query!(
            "UPDATE data_entries SET deleted_at = NOW(), updated_at = NOW()
             WHERE task_id = $1 AND data_schema_id = $2 AND deleted_at IS NULL",
            task_id,
            data_schema_id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DataSheetError::NotFound);
        }
        Ok(())
    }

    /// Update source links for a data entry.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn update_source_links(
        pool: &PgPool,
        task_id: Uuid,
        data_schema_id: Uuid,
        source_links: &serde_json::Value,
    ) -> Result<DataEntry, DataSheetError> {
        sqlx::query_as!(
            DataEntry,
            r#"UPDATE data_entries
             SET source_links = $3, updated_at = NOW()
             WHERE task_id = $1 AND data_schema_id = $2 AND deleted_at IS NULL
             RETURNING id, task_id, data_schema_id, values,
                       source_links, created_at, updated_at, deleted_at"#,
            task_id,
            data_schema_id,
            source_links,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(DataSheetError::NotFound)
    }
}
