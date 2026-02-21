use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use super::models::CrdtOperation;

pub struct CrdtRepository;

impl CrdtRepository {
    /// Insert a new CRDT operation.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn insert_operation(
        pool: &PgPool,
        id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        operation: &[u8],
        created_by: Uuid,
    ) -> Result<CrdtOperation, sqlx::Error> {
        Self::insert_operation_with_executor(
            pool,
            id,
            entity_type,
            entity_id,
            operation,
            created_by,
        )
        .await
    }

    /// Insert a new CRDT operation using any executor (pool or transaction).
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn insert_operation_with_executor<'e, E>(
        executor: E,
        id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        operation: &[u8],
        created_by: Uuid,
    ) -> Result<CrdtOperation, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as!(
            CrdtOperation,
            r#"INSERT INTO crdt_operations (id, entity_type, entity_id, operation, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, entity_type, entity_id, operation, created_by, created_at"#,
            id,
            entity_type,
            entity_id,
            operation,
            created_by,
        )
        .fetch_one(executor)
        .await
    }

    /// Get all CRDT operations for an entity, ordered by creation time.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn get_operations_by_entity(
        pool: &PgPool,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<CrdtOperation>, sqlx::Error> {
        sqlx::query_as!(
            CrdtOperation,
            r#"SELECT id, entity_type, entity_id, operation, created_by, created_at
             FROM crdt_operations
             WHERE entity_type = $1 AND entity_id = $2
             ORDER BY created_at ASC"#,
            entity_type,
            entity_id,
        )
        .fetch_all(pool)
        .await
    }

    /// List entity IDs that have more than `threshold` operations for a given entity type.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn list_entities_exceeding_threshold(
        pool: &PgPool,
        entity_type: &str,
        threshold: i64,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            r"SELECT entity_id
             FROM crdt_operations
             WHERE entity_type = $1
             GROUP BY entity_id
             HAVING COUNT(*) > $2",
        )
        .bind(entity_type)
        .bind(threshold)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Delete all CRDT operations for an entity using any executor.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn delete_operations_by_entity<'e, E>(
        executor: E,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM crdt_operations WHERE entity_type = $1 AND entity_id = $2")
                .bind(entity_type)
                .bind(entity_id)
                .execute(executor)
                .await?;

        Ok(result.rows_affected())
    }
}
