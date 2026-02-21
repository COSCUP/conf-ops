use std::collections::HashSet;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use serde::Deserialize;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::TaskError;
use super::models::{Task, TaskStatus};
use super::repository::{CreateTaskParams, TaskRepository};
use crate::modules::core::member::repository::MemberRepository;
use crate::modules::core::member_tag::repository::MemberTagRepository;
use crate::modules::core::task_template::repository::TaskTemplateRepository;
use crate::modules::core::todo::repository::TodoRepository;

pub struct TaskService {
    pool: PgPool,
    event_bus: EventBus,
    participant_cache: Cache<Uuid, Vec<Uuid>>,
}

impl TaskService {
    pub fn new(pool: PgPool, event_bus: EventBus, cache_ttl_secs: u64) -> Self {
        let participant_cache = Cache::builder()
            .time_to_live(Duration::from_secs(cache_ttl_secs))
            .max_capacity(5_000)
            .build();

        let cache_clone = participant_cache.clone();
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                Self::handle_participant_cache_invalidation(&cache_clone, &event).await;
            }
        });

        Self {
            pool,
            event_bus,
            participant_cache,
        }
    }

    async fn handle_participant_cache_invalidation(
        cache: &Cache<Uuid, Vec<Uuid>>,
        event: &DomainEvent,
    ) {
        match event {
            DomainEvent::TodoCompleted { task_id, .. }
            | DomainEvent::TaskCreated { task_id, .. }
            | DomainEvent::TaskDeleted { task_id, .. } => {
                cache.invalidate(task_id).await;
            }
            _ => {}
        }
    }

    /// Get participants for a task (cached).
    ///
    /// Participants = ownerTag members ∪ task creator ∪ todo assignees.
    /// Aggregated at service layer (no cross-module SQL JOINs).
    ///
    /// # Errors
    ///
    /// Returns `TaskError::Database` on database failure.
    pub async fn get_task_participants(&self, task_id: Uuid) -> Result<Vec<Uuid>, TaskError> {
        if let Some(cached) = self.participant_cache.get(&task_id).await {
            return Ok(cached);
        }

        let task = TaskRepository::get_by_id(&self.pool, task_id).await?;
        let mut participants = HashSet::new();

        // Owner tag members (via MemberTagRepository — stays within member_tag module)
        if let Ok(tag_members) =
            MemberTagRepository::list_assigned_members(&self.pool, task.owner_tag_id).await
        {
            participants.extend(tag_members.iter().map(|m| m.account_id));
        }

        // Task creator
        participants.insert(task.created_by);

        // Todo assignees (TodoRepository for member_ids → MemberRepository for account_ids)
        if let Ok(assignee_member_ids) =
            TodoRepository::list_assignee_member_ids_by_task(&self.pool, task_id).await
        {
            if !assignee_member_ids.is_empty() {
                if let Ok(account_ids) =
                    MemberRepository::get_account_ids_by_ids(&self.pool, &assignee_member_ids).await
                {
                    participants.extend(account_ids);
                }
            }
        }

        let result: Vec<Uuid> = participants.into_iter().collect();
        self.participant_cache.insert(task_id, result.clone()).await;
        Ok(result)
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

        // Check external task creation permission
        let is_member =
            MemberTagRepository::is_account_member_of_tag(&self.pool, created_by, owner_tag_id)
                .await
                .map_err(|_| TaskError::Database(sqlx::Error::RowNotFound))?;

        if !is_member {
            let tag = MemberTagRepository::get_by_id(&self.pool, owner_tag_id)
                .await
                .map_err(|_| TaskError::InvalidOwnerTag)?;

            let rules: Vec<ExternalTaskCreationRule> =
                serde_json::from_value(tag.external_task_creation.clone()).unwrap_or_default();

            let rule = rules
                .iter()
                .find(|r| r.task_template_id == task_template_id);

            match rule {
                None => return Err(TaskError::ExternalCreationNotAllowed),
                Some(r) => match &r.allowed_from_tags {
                    AllowedFromTags::Wildcard(()) => {} // Allow all
                    AllowedFromTags::List(allowed_names) => {
                        let creator_tag_names = MemberTagRepository::get_account_tag_names(
                            &self.pool, created_by, project_id,
                        )
                        .await
                        .map_err(|_| TaskError::Database(sqlx::Error::RowNotFound))?;

                        let has_match = creator_tag_names
                            .iter()
                            .any(|name| allowed_names.contains(name));
                        if !has_match {
                            return Err(TaskError::ExternalCreationNotAllowed);
                        }
                    }
                },
            }
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalTaskCreationRule {
    task_template_id: Uuid,
    allowed_from_tags: AllowedFromTags,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum AllowedFromTags {
    Wildcard(#[serde(deserialize_with = "deserialize_wildcard")] ()),
    List(Vec<String>),
}

fn deserialize_wildcard<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == "*" {
        Ok(())
    } else {
        Err(serde::de::Error::custom("expected \"*\""))
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
