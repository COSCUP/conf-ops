use sqlx::PgPool;
use uuid::Uuid;

use super::error::TaskError;
use super::models::{Task, TaskStatus};

pub struct CreateTaskParams<'a> {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_template_id: Uuid,
    pub owner_tag_id: Uuid,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub created_by: Uuid,
}

pub struct TaskRepository;

impl TaskRepository {
    /// Insert a new task into the database.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn create(pool: &PgPool, params: &CreateTaskParams<'_>) -> Result<Task, TaskError> {
        sqlx::query_as!(
            Task,
            r#"INSERT INTO tasks (id, project_id, task_template_id, owner_tag_id, name, description, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, project_id, task_template_id, owner_tag_id, name, description,
                       status AS "status: TaskStatus",
                       created_by, created_at, updated_at, deleted_at"#,
            params.id,
            params.project_id,
            params.task_template_id,
            params.owner_tag_id,
            params.name,
            params.description,
            params.created_by,
        )
        .fetch_one(pool)
        .await
        .map_err(TaskError::Database)
    }

    /// Fetch a task by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Task, TaskError> {
        sqlx::query_as!(
            Task,
            r#"SELECT id, project_id, task_template_id, owner_tag_id, name, description,
                      status AS "status: TaskStatus",
                      created_by, created_at, updated_at, deleted_at
             FROM tasks
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskError::NotFound)
    }

    /// List tasks for a project with optional status and tag filters.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
        status_filter: Option<&TaskStatus>,
        tag_filter: Option<Uuid>,
    ) -> Result<Vec<Task>, TaskError> {
        let tasks = sqlx::query_as!(
            Task,
            r#"SELECT id, project_id, task_template_id, owner_tag_id, name, description,
                      status AS "status: TaskStatus",
                      created_by, created_at, updated_at, deleted_at
             FROM tasks
             WHERE project_id = $1
               AND deleted_at IS NULL
               AND ($2::task_status IS NULL OR status = $2)
               AND ($3::UUID IS NULL OR owner_tag_id = $3)
             ORDER BY created_at DESC"#,
            project_id,
            status_filter as Option<&TaskStatus>,
            tag_filter,
        )
        .fetch_all(pool)
        .await?;

        Ok(tasks)
    }

    /// Update a task's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<Task, TaskError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };

        sqlx::query_as!(
            Task,
            r#"UPDATE tasks
             SET name = $2, description = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, task_template_id, owner_tag_id, name, description,
                       status AS "status: TaskStatus",
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskError::NotFound)
    }

    /// Update a task's status.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: &TaskStatus,
    ) -> Result<Task, TaskError> {
        sqlx::query_as!(
            Task,
            r#"UPDATE tasks
             SET status = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, task_template_id, owner_tag_id, name, description,
                       status AS "status: TaskStatus",
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            status as &TaskStatus,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TaskError::NotFound)
    }

    /// Soft-delete a task by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), TaskError> {
        let result = sqlx::query!(
            "UPDATE tasks SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskError::NotFound);
        }

        Ok(())
    }

    /// Get the creator account ID of a task.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn get_creator_account_id(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Option<Uuid>, TaskError> {
        let id = sqlx::query_scalar!(
            r#"SELECT created_by FROM tasks WHERE id = $1 AND deleted_at IS NULL"#,
            task_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(id)
    }

    /// Fetch multiple tasks by their IDs.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn get_by_ids(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<Task>, TaskError> {
        let tasks = sqlx::query_as!(
            Task,
            r#"SELECT id, project_id, task_template_id, owner_tag_id, name, description,
                      status AS "status: TaskStatus",
                      created_by, created_at, updated_at, deleted_at
             FROM tasks
             WHERE id = ANY($1) AND deleted_at IS NULL"#,
            ids,
        )
        .fetch_all(pool)
        .await?;

        Ok(tasks)
    }

    /// Check if an owner tag is linked to a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn is_tag_linked_to_template(
        pool: &PgPool,
        task_template_id: Uuid,
        member_tag_id: Uuid,
    ) -> Result<bool, TaskError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                SELECT 1 FROM task_template_tags
                WHERE task_template_id = $1 AND member_tag_id = $2
             ) AS "exists!""#,
            task_template_id,
            member_tag_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }
}
