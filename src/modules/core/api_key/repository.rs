use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::ApiKeyError;
use super::models::{ApiKey, CreateApiKeyParams};

fn map_api_key(row: &sqlx::postgres::PgRow) -> Result<ApiKey, sqlx::Error> {
    Ok(ApiKey {
        id: row.try_get("id")?,
        project_id: row.try_get("project_id")?,
        name: row.try_get("name")?,
        key_hash: row.try_get("key_hash")?,
        permissions: row.try_get("permissions")?,
        created_by: row.try_get("created_by")?,
        last_used_at: row.try_get("last_used_at")?,
        created_at: row.try_get("created_at")?,
        deleted_at: row.try_get("deleted_at")?,
    })
}

pub struct ApiKeyRepository;

impl ApiKeyRepository {
    /// Create a new API key.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::Database` on database failure.
    pub async fn create(pool: &PgPool, params: &CreateApiKeyParams) -> Result<ApiKey, ApiKeyError> {
        let row = sqlx::query(
            r"INSERT INTO api_keys (id, project_id, name, key_hash, permissions, created_by)
              VALUES ($1, $2, $3, $4, $5, $6)
              RETURNING *",
        )
        .bind(params.id)
        .bind(params.project_id)
        .bind(&params.name)
        .bind(&params.key_hash)
        .bind(&params.permissions)
        .bind(params.created_by)
        .fetch_one(pool)
        .await?;

        map_api_key(&row).map_err(ApiKeyError::Database)
    }

    /// Find an API key by its hash.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::InvalidKey` if not found.
    pub async fn find_by_hash(pool: &PgPool, key_hash: &str) -> Result<ApiKey, ApiKeyError> {
        let row = sqlx::query("SELECT * FROM api_keys WHERE key_hash = $1 AND deleted_at IS NULL")
            .bind(key_hash)
            .fetch_optional(pool)
            .await?
            .ok_or(ApiKeyError::InvalidKey)?;

        map_api_key(&row).map_err(ApiKeyError::Database)
    }

    /// List active API keys for a project.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<ApiKey>, ApiKeyError> {
        let rows = sqlx::query(
            "SELECT * FROM api_keys WHERE project_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_api_key(r).map_err(ApiKeyError::Database))
            .collect()
    }

    /// Count active API keys for a project.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::Database` on database failure.
    pub async fn count_by_project(pool: &PgPool, project_id: Uuid) -> Result<i64, ApiKeyError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM api_keys WHERE project_id = $1 AND deleted_at IS NULL",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        row.try_get::<i64, _>("count")
            .map_err(ApiKeyError::Database)
    }

    /// Soft-delete an API key.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::NotFound` if not found.
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), ApiKeyError> {
        let result = sqlx::query(
            "UPDATE api_keys SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiKeyError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Update the last used timestamp for an API key.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::Database` on database failure.
    pub async fn touch_last_used(pool: &PgPool, id: Uuid) -> Result<(), ApiKeyError> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
