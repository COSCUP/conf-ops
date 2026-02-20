use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::TodoError;

/// Entry for creating todos from templates.
pub type TemplateTodoEntry = (Uuid, Option<Uuid>, String, Option<String>, i32);
use super::models::{MyTodoItem, Todo, TodoAssignee, TodoStatus, TodoType};
use super::repository::{CreateTodoParams, TodoRepository};
use crate::modules::core::task::models::TaskStatus;
use crate::modules::core::task::repository::TaskRepository;

pub struct TodoService {
    pool: PgPool,
    event_bus: EventBus,
}

impl TodoService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Create a new ad-hoc todo for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NestingLimitExceeded` if the parent is not top-level.
    /// Returns `TodoError::Database` on database failure.
    pub async fn create_todo(
        &self,
        task_id: Uuid,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
    ) -> Result<Todo, TodoError> {
        // Validate nesting
        if let Some(pid) = parent_id {
            let parent = TodoRepository::get_by_id(&self.pool, pid).await?;
            if parent.parent_id.is_some() {
                return Err(TodoError::NestingLimitExceeded);
            }
        }

        let sort_order = TodoRepository::next_sort_order(&self.pool, task_id).await?;

        let id = generate_id();
        TodoRepository::create(
            &self.pool,
            &CreateTodoParams {
                id,
                task_id,
                parent_id,
                title,
                description,
                todo_type: &TodoType::AdHoc,
                source_template_id: None,
                sort_order,
            },
        )
        .await
    }

    /// Create todos from task templates when a task is instantiated.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn create_from_templates(
        &self,
        task_id: Uuid,
        template_todos: &[TemplateTodoEntry],
    ) -> Result<Vec<Todo>, TodoError> {
        let mut created = Vec::new();
        // Map from template todo ID → actual todo ID (for parent references)
        let mut id_map = std::collections::HashMap::new();

        // First pass: create top-level todos
        for (template_id, parent_template_id, title, description, sort_order) in template_todos {
            if parent_template_id.is_some() {
                continue;
            }
            let id = generate_id();
            id_map.insert(*template_id, id);
            let todo = TodoRepository::create(
                &self.pool,
                &CreateTodoParams {
                    id,
                    task_id,
                    parent_id: None,
                    title,
                    description: description.as_deref(),
                    todo_type: &TodoType::Template,
                    source_template_id: Some(*template_id),
                    sort_order: *sort_order,
                },
            )
            .await?;
            created.push(todo);
        }

        // Second pass: create child todos
        for (template_id, parent_template_id, title, description, sort_order) in template_todos {
            let Some(parent_tid) = parent_template_id else {
                continue;
            };
            let actual_parent = id_map.get(parent_tid).copied();
            let id = generate_id();
            let todo = TodoRepository::create(
                &self.pool,
                &CreateTodoParams {
                    id,
                    task_id,
                    parent_id: actual_parent,
                    title,
                    description: description.as_deref(),
                    todo_type: &TodoType::Template,
                    source_template_id: Some(*template_id),
                    sort_order: *sort_order,
                },
            )
            .await?;
            created.push(todo);
        }

        Ok(created)
    }

    /// Get a todo by ID.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn get_todo(&self, id: Uuid) -> Result<Todo, TodoError> {
        TodoRepository::get_by_id(&self.pool, id).await
    }

    /// List all todos for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn list_todos(&self, task_id: Uuid) -> Result<Vec<Todo>, TodoError> {
        TodoRepository::list_by_task(&self.pool, task_id).await
    }

    /// Update a todo's title, description, and/or due date.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn update_todo(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        due_date: Option<Option<chrono::DateTime<chrono::Utc>>>,
    ) -> Result<Todo, TodoError> {
        TodoRepository::update(&self.pool, id, title, description, due_date).await
    }

    /// Update a todo's status (complete or reopen).
    ///
    /// When completing, checks if the todo has a linked task that must be completed first.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::LinkedTaskNotCompleted` if the linked task is not completed.
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn update_todo_status(
        &self,
        id: Uuid,
        status: &TodoStatus,
        account_id: Uuid,
    ) -> Result<Todo, TodoError> {
        let current = TodoRepository::get_by_id(&self.pool, id).await?;

        // Check linked task completion constraint
        if *status == TodoStatus::Completed {
            if let Some(linked_task_id) = current.linked_task_id {
                let linked_task = TaskRepository::get_by_id(&self.pool, linked_task_id)
                    .await
                    .map_err(|_| TodoError::NotFound)?;
                if linked_task.status != TaskStatus::Completed {
                    return Err(TodoError::LinkedTaskNotCompleted);
                }
            }
        }

        let completed_by = if *status == TodoStatus::Completed {
            Some(account_id)
        } else {
            None
        };

        let todo = TodoRepository::update_status(&self.pool, id, status, completed_by).await?;

        if *status == TodoStatus::Completed {
            self.event_bus.publish(DomainEvent::TodoCompleted {
                todo_id: id,
                task_id: current.task_id,
            });
        }

        Ok(todo)
    }

    /// Delete a todo (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn delete_todo(&self, id: Uuid) -> Result<(), TodoError> {
        TodoRepository::soft_delete(&self.pool, id).await
    }

    /// Count incomplete todos for a task.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn count_incomplete(&self, task_id: Uuid) -> Result<i64, TodoError> {
        TodoRepository::count_incomplete_by_task(&self.pool, task_id).await
    }

    // ── Assignees ─────────────────────────────────────────────

    /// Add an assignee to a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::AssigneeAlreadyExists` if already assigned.
    pub async fn add_assignee(
        &self,
        todo_id: Uuid,
        member_id: Uuid,
    ) -> Result<TodoAssignee, TodoError> {
        let id = generate_id();
        TodoRepository::add_assignee(&self.pool, id, todo_id, member_id).await
    }

    /// Remove an assignee from a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::AssigneeNotFound` if the assignment does not exist.
    pub async fn remove_assignee(&self, todo_id: Uuid, member_id: Uuid) -> Result<(), TodoError> {
        TodoRepository::remove_assignee(&self.pool, todo_id, member_id).await
    }

    /// List assignees for a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn list_assignees(&self, todo_id: Uuid) -> Result<Vec<TodoAssignee>, TodoError> {
        TodoRepository::list_assignees(&self.pool, todo_id).await
    }

    /// List todos assigned to the current account across all projects.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::Database` on database failure.
    pub async fn list_my_todos(
        &self,
        account_id: Uuid,
        status_filter: Option<&TodoStatus>,
        project_id_filter: Option<Uuid>,
    ) -> Result<Vec<MyTodoItem>, TodoError> {
        TodoRepository::list_my_todos(&self.pool, account_id, status_filter, project_id_filter)
            .await
    }
}
