use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::core::crdt::repository::CrdtRepository;

use super::error::TodoError;

/// Entry for creating todos from templates.
pub type TemplateTodoEntry = (Uuid, Option<Uuid>, String, Option<String>, i32);
use super::models::{MyTodoItem, Todo, TodoAssignee, TodoStatus, TodoType};
use super::repository::{CreateTodoParams, TodoRepository};
use crate::modules::core::member::repository::MemberRepository;
use crate::modules::core::project::repository::ProjectRepository;
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
        account_id: Uuid,
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
        let todo = TodoRepository::create(
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
        .await?;

        // Best-effort CRDT recording
        let crdt_op = serde_json::json!({
            "type": "create",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ =
            CrdtRepository::insert_operation(&self.pool, op_id, "todo", id, &op_bytes, account_id)
                .await;

        Ok(todo)
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
        account_id: Uuid,
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

        // Best-effort CRDT recording for batch creation
        for todo in &created {
            let crdt_op = serde_json::json!({
                "type": "create_from_template",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
            let op_id = generate_id();
            let _ = CrdtRepository::insert_operation(
                &self.pool, op_id, "todo", todo.id, &op_bytes, account_id,
            )
            .await;
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
        account_id: Uuid,
    ) -> Result<Todo, TodoError> {
        let todo = TodoRepository::update(&self.pool, id, title, description, due_date).await?;

        let crdt_op = serde_json::json!({
            "type": "update",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ =
            CrdtRepository::insert_operation(&self.pool, op_id, "todo", id, &op_bytes, account_id)
                .await;

        Ok(todo)
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

        // Record CRDT operation for status change
        let crdt_op = serde_json::json!({
            "type": "status_change",
            "status": format!("{status:?}"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        // Best-effort CRDT recording — don't fail the main operation
        let _ =
            CrdtRepository::insert_operation(&self.pool, op_id, "todo", id, &op_bytes, account_id)
                .await;

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
    pub async fn delete_todo(&self, id: Uuid, account_id: Uuid) -> Result<(), TodoError> {
        TodoRepository::soft_delete(&self.pool, id).await?;

        let crdt_op = serde_json::json!({
            "type": "delete",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ =
            CrdtRepository::insert_operation(&self.pool, op_id, "todo", id, &op_bytes, account_id)
                .await;

        Ok(())
    }

    /// Link a task to a todo (cross-task dependency).
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn link_task(
        &self,
        todo_id: Uuid,
        linked_task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Todo, TodoError> {
        let todo =
            TodoRepository::update_linked_task(&self.pool, todo_id, Some(linked_task_id)).await?;

        let crdt_op = serde_json::json!({
            "type": "link_task",
            "linked_task_id": linked_task_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool, op_id, "todo", todo_id, &op_bytes, account_id,
        )
        .await;

        Ok(todo)
    }

    /// Unlink a task from a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::NotFound` if the todo does not exist.
    pub async fn unlink_task(&self, todo_id: Uuid, account_id: Uuid) -> Result<Todo, TodoError> {
        let todo = TodoRepository::update_linked_task(&self.pool, todo_id, None).await?;

        let crdt_op = serde_json::json!({
            "type": "unlink_task",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool, op_id, "todo", todo_id, &op_bytes, account_id,
        )
        .await;

        Ok(todo)
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
        account_id: Uuid,
    ) -> Result<TodoAssignee, TodoError> {
        let id = generate_id();
        let assignee = TodoRepository::add_assignee(&self.pool, id, todo_id, member_id).await?;

        let crdt_op = serde_json::json!({
            "type": "add_assignee",
            "member_id": member_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool, op_id, "todo", todo_id, &op_bytes, account_id,
        )
        .await;

        Ok(assignee)
    }

    /// Remove an assignee from a todo.
    ///
    /// # Errors
    ///
    /// Returns `TodoError::AssigneeNotFound` if the assignment does not exist.
    pub async fn remove_assignee(
        &self,
        todo_id: Uuid,
        member_id: Uuid,
        account_id: Uuid,
    ) -> Result<(), TodoError> {
        TodoRepository::remove_assignee(&self.pool, todo_id, member_id).await?;

        let crdt_op = serde_json::json!({
            "type": "remove_assignee",
            "member_id": member_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool, op_id, "todo", todo_id, &op_bytes, account_id,
        )
        .await;

        Ok(())
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
    /// Aggregated at service layer (no cross-module SQL JOINs):
    /// 1. `MemberRepository` → get `member_ids` for account
    /// 2. `TodoRepository` → get assigned todos (todo-module tables only)
    /// 3. `TaskRepository` → get task names
    /// 4. `ProjectRepository` → get project names
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
        // 1. Get all member_ids for this account
        let member_ids = MemberRepository::get_ids_by_account_id(&self.pool, account_id)
            .await
            .map_err(|e| {
                TodoError::Database(match e {
                    crate::modules::core::member::error::MemberError::Database(db) => db,
                    _ => sqlx::Error::RowNotFound,
                })
            })?;

        if member_ids.is_empty() {
            return Ok(vec![]);
        }

        // 2. Get todos assigned to these members (todo-module tables only)
        let todos =
            TodoRepository::list_todos_for_members(&self.pool, &member_ids, status_filter).await?;

        if todos.is_empty() {
            return Ok(vec![]);
        }

        // 3. Get unique task_ids and fetch task info
        let task_ids: Vec<Uuid> = todos
            .iter()
            .map(|t| t.task_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let tasks = TaskRepository::get_by_ids(&self.pool, &task_ids)
            .await
            .map_err(|e| {
                TodoError::Database(match e {
                    crate::modules::core::task::error::TaskError::Database(db) => db,
                    _ => sqlx::Error::RowNotFound,
                })
            })?;

        let task_map: std::collections::HashMap<Uuid, &crate::modules::core::task::models::Task> =
            tasks.iter().map(|t| (t.id, t)).collect();

        // 4. Get unique project_ids and fetch project names
        let project_ids: Vec<Uuid> = tasks
            .iter()
            .map(|t| t.project_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let project_names = ProjectRepository::get_names_by_ids(&self.pool, &project_ids)
            .await
            .map_err(|e| {
                TodoError::Database(match e {
                    crate::modules::core::project::error::ProjectError::Database(db) => db,
                    _ => sqlx::Error::RowNotFound,
                })
            })?;

        let project_map: std::collections::HashMap<Uuid, String> =
            project_names.into_iter().collect();

        // 5. Assemble MyTodoItem, filtering by project_id if specified
        let items: Vec<MyTodoItem> = todos
            .into_iter()
            .filter_map(|todo| {
                let task = task_map.get(&todo.task_id)?;
                let project_name = project_map.get(&task.project_id)?;

                // Apply project_id filter
                if let Some(filter_pid) = project_id_filter {
                    if task.project_id != filter_pid {
                        return None;
                    }
                }

                Some(MyTodoItem {
                    id: todo.id,
                    task_id: todo.task_id,
                    title: todo.title,
                    description: todo.description,
                    status: todo.status,
                    todo_type: todo.todo_type,
                    due_date: todo.due_date,
                    project_id: task.project_id,
                    project_name: project_name.clone(),
                    task_name: task.name.clone(),
                    created_at: todo.created_at,
                    updated_at: todo.updated_at,
                })
            })
            .collect();

        Ok(items)
    }
}
