use sqlx::PgPool;
use uuid::Uuid;

use super::error::TodoError;
use super::models::{Todo, TodoAssignee, TodoStatus, TodoType};

pub struct CreateTodoParams<'a> {
    pub id: Uuid,
    pub task_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub todo_type: &'a TodoType,
    pub source_template_id: Option<Uuid>,
    pub sort_order: i32,
}

pub struct TodoRepository;

impl TodoRepository {
    /// Insert a new todo into the database.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn create(pool: &PgPool, params: &CreateTodoParams<'_>) -> Result<Todo, TodoError> {
        sqlx::query_as!(
            Todo,
            r#"INSERT INTO todos (id, task_id, parent_id, title, description, type, source_template_id, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, task_id, parent_id, title, description,
                       status AS "status: TodoStatus",
                       type AS "todo_type: TodoType",
                       source_template_id, due_date, sort_order,
                       linked_task_id, completed_at, completed_by,
                       created_at, updated_at, deleted_at"#,
            params.id,
            params.task_id,
            params.parent_id,
            params.title,
            params.description,
            params.todo_type as &TodoType,
            params.source_template_id,
            params.sort_order,
        )
        .fetch_one(pool)
        .await
        .map_err(TodoError::Database)
    }

    /// Fetch a todo by ID.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Todo, TodoError> {
        sqlx::query_as!(
            Todo,
            r#"SELECT id, task_id, parent_id, title, description,
                      status AS "status: TodoStatus",
                      type AS "todo_type: TodoType",
                      source_template_id, due_date, sort_order,
                      linked_task_id, completed_at, completed_by,
                      created_at, updated_at, deleted_at
             FROM todos
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TodoError::NotFound)
    }

    /// List all todos for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn list_by_task(pool: &PgPool, task_id: Uuid) -> Result<Vec<Todo>, TodoError> {
        let todos = sqlx::query_as!(
            Todo,
            r#"SELECT id, task_id, parent_id, title, description,
                      status AS "status: TodoStatus",
                      type AS "todo_type: TodoType",
                      source_template_id, due_date, sort_order,
                      linked_task_id, completed_at, completed_by,
                      created_at, updated_at, deleted_at
             FROM todos
             WHERE task_id = $1 AND deleted_at IS NULL
             ORDER BY sort_order ASC"#,
            task_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(todos)
    }

    /// Update a todo's title, description, and/or due date.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        due_date: Option<Option<chrono::DateTime<chrono::Utc>>>,
    ) -> Result<Todo, TodoError> {
        let current = Self::get_by_id(pool, id).await?;

        let title = title.unwrap_or(&current.title);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };
        let due_date = match due_date {
            Some(d) => d,
            None => current.due_date,
        };

        sqlx::query_as!(
            Todo,
            r#"UPDATE todos
             SET title = $2, description = $3, due_date = $4, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, task_id, parent_id, title, description,
                       status AS "status: TodoStatus",
                       type AS "todo_type: TodoType",
                       source_template_id, due_date, sort_order,
                       linked_task_id, completed_at, completed_by,
                       created_at, updated_at, deleted_at"#,
            id,
            title,
            description,
            due_date,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TodoError::NotFound)
    }

    /// Update a todo's status (complete or reopen).
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: &TodoStatus,
        completed_by: Option<Uuid>,
    ) -> Result<Todo, TodoError> {
        let (completed_at, completed_by_val) = match status {
            TodoStatus::Completed => (Some(chrono::Utc::now()), completed_by),
            TodoStatus::Open => (None, None),
        };

        sqlx::query_as!(
            Todo,
            r#"UPDATE todos
             SET status = $2, completed_at = $3, completed_by = $4, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, task_id, parent_id, title, description,
                       status AS "status: TodoStatus",
                       type AS "todo_type: TodoType",
                       source_template_id, due_date, sort_order,
                       linked_task_id, completed_at, completed_by,
                       created_at, updated_at, deleted_at"#,
            id,
            status as &TodoStatus,
            completed_at,
            completed_by_val,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(TodoError::NotFound)
    }

    /// Soft-delete a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), TodoError> {
        let result = sqlx::query!(
            "UPDATE todos SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TodoError::NotFound);
        }

        Ok(())
    }

    /// Count incomplete (open) todos for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn count_incomplete_by_task(pool: &PgPool, task_id: Uuid) -> Result<i64, TodoError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM todos
             WHERE task_id = $1 AND status = 'open' AND deleted_at IS NULL"#,
            task_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// Get the next sort order value for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn next_sort_order(pool: &PgPool, task_id: Uuid) -> Result<i32, TodoError> {
        let max = sqlx::query_scalar!(
            r#"SELECT COALESCE(MAX(sort_order), -1) AS "max!" FROM todos
             WHERE task_id = $1 AND deleted_at IS NULL"#,
            task_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(max + 1)
    }

    // ── Assignees ─────────────────────────────────────────────

    /// Add an assignee to a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::AssigneeAlreadyExists` if already assigned.
    /// Returns `TodoError::Database` on other database failure.
    pub async fn add_assignee(
        pool: &PgPool,
        id: Uuid,
        todo_id: Uuid,
        member_id: Uuid,
    ) -> Result<TodoAssignee, TodoError> {
        sqlx::query_as!(
            TodoAssignee,
            r#"INSERT INTO todo_assignees (id, todo_id, member_id)
             VALUES ($1, $2, $3)
             RETURNING id, todo_id, member_id, created_at"#,
            id,
            todo_id,
            member_id,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err)
                if db_err.constraint() == Some("uq_todo_assignees_todo_member") =>
            {
                TodoError::AssigneeAlreadyExists
            }
            _ => TodoError::Database(e),
        })
    }

    /// Remove an assignee from a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::AssigneeNotFound` if the assignment does not exist.
    pub async fn remove_assignee(
        pool: &PgPool,
        todo_id: Uuid,
        member_id: Uuid,
    ) -> Result<(), TodoError> {
        let result = sqlx::query!(
            "DELETE FROM todo_assignees WHERE todo_id = $1 AND member_id = $2",
            todo_id,
            member_id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TodoError::AssigneeNotFound);
        }

        Ok(())
    }

    /// List assignees for a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn list_assignees(
        pool: &PgPool,
        todo_id: Uuid,
    ) -> Result<Vec<TodoAssignee>, TodoError> {
        let assignees = sqlx::query_as!(
            TodoAssignee,
            r#"SELECT id, todo_id, member_id, created_at
             FROM todo_assignees
             WHERE todo_id = $1
             ORDER BY created_at ASC"#,
            todo_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(assignees)
    }
}
