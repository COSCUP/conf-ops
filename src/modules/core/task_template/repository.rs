use sqlx::PgPool;
use uuid::Uuid;

use super::error::TaskTemplateError;
use super::models::{DataSchema, TaskTemplate, TaskTemplateTag, TodoTemplate};

pub struct TaskTemplateRepository;

impl TaskTemplateRepository {
    /// Insert a new task template into the database.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        created_by: Uuid,
    ) -> Result<TaskTemplate, TaskTemplateError> {
        sqlx::query_as!(
            TaskTemplate,
            r#"INSERT INTO task_templates (id, project_id, name, description, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, project_id, name, description, created_by,
                       created_at, updated_at, deleted_at"#,
            id,
            project_id,
            name,
            description,
            created_by,
        )
        .fetch_one(pool)
        .await
        .map_err(TaskTemplateError::Database)
    }

    /// Fetch a task template by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<TaskTemplate, TaskTemplateError> {
        sqlx::query_as!(
            TaskTemplate,
            r#"SELECT id, project_id, name, description, created_by,
                      created_at, updated_at, deleted_at
             FROM task_templates
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::NotFound)
    }

    /// List all task templates for a project.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<TaskTemplate>, TaskTemplateError> {
        let templates = sqlx::query_as!(
            TaskTemplate,
            r#"SELECT id, project_id, name, description, created_by,
                      created_at, updated_at, deleted_at
             FROM task_templates
             WHERE project_id = $1 AND deleted_at IS NULL
             ORDER BY created_at ASC"#,
            project_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(templates)
    }

    /// Update a task template's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<TaskTemplate, TaskTemplateError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };

        sqlx::query_as!(
            TaskTemplate,
            r#"UPDATE task_templates
             SET name = $2, description = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, name, description, created_by,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::NotFound)
    }

    /// Soft-delete a task template by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), TaskTemplateError> {
        let result = sqlx::query!(
            "UPDATE task_templates SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskTemplateError::NotFound);
        }

        Ok(())
    }

    // ── Tag links ───────────────────────────────────────────────

    /// Link a member tag to a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TagAlreadyLinked` if already linked.
    /// Returns `TaskTemplateError::Database` on other database failure.
    pub async fn link_tag(
        pool: &PgPool,
        id: Uuid,
        task_template_id: Uuid,
        member_tag_id: Uuid,
    ) -> Result<TaskTemplateTag, TaskTemplateError> {
        sqlx::query_as!(
            TaskTemplateTag,
            r#"INSERT INTO task_template_tags (id, task_template_id, member_tag_id)
             VALUES ($1, $2, $3)
             RETURNING id, task_template_id, member_tag_id, created_at"#,
            id,
            task_template_id,
            member_tag_id,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err)
                if db_err.constraint() == Some("uq_task_template_tags_template_tag") =>
            {
                TaskTemplateError::TagAlreadyLinked
            }
            _ => TaskTemplateError::Database(e),
        })
    }

    /// Unlink a member tag from a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TagLinkNotFound` if the link does not exist.
    pub async fn unlink_tag(
        pool: &PgPool,
        task_template_id: Uuid,
        member_tag_id: Uuid,
    ) -> Result<(), TaskTemplateError> {
        let result = sqlx::query!(
            "DELETE FROM task_template_tags
             WHERE task_template_id = $1 AND member_tag_id = $2",
            task_template_id,
            member_tag_id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskTemplateError::TagLinkNotFound);
        }

        Ok(())
    }

    /// List all tags linked to a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_tags(
        pool: &PgPool,
        task_template_id: Uuid,
    ) -> Result<Vec<TaskTemplateTag>, TaskTemplateError> {
        let tags = sqlx::query_as!(
            TaskTemplateTag,
            r#"SELECT id, task_template_id, member_tag_id, created_at
             FROM task_template_tags
             WHERE task_template_id = $1
             ORDER BY created_at ASC"#,
            task_template_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    // ── Todo templates ──────────────────────────────────────────

    /// Insert a new todo template into the database.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create_todo_template(
        pool: &PgPool,
        id: Uuid,
        task_template_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<TodoTemplate, TaskTemplateError> {
        sqlx::query_as!(
            TodoTemplate,
            r#"INSERT INTO todo_templates (id, task_template_id, parent_id, name, description, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, task_template_id, parent_id, name, description, sort_order,
                       created_at, updated_at, deleted_at"#,
            id,
            task_template_id,
            parent_id,
            name,
            description,
            sort_order,
        )
        .fetch_one(pool)
        .await
        .map_err(TaskTemplateError::Database)
    }

    /// Fetch a todo template by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn get_todo_template_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<TodoTemplate, TaskTemplateError> {
        sqlx::query_as!(
            TodoTemplate,
            r#"SELECT id, task_template_id, parent_id, name, description, sort_order,
                      created_at, updated_at, deleted_at
             FROM todo_templates
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::TodoTemplateNotFound)
    }

    /// List all todo templates for a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_todo_templates(
        pool: &PgPool,
        task_template_id: Uuid,
    ) -> Result<Vec<TodoTemplate>, TaskTemplateError> {
        let templates = sqlx::query_as!(
            TodoTemplate,
            r#"SELECT id, task_template_id, parent_id, name, description, sort_order,
                      created_at, updated_at, deleted_at
             FROM todo_templates
             WHERE task_template_id = $1 AND deleted_at IS NULL
             ORDER BY sort_order ASC"#,
            task_template_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(templates)
    }

    /// Update a todo template's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn update_todo_template(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<TodoTemplate, TaskTemplateError> {
        let current = Self::get_todo_template_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };

        sqlx::query_as!(
            TodoTemplate,
            r#"UPDATE todo_templates
             SET name = $2, description = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, task_template_id, parent_id, name, description, sort_order,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::TodoTemplateNotFound)
    }

    /// Soft-delete a todo template by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn delete_todo_template(pool: &PgPool, id: Uuid) -> Result<(), TaskTemplateError> {
        let result = sqlx::query!(
            "UPDATE todo_templates SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskTemplateError::TodoTemplateNotFound);
        }

        Ok(())
    }

    /// Reorder todo templates by updating their sort orders.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn reorder_todo_templates(
        pool: &PgPool,
        orders: &[(Uuid, i32)],
    ) -> Result<(), TaskTemplateError> {
        for (id, sort_order) in orders {
            sqlx::query!(
                "UPDATE todo_templates SET sort_order = $2, updated_at = NOW()
                 WHERE id = $1 AND deleted_at IS NULL",
                id,
                sort_order,
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    // ── Data schemas ────────────────────────────────────────────

    /// Insert a new data schema into the database.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create_data_schema(
        pool: &PgPool,
        id: Uuid,
        task_template_id: Uuid,
        name: &str,
        fields: &serde_json::Value,
    ) -> Result<DataSchema, TaskTemplateError> {
        sqlx::query_as!(
            DataSchema,
            r#"INSERT INTO data_schemas (id, task_template_id, name, fields)
             VALUES ($1, $2, $3, $4)
             RETURNING id, task_template_id, name, fields,
                       created_at, updated_at, deleted_at"#,
            id,
            task_template_id,
            name,
            fields,
        )
        .fetch_one(pool)
        .await
        .map_err(TaskTemplateError::Database)
    }

    /// Fetch a data schema by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn get_data_schema_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<DataSchema, TaskTemplateError> {
        sqlx::query_as!(
            DataSchema,
            r#"SELECT id, task_template_id, name, fields,
                      created_at, updated_at, deleted_at
             FROM data_schemas
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::DataSchemaNotFound)
    }

    /// List all data schemas for a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_data_schemas(
        pool: &PgPool,
        task_template_id: Uuid,
    ) -> Result<Vec<DataSchema>, TaskTemplateError> {
        let schemas = sqlx::query_as!(
            DataSchema,
            r#"SELECT id, task_template_id, name, fields,
                      created_at, updated_at, deleted_at
             FROM data_schemas
             WHERE task_template_id = $1 AND deleted_at IS NULL
             ORDER BY created_at ASC"#,
            task_template_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(schemas)
    }

    /// Update a data schema's name and/or fields.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn update_data_schema(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        fields: Option<&serde_json::Value>,
    ) -> Result<DataSchema, TaskTemplateError> {
        let current = Self::get_data_schema_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let fields = fields.unwrap_or(&current.fields);

        sqlx::query_as!(
            DataSchema,
            r#"UPDATE data_schemas
             SET name = $2, fields = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, task_template_id, name, fields,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            fields,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskTemplateError::DataSchemaNotFound)
    }

    /// Soft-delete a data schema by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn delete_data_schema(pool: &PgPool, id: Uuid) -> Result<(), TaskTemplateError> {
        let result = sqlx::query!(
            "UPDATE data_schemas SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskTemplateError::DataSchemaNotFound);
        }

        Ok(())
    }
}
