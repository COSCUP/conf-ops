use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::extractors::permission::require_permission;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::models::ProjectStatus;
use crate::modules::core::project::repository::ProjectRepository;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CopyProjectRequest {
    pub source_project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectStatusRequest {
    pub status: ProjectStatus,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePermissionSettingsRequest {
    #[schema(value_type = Object)]
    pub settings: serde_json::Value,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ProjectListQuery {
    pub status: Option<ProjectStatus>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub source_project_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItemResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectListItemResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSettingsResponse {
    #[schema(value_type = Object)]
    pub settings: serde_json::Value,
}

// ── Nested under /organizations/{orgId}/projects ──────────────

/// Create a new project within an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{orgId}/projects",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created", body = ProjectResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::CreateProject,
    )
    .await?;

    let project = state
        .project_service
        .create_project(
            org_id,
            &body.name,
            body.description.as_deref(),
            user.account_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(project_to_response(project))))
}

/// List projects belonging to an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/projects",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ProjectListQuery,
    ),
    responses(
        (status = 200, description = "Project list", body = ProjectListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_projects(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ProjectListQuery>,
) -> Result<Json<ProjectListResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::ViewOrganization,
    )
    .await?;

    let items = state
        .project_service
        .list_projects(org_id, query.status)
        .await
        .map_err(ProblemDetails::from)?;

    let projects = items
        .into_iter()
        .map(|p| ProjectListItemResponse {
            id: p.id,
            name: p.name,
            description: p.description,
            status: p.status,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ProjectListResponse { projects }))
}

/// Copy an existing project into a new project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{orgId}/projects/copy",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = CopyProjectRequest,
    responses(
        (status = 201, description = "Project copied", body = ProjectResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Source project not found", body = ProblemDetails)
    )
)]
pub async fn copy_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CopyProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::CreateProject,
    )
    .await?;

    let project = state
        .project_service
        .copy_project(
            org_id,
            body.source_project_id,
            &body.name,
            body.description.as_deref(),
            user.account_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(project_to_response(project))))
}

// ── Top-level /projects/{projectId} ───────────────────────────

/// Get project details by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Project details", body = ProjectResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ProjectResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::ViewProject,
    )
    .await?;

    let project = state
        .project_service
        .get_project(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(project_to_response(project)))
}

/// Update an existing project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated", body = ProjectResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::UpdateProject,
    )
    .await?;

    let description = body.description.as_ref().map(|d| d.as_deref());

    let project = state
        .project_service
        .update_project(project_id, body.name.as_deref(), description)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(project_to_response(project)))
}

/// Delete a project (soft delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 204, description = "Project deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::DeleteProject,
    )
    .await?;

    state
        .project_service
        .delete_project(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update the status of a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/status",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = UpdateProjectStatusRequest,
    responses(
        (status = 200, description = "Status updated", body = ProjectResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Invalid status transition", body = ProblemDetails)
    )
)]
pub async fn update_project_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<UpdateProjectStatusRequest>,
) -> Result<Json<ProjectResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::UpdateProjectStatus,
    )
    .await?;

    let project = state
        .project_service
        .update_project_status(project_id, body.status)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(project_to_response(project)))
}

/// Get permission settings for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/permission-settings",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Permission settings", body = PermissionSettingsResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn get_permission_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<PermissionSettingsResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::ViewPermissionSettings,
    )
    .await?;

    let settings = state
        .project_service
        .get_permission_settings(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(PermissionSettingsResponse { settings }))
}

/// Update permission settings for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/permission-settings",
    tag = "projects",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = UpdatePermissionSettingsRequest,
    responses(
        (status = 200, description = "Permission settings updated", body = PermissionSettingsResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn update_permission_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<UpdatePermissionSettingsRequest>,
) -> Result<Json<PermissionSettingsResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::Project { org_id },
        Action::UpdatePermissionSettings,
    )
    .await?;

    let project = state
        .project_service
        .update_permission_settings(project_id, &body.settings)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(PermissionSettingsResponse {
        settings: project.permission_settings,
    }))
}

// ── Helpers ───────────────────────────────────────────────────

fn project_to_response(p: crate::modules::core::project::models::Project) -> ProjectResponse {
    ProjectResponse {
        id: p.id,
        organization_id: p.organization_id,
        name: p.name,
        description: p.description,
        status: p.status,
        source_project_id: p.source_project_id,
        created_by: p.created_by,
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}
