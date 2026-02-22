use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::ToolError;
use super::models::{CreateToolConfigParams, RecordToolExecutionParams, ToolConfig, ToolExecution};

// ── Row Mapping Helpers ─────────────────────────────────────

fn map_tool_config(row: &sqlx::postgres::PgRow) -> Result<ToolConfig, sqlx::Error> {
    Ok(ToolConfig {
        id: row.try_get("id")?,
        scope_type: row.try_get("scope_type")?,
        scope_id: row.try_get("scope_id")?,
        tool_type: row.try_get("tool_type")?,
        tool_name: row.try_get("tool_name")?,
        display_name: row.try_get("display_name")?,
        description: row.try_get("description")?,
        enabled: row.try_get("enabled")?,
        config: row.try_get("config")?,
        mcp_server_config: row.try_get("mcp_server_config")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        deleted_at: row.try_get("deleted_at")?,
    })
}

fn map_tool_execution(row: &sqlx::postgres::PgRow) -> Result<ToolExecution, sqlx::Error> {
    Ok(ToolExecution {
        id: row.try_get("id")?,
        task_id: row.try_get("task_id")?,
        suggestion_id: row.try_get("suggestion_id")?,
        tool_name: row.try_get("tool_name")?,
        parameters: row.try_get("parameters")?,
        result: row.try_get("result")?,
        status: row.try_get("status")?,
        executed_by: row.try_get("executed_by")?,
        executed_at: row.try_get("executed_at")?,
        duration_ms: row.try_get("duration_ms")?,
    })
}

// ── Tool Config Repository ──────────────────────────────────

pub struct ToolConfigRepository;

impl ToolConfigRepository {
    /// Create a tool configuration.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::DuplicateConfig` if a config with the same scope + `tool_name` exists.
    /// Returns `ToolError::Database` on other database failures.
    pub async fn create(
        pool: &PgPool,
        params: &CreateToolConfigParams,
    ) -> Result<ToolConfig, ToolError> {
        let row = sqlx::query(
            "INSERT INTO tool_configs
                 (id, scope_type, scope_id, tool_type, tool_name, display_name,
                  description, enabled, config, mcp_server_config)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING id, scope_type, scope_id, tool_type, tool_name, display_name,
                       description, enabled, config, mcp_server_config,
                       created_at, updated_at, deleted_at",
        )
        .bind(params.id)
        .bind(params.scope_type.as_str())
        .bind(params.scope_id)
        .bind(params.tool_type.as_str())
        .bind(&params.tool_name)
        .bind(params.display_name.as_deref())
        .bind(params.description.as_deref())
        .bind(params.enabled)
        .bind(&params.config)
        .bind(&params.mcp_server_config)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("uq_tool_configs_scope_tool_name") {
                    return ToolError::DuplicateConfig;
                }
            }
            ToolError::Database(e)
        })?;

        map_tool_config(&row).map_err(ToolError::Database)
    }

    /// Get a tool config by ID (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ConfigNotFound` if not found.
    /// Returns `ToolError::Database` on database failure.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<ToolConfig, ToolError> {
        let row = sqlx::query(
            "SELECT id, scope_type, scope_id, tool_type, tool_name, display_name,
                      description, enabled, config, mcp_server_config,
                      created_at, updated_at, deleted_at
             FROM tool_configs
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ToolError::ConfigNotFound)?;

        map_tool_config(&row).map_err(ToolError::Database)
    }

    /// List tool configs by scope (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn list_by_scope(
        pool: &PgPool,
        scope_type: &str,
        scope_id: Uuid,
    ) -> Result<Vec<ToolConfig>, ToolError> {
        let rows = sqlx::query(
            "SELECT id, scope_type, scope_id, tool_type, tool_name, display_name,
                      description, enabled, config, mcp_server_config,
                      created_at, updated_at, deleted_at
             FROM tool_configs
             WHERE scope_type = $1 AND scope_id = $2 AND deleted_at IS NULL
             ORDER BY tool_name",
        )
        .bind(scope_type)
        .bind(scope_id)
        .fetch_all(pool)
        .await
        .map_err(ToolError::Database)?;

        rows.iter()
            .map(|r| map_tool_config(r).map_err(ToolError::Database))
            .collect()
    }

    /// Get effective tool configs for a project (with organization inheritance).
    ///
    /// Returns configs where project-level settings override org-level settings
    /// for the same `tool_name`, using `DISTINCT ON (tool_name)`.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn get_effective_configs(
        pool: &PgPool,
        project_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<ToolConfig>, ToolError> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON (tool_name)
                      id, scope_type, scope_id, tool_type, tool_name, display_name,
                      description, enabled, config, mcp_server_config,
                      created_at, updated_at, deleted_at
             FROM tool_configs
             WHERE deleted_at IS NULL
               AND (
                   (scope_type = 'project' AND scope_id = $1)
                   OR
                   (scope_type = 'organization' AND scope_id = $2)
               )
             ORDER BY tool_name,
                      CASE scope_type WHEN 'project' THEN 0 ELSE 1 END",
        )
        .bind(project_id)
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(ToolError::Database)?;

        rows.iter()
            .map(|r| map_tool_config(r).map_err(ToolError::Database))
            .collect()
    }

    /// Get a specific effective tool config by name.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ConfigNotFound` if no matching config.
    /// Returns `ToolError::Database` on database failure.
    pub async fn get_effective_config_by_name(
        pool: &PgPool,
        project_id: Uuid,
        organization_id: Uuid,
        tool_name: &str,
    ) -> Result<ToolConfig, ToolError> {
        let row = sqlx::query(
            "SELECT DISTINCT ON (tool_name)
                      id, scope_type, scope_id, tool_type, tool_name, display_name,
                      description, enabled, config, mcp_server_config,
                      created_at, updated_at, deleted_at
             FROM tool_configs
             WHERE deleted_at IS NULL
               AND tool_name = $3
               AND (
                   (scope_type = 'project' AND scope_id = $1)
                   OR
                   (scope_type = 'organization' AND scope_id = $2)
               )
             ORDER BY tool_name,
                      CASE scope_type WHEN 'project' THEN 0 ELSE 1 END",
        )
        .bind(project_id)
        .bind(organization_id)
        .bind(tool_name)
        .fetch_optional(pool)
        .await?
        .ok_or(ToolError::ConfigNotFound)?;

        map_tool_config(&row).map_err(ToolError::Database)
    }

    /// Update a tool configuration.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ConfigNotFound` if not found.
    /// Returns `ToolError::Database` on database failure.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
        enabled: Option<bool>,
        config: Option<&serde_json::Value>,
        mcp_server_config: Option<&serde_json::Value>,
    ) -> Result<ToolConfig, ToolError> {
        let row = sqlx::query(
            "UPDATE tool_configs
             SET display_name = COALESCE($2, display_name),
                 description = COALESCE($3, description),
                 enabled = COALESCE($4, enabled),
                 config = COALESCE($5, config),
                 mcp_server_config = COALESCE($6, mcp_server_config),
                 updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, scope_type, scope_id, tool_type, tool_name, display_name,
                       description, enabled, config, mcp_server_config,
                       created_at, updated_at, deleted_at",
        )
        .bind(id)
        .bind(display_name)
        .bind(description)
        .bind(enabled)
        .bind(config)
        .bind(mcp_server_config)
        .fetch_optional(pool)
        .await?
        .ok_or(ToolError::ConfigNotFound)?;

        map_tool_config(&row).map_err(ToolError::Database)
    }

    /// Soft delete a tool configuration.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ConfigNotFound` if not found.
    /// Returns `ToolError::Database` on database failure.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), ToolError> {
        let result = sqlx::query(
            "UPDATE tool_configs SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ToolError::ConfigNotFound);
        }
        Ok(())
    }
}

