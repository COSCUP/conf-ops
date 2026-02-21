use sqlx::PgPool;
use uuid::Uuid;

use super::error::StorageError;
use super::models::{FileMetadata, FileRecord, FileStatus};

pub struct CreateFileParams<'a> {
    pub id: Uuid,
    pub filename: &'a str,
    pub mime_type: &'a str,
    pub file_size: i64,
    pub storage_path: &'a str,
    pub scope_type: &'a str,
    pub scope_id: Uuid,
    pub uploaded_by: Uuid,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
}

pub struct FileRepository;

impl FileRepository {
    /// Insert a new file record.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateFileParams<'_>,
    ) -> Result<FileRecord, StorageError> {
        sqlx::query_as!(
            FileRecord,
            r#"INSERT INTO files (id, filename, mime_type, file_size, storage_path, scope_type, scope_id, uploaded_by, organization_id, project_id, task_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id, filename, mime_type, file_size, storage_path, scope_type, scope_id,
                       status AS "status: FileStatus",
                       uploaded_by, organization_id, project_id, task_id,
                       created_at, updated_at, deleted_at"#,
            params.id,
            params.filename,
            params.mime_type,
            params.file_size,
            params.storage_path,
            params.scope_type,
            params.scope_id,
            params.uploaded_by,
            params.organization_id,
            params.project_id,
            params.task_id,
        )
        .fetch_one(pool)
        .await
        .map_err(StorageError::Database)
    }

    /// Fetch a file record by ID (non-deleted only).
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<FileRecord, StorageError> {
        sqlx::query_as!(
            FileRecord,
            r#"SELECT id, filename, mime_type, file_size, storage_path, scope_type, scope_id,
                      status AS "status: FileStatus",
                      uploaded_by, organization_id, project_id, task_id,
                      created_at, updated_at, deleted_at
             FROM files
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(StorageError::NotFound)
    }

    /// List files by scope (non-deleted only).
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn list_by_scope(
        pool: &PgPool,
        scope_type: &str,
        scope_id: Uuid,
    ) -> Result<Vec<FileRecord>, StorageError> {
        let files = sqlx::query_as!(
            FileRecord,
            r#"SELECT id, filename, mime_type, file_size, storage_path, scope_type, scope_id,
                      status AS "status: FileStatus",
                      uploaded_by, organization_id, project_id, task_id,
                      created_at, updated_at, deleted_at
             FROM files
             WHERE scope_type = $1 AND scope_id = $2 AND deleted_at IS NULL
             ORDER BY created_at ASC"#,
            scope_type,
            scope_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(files)
    }

    /// Soft-delete a file record.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), StorageError> {
        let result = sqlx::query!(
            "UPDATE files SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

        Ok(())
    }

    /// Find orphaned files (soft-deleted longer than grace period).
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn find_orphaned_files(
        pool: &PgPool,
        grace_period_secs: i64,
    ) -> Result<Vec<FileRecord>, StorageError> {
        let files = sqlx::query_as!(
            FileRecord,
            r#"SELECT id, filename, mime_type, file_size, storage_path, scope_type, scope_id,
                      status AS "status: FileStatus",
                      uploaded_by, organization_id, project_id, task_id,
                      created_at, updated_at, deleted_at
             FROM files
             WHERE deleted_at IS NOT NULL
               AND deleted_at < NOW() - ($1 || ' seconds')::INTERVAL
             ORDER BY deleted_at ASC"#,
            grace_period_secs.to_string(),
        )
        .fetch_all(pool)
        .await?;

        Ok(files)
    }

    /// Hard-delete a file record (physical removal from DB).
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn hard_delete(pool: &PgPool, id: Uuid) -> Result<(), StorageError> {
        sqlx::query!("DELETE FROM files WHERE id = $1", id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct FileMetadataRepository;

impl FileMetadataRepository {
    /// Insert or update a metadata entry for a file (upsert on `file_id` + key).
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn upsert(
        pool: &PgPool,
        id: Uuid,
        file_id: Uuid,
        key: &str,
        value: &str,
    ) -> Result<FileMetadata, StorageError> {
        sqlx::query_as!(
            FileMetadata,
            r#"INSERT INTO file_metadata (id, file_id, key, value)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (file_id, key) DO UPDATE SET value = EXCLUDED.value
             RETURNING id, file_id, key, value, created_at"#,
            id,
            file_id,
            key,
            value,
        )
        .fetch_one(pool)
        .await
        .map_err(StorageError::Database)
    }

    /// List all metadata entries for a file.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn list_by_file(
        pool: &PgPool,
        file_id: Uuid,
    ) -> Result<Vec<FileMetadata>, StorageError> {
        sqlx::query_as!(
            FileMetadata,
            r#"SELECT id, file_id, key, value, created_at
             FROM file_metadata
             WHERE file_id = $1
             ORDER BY created_at ASC"#,
            file_id,
        )
        .fetch_all(pool)
        .await
        .map_err(StorageError::Database)
    }

    /// Delete a specific metadata key for a file.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn delete_by_key(
        pool: &PgPool,
        file_id: Uuid,
        key: &str,
    ) -> Result<(), StorageError> {
        sqlx::query!(
            "DELETE FROM file_metadata WHERE file_id = $1 AND key = $2",
            file_id,
            key,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
