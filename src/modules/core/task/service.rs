use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::TaskError;
use super::models::{Task, TaskStatus};
use super::repository::{CreateTaskParams, TaskRepository};
use crate::modules::core::task_template::repository::TaskTemplateRepository;
use crate::modules::core::todo::repository::TodoRepository;

pub struct TaskService {
    pool: PgPool,
    event_bus: EventBus,
}

impl TaskService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Create a new task from a template.
    ///
    /// Validates that the template exists, that the owner tag is linked to the template,
    /// and then creates the task record.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::TemplateNotFound` if the template does not exist.
    /// Returns `TaskError::InvalidOwnerTag` if the tag is not linked to the template.
    /// Returns `TaskError::Database` on database failure.
    pub async fn create_task(
        &self,
        project_id: Uuid,
        task_template_id: Uuid,
        owner_tag_id: Uuid,
        name: &str,
        description: Option<&str>,
        created_by: Uuid,
    ) -> Result<Task, TaskError> {
        // Verify template exists and belongs to this project
        let template = TaskTemplateRepository::get_by_id(&self.pool, task_template_id)
            .await
            .map_err(|_| TaskError::TemplateNotFound)?;

        if template.project_id != project_id {
            return Err(TaskError::TemplateNotFound);
        }

        // Verify owner_tag_id is linked to the template
        let is_linked =
            TaskRepository::is_tag_linked_to_template(&self.pool, task_template_id, owner_tag_id)
                .await?;
        if !is_linked {
            return Err(TaskError::InvalidOwnerTag);
        }

        let id = generate_id();
        let task = TaskRepository::create(
            &self.pool,
            &CreateTaskParams {
                id,
                project_id,
                task_template_id,
                owner_tag_id,
                name,
                description,
                created_by,
            },
        )
        .await?;

        self.event_bus.publish(DomainEvent::TaskCreated {
            task_id: id,
            project_id,
            template_id: task_template_id,
        });

        Ok(task)
    }

    /// Get a task by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn get_task(&self, id: Uuid) -> Result<Task, TaskError> {
        TaskRepository::get_by_id(&self.pool, id).await
    }

    /// List tasks for a project with optional filters.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn list_tasks(
        &self,
        project_id: Uuid,
        status_filter: Option<&TaskStatus>,
        tag_filter: Option<Uuid>,
    ) -> Result<Vec<Task>, TaskError> {
        TaskRepository::list_by_project(&self.pool, project_id, status_filter, tag_filter).await
    }

    /// Update a task's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn update_task(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<Task, TaskError> {
        TaskRepository::update(&self.pool, id, name, description).await
    }

    /// Update a task's status with transition validation.
    ///
    /// Valid transitions:
    /// - `pending` → `in_progress`
    /// - `pending` → `completed`
    /// - `pending` → `cancelled`
    /// - `in_progress` → `completed`
    /// - `in_progress` → `cancelled`
    ///
    /// # Errors
    ///
    /// Returns `TaskError::InvalidStatusTransition` on invalid transitions.
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn update_task_status(
        &self,
        id: Uuid,
        new_status: &TaskStatus,
    ) -> Result<Task, TaskError> {
        let current = TaskRepository::get_by_id(&self.pool, id).await?;

        validate_status_transition(&current.status, new_status)?;

        if *new_status == TaskStatus::Completed {
            let incomplete = TodoRepository::count_incomplete_by_task(&self.pool, id)
                .await
                .map_err(|e| {
                    TaskError::Database(match e {
                        crate::modules::core::todo::error::TodoError::Database(db_err) => db_err,
                        _ => sqlx::Error::RowNotFound,
                    })
                })?;
            if incomplete > 0 {
                return Err(TaskError::IncompleteTodos(incomplete));
            }
        }

        let task = TaskRepository::update_status(&self.pool, id, new_status).await?;

        self.event_bus.publish(DomainEvent::TaskStatusChanged {
            task_id: id,
            project_id: current.project_id,
            old_status: format!("{:?}", current.status),
            new_status: format!("{:?}", task.status),
        });

        Ok(task)
    }

    /// Delete a task (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `TaskError::NotFound` if the task does not exist.
    pub async fn delete_task(&self, id: Uuid) -> Result<(), TaskError> {
        let task = TaskRepository::get_by_id(&self.pool, id).await?;
        TaskRepository::soft_delete(&self.pool, id).await?;

        self.event_bus.publish(DomainEvent::TaskDeleted {
            task_id: id,
            project_id: task.project_id,
        });

        Ok(())
    }
}

fn validate_status_transition(current: &TaskStatus, new: &TaskStatus) -> Result<(), TaskError> {
    let valid = matches!(
        (current, new),
        (
            TaskStatus::Pending | TaskStatus::InProgress,
            TaskStatus::Completed | TaskStatus::Cancelled
        ) | (TaskStatus::Pending, TaskStatus::InProgress)
    );

    if !valid {
        return Err(TaskError::InvalidStatusTransition {
            from: format!("{current:?}"),
            to: format!("{new:?}"),
        });
    }

    Ok(())
}