// ── Tool Execution Repository ───────────────────────────────

pub struct ToolExecutionRepository;

impl ToolExecutionRepository {
    /// Record a tool execution.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn record(
        pool: &PgPool,
        params: &RecordToolExecutionParams,
    ) -> Result<ToolExecution, ToolError> {
        let row = sqlx::query(
            "INSERT INTO tool_executions
                 (id, task_id, suggestion_id, tool_name, parameters, result,
                  status, executed_by, duration_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7::tool_execution_status, $8, $9)
             RETURNING id, task_id, suggestion_id, tool_name, parameters, result,
                       status::TEXT, executed_by, executed_at, duration_ms",
        )
        .bind(params.id)
        .bind(params.task_id)
        .bind(params.suggestion_id)
        .bind(&params.tool_name)
        .bind(&params.parameters)
        .bind(&params.result)
        .bind(params.status.as_str())
        .bind(params.executed_by)
        .bind(params.duration_ms)
        .fetch_one(pool)
        .await
        .map_err(ToolError::Database)?;

        map_tool_execution(&row).map_err(ToolError::Database)
    }

    /// List tool executions for a task.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn list_by_task(
        pool: &PgPool,
        task_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<ToolExecution>, ToolError> {
        let rows = sqlx::query(
            "SELECT id, task_id, suggestion_id, tool_name, parameters, result,
                      status::TEXT, executed_by, executed_at, duration_ms
             FROM tool_executions
             WHERE task_id = $1
               AND ($2::UUID IS NULL OR id < $2)
             ORDER BY id DESC
             LIMIT $3",
        )
        .bind(task_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ToolError::Database)?;

        rows.iter()
            .map(|r| map_tool_execution(r).map_err(ToolError::Database))
            .collect()
    }
}
