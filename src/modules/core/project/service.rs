use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::core::member::models::MemberRole;
use crate::modules::core::member::repository::MemberRepository;

use super::error::ProjectError;
use super::models::{Project, ProjectListItem, ProjectStatus};
use super::repository::ProjectRepository;

pub struct ProjectService {
    pool: PgPool,
    event_bus: EventBus,
}

impl ProjectService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Create a new project within an organization.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn create_project(
        &self,
        org_id: Uuid,
        name: &str,
        description: Option<&str>,
        created_by: Uuid,
    ) -> Result<Project, ProjectError> {
        let project_id = generate_id();
        let project = ProjectRepository::create(
            &self.pool,
            project_id,
            org_id,
            name,
            description,
            None,
            created_by,
        )
        .await?;

        let member_id = generate_id();
        MemberRepository::create(
            &self.pool,
            member_id,
            project_id,
            created_by,
            MemberRole::Owner,
        )
        .await
        .map_err(|e| {
            ProjectError::Database(match e {
                crate::modules::core::member::error::MemberError::Database(db_err) => db_err,
                _ => {
                    return ProjectError::Database(sqlx::Error::Protocol(
                        "Failed to create owner member".into(),
                    ))
                }
            })
        })?;

        self.event_bus.publish(DomainEvent::ProjectCreated {
            project_id,
            organization_id: org_id,
        });

        Ok(project)
    }

    /// Copy an existing project into a new project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the source project does not exist.
    pub async fn copy_project(
        &self,
        org_id: Uuid,
        source_project_id: Uuid,
        new_name: &str,
        new_description: Option<&str>,
        created_by: Uuid,
    ) -> Result<Project, ProjectError> {
        // Verify source project exists
        ProjectRepository::get_by_id(&self.pool, source_project_id).await?;

        let project_id = generate_id();
        let project = ProjectRepository::create(
            &self.pool,
            project_id,
            org_id,
            new_name,
            new_description,
            Some(source_project_id),
            created_by,
        )
        .await?;

        let member_id = generate_id();
        MemberRepository::create(
            &self.pool,
            member_id,
            project_id,
            created_by,
            MemberRole::Owner,
        )
        .await
        .map_err(|e| {
            ProjectError::Database(match e {
                crate::modules::core::member::error::MemberError::Database(db_err) => db_err,
                _ => {
                    return ProjectError::Database(sqlx::Error::Protocol(
                        "Failed to create owner member".into(),
                    ))
                }
            })
        })?;

        self.event_bus.publish(DomainEvent::ProjectCopied {
            project_id,
            source_project_id,
            organization_id: org_id,
        });

        Ok(project)
    }

    /// Update a project's name and description.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn update_project(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<Project, ProjectError> {
        ProjectRepository::update(&self.pool, project_id, name, description).await
    }

    /// Transition a project to a new status.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::InvalidStatusTransition` if the transition is not allowed.
    pub async fn update_project_status(
        &self,
        project_id: Uuid,
        new_status: ProjectStatus,
    ) -> Result<Project, ProjectError> {
        let current = ProjectRepository::get_by_id(&self.pool, project_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(ProjectError::InvalidStatusTransition {
                from: current.status.to_string(),
                to: new_status.to_string(),
            });
        }

        let project = ProjectRepository::update_status(&self.pool, project_id, new_status).await?;

        self.event_bus.publish(DomainEvent::ProjectStatusChanged {
            project_id,
            old_status: current.status.to_string(),
            new_status: new_status.to_string(),
        });

        Ok(project)
    }

    /// Soft-delete a project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn delete_project(&self, project_id: Uuid) -> Result<(), ProjectError> {
        ProjectRepository::soft_delete(&self.pool, project_id).await
    }

    /// List projects for an organization, optionally filtered by status.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn list_projects(
        &self,
        org_id: Uuid,
        status_filter: Option<ProjectStatus>,
    ) -> Result<Vec<ProjectListItem>, ProjectError> {
        ProjectRepository::list_by_org(&self.pool, org_id, status_filter).await
    }

    /// Fetch a project by ID.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn get_project(&self, project_id: Uuid) -> Result<Project, ProjectError> {
        ProjectRepository::get_by_id(&self.pool, project_id).await
    }

    /// Get the permission settings for a project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn get_permission_settings(
        &self,
        project_id: Uuid,
    ) -> Result<serde_json::Value, ProjectError> {
        let project = ProjectRepository::get_by_id(&self.pool, project_id).await?;
        Ok(project.permission_settings)
    }

    /// Update the permission settings for a project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn update_permission_settings(
        &self,
        project_id: Uuid,
        settings: &serde_json::Value,
    ) -> Result<Project, ProjectError> {
        ProjectRepository::update_permission_settings(&self.pool, project_id, settings).await
    }
}
