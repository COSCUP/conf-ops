use sqlx::PgPool;
use uuid::Uuid;

use super::error::ProjectError;
use super::models::{Project, ProjectListItem, ProjectStatus};

pub struct ProjectRepository;

impl ProjectRepository {
    /// Insert a new project into the database.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        organization_id: Uuid,
        name: &str,
        description: Option<&str>,
        source_project_id: Option<Uuid>,
        created_by: Uuid,
    ) -> Result<Project, ProjectError> {
        sqlx::query_as!(
            Project,
            r#"INSERT INTO projects (id, organization_id, name, description, source_project_id, created_by)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, organization_id, name, description,
                       status as "status: ProjectStatus",
                       source_project_id, permission_settings,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            organization_id,
            name,
            description,
            source_project_id,
            created_by,
        )
        .fetch_one(pool)
        .await
        .map_err(ProjectError::Database)
    }

    /// Fetch a project by ID.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Project, ProjectError> {
        sqlx::query_as!(
            Project,
            r#"SELECT id, organization_id, name, description,
                      status as "status: ProjectStatus",
                      source_project_id, permission_settings,
                      created_by, created_at, updated_at, deleted_at
             FROM projects
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ProjectError::NotFound)
    }

    /// Update a project's name and description.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<Project, ProjectError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };

        sqlx::query_as!(
            Project,
            r#"UPDATE projects
             SET name = $2, description = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, organization_id, name, description,
                       status as "status: ProjectStatus",
                       source_project_id, permission_settings,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ProjectError::NotFound)
    }

    /// Soft-delete a project by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), ProjectError> {
        let result = sqlx::query!(
            "UPDATE projects SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ProjectError::NotFound);
        }

        Ok(())
    }

    /// Update a project's status.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: ProjectStatus,
    ) -> Result<Project, ProjectError> {
        sqlx::query_as!(
            Project,
            r#"UPDATE projects
             SET status = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, organization_id, name, description,
                       status as "status: ProjectStatus",
                       source_project_id, permission_settings,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            status as ProjectStatus,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ProjectError::NotFound)
    }

    /// Update the permission settings JSON for a project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn update_permission_settings(
        pool: &PgPool,
        id: Uuid,
        settings: &serde_json::Value,
    ) -> Result<Project, ProjectError> {
        sqlx::query_as!(
            Project,
            r#"UPDATE projects
             SET permission_settings = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, organization_id, name, description,
                       status as "status: ProjectStatus",
                       source_project_id, permission_settings,
                       created_by, created_at, updated_at, deleted_at"#,
            id,
            settings,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ProjectError::NotFound)
    }

    /// List projects for an organization, optionally filtered by status.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn list_by_org(
        pool: &PgPool,
        organization_id: Uuid,
        status_filter: Option<ProjectStatus>,
    ) -> Result<Vec<ProjectListItem>, ProjectError> {
        let items = if let Some(status) = status_filter {
            sqlx::query_as!(
                ProjectListItem,
                r#"SELECT id, name, description,
                          status as "status: ProjectStatus",
                          created_at, updated_at
                 FROM projects
                 WHERE organization_id = $1 AND status = $2 AND deleted_at IS NULL
                 ORDER BY created_at DESC"#,
                organization_id,
                status as ProjectStatus,
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                ProjectListItem,
                r#"SELECT id, name, description,
                          status as "status: ProjectStatus",
                          created_at, updated_at
                 FROM projects
                 WHERE organization_id = $1 AND deleted_at IS NULL
                 ORDER BY created_at DESC"#,
                organization_id,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(items)
    }

    /// Count non-archived projects in an organization.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn count_not_archived_by_org(
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<i64, ProjectError> {
        let record = sqlx::query!(
            r#"SELECT COUNT(*) as "count!" FROM projects
             WHERE organization_id = $1 AND status != 'archived' AND deleted_at IS NULL"#,
            organization_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(record.count)
    }

    /// Get project names for a batch of project IDs.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::Database` on database failure.
    pub async fn get_names_by_ids(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, ProjectError> {
        let rows = sqlx::query!(
            r#"SELECT id, name FROM projects WHERE id = ANY($1) AND deleted_at IS NULL"#,
            ids,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.id, r.name)).collect())
    }

    /// Get the organization ID for a given project.
    ///
    /// # Errors
    ///
    /// Returns `ProjectError::NotFound` if the project does not exist.
    pub async fn get_organization_id(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Uuid, ProjectError> {
        let record = sqlx::query!(
            "SELECT organization_id FROM projects WHERE id = $1 AND deleted_at IS NULL",
            project_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ProjectError::NotFound)?;

        Ok(record.organization_id)
    }
}
