use sqlx::PgPool;
use uuid::Uuid;

use super::error::MemoryError;
use super::models::{
    CreateLibraryDocumentParams, CreateMemoryParams, InheritedMemories, LibraryDocument,
    LibraryDocumentVersion, Memory, MemoryVersion, UpdateLibraryDocumentParams, UpdateMemoryParams,
};

// ── Memory Repository ───────────────────────────────────────

pub struct MemoryRepository;

impl MemoryRepository {
    /// Create a memory and its initial version.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn create(pool: &PgPool, params: &CreateMemoryParams) -> Result<Memory, MemoryError> {
        let memory = sqlx::query_as!(
            Memory,
            r#"INSERT INTO memories (id, scope_type, scope_id, content, library_ref, source, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, scope_type, scope_id, content, library_ref, source,
                       created_by, created_at, updated_at, deleted_at"#,
            params.id,
            params.scope_type.as_str(),
            params.scope_id,
            params.content,
            params.library_ref,
            params.source.as_str(),
            params.created_by,
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO memory_versions (id, memory_id, content, changed_by)
             VALUES ($1, $2, $3, $4)"#,
            Uuid::now_v7(),
            params.id,
            params.content,
            params.created_by,
        )
        .execute(pool)
        .await?;

        Ok(memory)
    }

    /// Get a memory by ID (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Memory, MemoryError> {
        sqlx::query_as!(
            Memory,
            r#"SELECT id, scope_type, scope_id, content, library_ref, source,
                      created_by, created_at, updated_at, deleted_at
             FROM memories
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemoryError::NotFound)
    }

    /// List memories by scope with cursor-based pagination.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn list_by_scope(
        pool: &PgPool,
        scope_type: &str,
        scope_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<Memory>, MemoryError> {
        sqlx::query_as!(
            Memory,
            r#"SELECT id, scope_type, scope_id, content, library_ref, source,
                      created_by, created_at, updated_at, deleted_at
             FROM memories
             WHERE scope_type = $1 AND scope_id = $2
               AND deleted_at IS NULL
               AND ($3::UUID IS NULL OR id < $3)
             ORDER BY id DESC
             LIMIT $4"#,
            scope_type,
            scope_id,
            cursor,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(MemoryError::Database)
    }

    /// Update a memory and insert a version record.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        params: &UpdateMemoryParams,
        changed_by: Uuid,
    ) -> Result<Memory, MemoryError> {
        let memory = sqlx::query_as!(
            Memory,
            r#"UPDATE memories
             SET content = $2, library_ref = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, scope_type, scope_id, content, library_ref, source,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            params.content,
            params.library_ref,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemoryError::NotFound)?;

        sqlx::query!(
            r#"INSERT INTO memory_versions (id, memory_id, content, changed_by)
             VALUES ($1, $2, $3, $4)"#,
            Uuid::now_v7(),
            id,
            params.content,
            changed_by,
        )
        .execute(pool)
        .await?;

        Ok(memory)
    }

    /// Soft-delete a memory.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), MemoryError> {
        let result = sqlx::query!(
            r#"UPDATE memories SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::NotFound);
        }
        Ok(())
    }

    /// List version history for a memory.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn list_versions(
        pool: &PgPool,
        memory_id: Uuid,
    ) -> Result<Vec<MemoryVersion>, MemoryError> {
        sqlx::query_as!(
            MemoryVersion,
            r#"SELECT id, memory_id, content, changed_by, created_at
             FROM memory_versions
             WHERE memory_id = $1
             ORDER BY created_at DESC"#,
            memory_id,
        )
        .fetch_all(pool)
        .await
        .map_err(MemoryError::Database)
    }

    /// Collect all inherited memories for a given task.
    ///
    /// Resolves the inheritance chain:
    /// task -> `task_template` -> `member_tag` -> project -> organization -> account.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn collect_inherited(
        pool: &PgPool,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<InheritedMemories, MemoryError> {
        let memories = sqlx::query_as!(
            Memory,
            r#"WITH task_info AS (
                SELECT t.id AS task_id,
                       t.task_template_id,
                       t.owner_tag_id,
                       t.project_id,
                       p.organization_id
                FROM tasks t
                JOIN projects p ON p.id = t.project_id
                WHERE t.id = $1 AND t.deleted_at IS NULL
            ),
            scope_chain AS (
                SELECT 'task' AS scope_type, task_id AS scope_id, 0 AS sort_order FROM task_info
                UNION ALL
                SELECT 'task_template', task_template_id, 1 FROM task_info
                UNION ALL
                SELECT 'member_tag', owner_tag_id, 2 FROM task_info
                UNION ALL
                SELECT 'project', project_id, 3 FROM task_info
                UNION ALL
                SELECT 'organization', organization_id, 4 FROM task_info
                UNION ALL
                SELECT 'account', $2::UUID, 5
            )
            SELECT m.id, m.scope_type, m.scope_id, m.content, m.library_ref, m.source,
                   m.created_by, m.created_at, m.updated_at, m.deleted_at
            FROM scope_chain sc
            JOIN memories m ON m.scope_type = sc.scope_type AND m.scope_id = sc.scope_id
            WHERE m.deleted_at IS NULL
            ORDER BY sc.sort_order, m.created_at DESC"#,
            task_id,
            account_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(InheritedMemories { task_id, memories })
    }
}

// ── Library Document Repository ─────────────────────────────

pub struct LibraryDocumentRepository;

impl LibraryDocumentRepository {
    /// Create a library document and its initial version.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateLibraryDocumentParams,
    ) -> Result<LibraryDocument, MemoryError> {
        let doc = sqlx::query_as!(
            LibraryDocument,
            r#"INSERT INTO library_documents (id, scope_type, scope_id, title, content, created_by)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, scope_type, scope_id, title, content,
                       created_by, created_at, updated_at, deleted_at"#,
            params.id,
            params.scope_type.as_str(),
            params.scope_id,
            params.title,
            params.content,
            params.created_by,
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO library_document_versions (id, document_id, title, content, changed_by)
             VALUES ($1, $2, $3, $4, $5)"#,
            Uuid::now_v7(),
            params.id,
            params.title,
            params.content,
            params.created_by,
        )
        .execute(pool)
        .await?;

        Ok(doc)
    }

    /// Get a library document by ID (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<LibraryDocument, MemoryError> {
        sqlx::query_as!(
            LibraryDocument,
            r#"SELECT id, scope_type, scope_id, title, content,
                      created_by, created_at, updated_at, deleted_at
             FROM library_documents
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemoryError::LibraryDocumentNotFound)
    }

    /// List library documents by scope with cursor-based pagination.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn list_by_scope(
        pool: &PgPool,
        scope_type: &str,
        scope_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<LibraryDocument>, MemoryError> {
        sqlx::query_as!(
            LibraryDocument,
            r#"SELECT id, scope_type, scope_id, title, content,
                      created_by, created_at, updated_at, deleted_at
             FROM library_documents
             WHERE scope_type = $1 AND scope_id = $2
               AND deleted_at IS NULL
               AND ($3::UUID IS NULL OR id < $3)
             ORDER BY id DESC
             LIMIT $4"#,
            scope_type,
            scope_id,
            cursor,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(MemoryError::Database)
    }

    /// Update a library document and insert a version record.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        params: &UpdateLibraryDocumentParams,
        changed_by: Uuid,
    ) -> Result<LibraryDocument, MemoryError> {
        let doc = sqlx::query_as!(
            LibraryDocument,
            r#"UPDATE library_documents
             SET title = COALESCE($2, title), content = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, scope_type, scope_id, title, content,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            params.title,
            params.content,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemoryError::LibraryDocumentNotFound)?;

        sqlx::query!(
            r#"INSERT INTO library_document_versions (id, document_id, title, content, changed_by)
             VALUES ($1, $2, $3, $4, $5)"#,
            Uuid::now_v7(),
            id,
            params.title,
            params.content,
            changed_by,
        )
        .execute(pool)
        .await?;

        Ok(doc)
    }

    /// Soft-delete a library document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), MemoryError> {
        let result = sqlx::query!(
            r#"UPDATE library_documents SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::LibraryDocumentNotFound);
        }
        Ok(())
    }

    /// List version history for a library document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn list_versions(
        pool: &PgPool,
        document_id: Uuid,
    ) -> Result<Vec<LibraryDocumentVersion>, MemoryError> {
        sqlx::query_as!(
            LibraryDocumentVersion,
            r#"SELECT id, document_id, title, content, changed_by, created_at
             FROM library_document_versions
             WHERE document_id = $1
             ORDER BY created_at DESC"#,
            document_id,
        )
        .fetch_all(pool)
        .await
        .map_err(MemoryError::Database)
    }
}
