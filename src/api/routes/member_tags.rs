use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::extractors::permission::require_permission;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignTagRequest {
    pub member_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExternalTaskCreationRequest {
    #[schema(value_type = Object)]
    pub settings: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberTagResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[schema(value_type = Object)]
    pub external_task_creation: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberTagListItemResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub member_count: i64,
    pub contact_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberTagListResponse {
    pub tags: Vec<MemberTagListItemResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagAssignedMemberResponse {
    pub assignment_id: Uuid,
    pub member_id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagAssignedContactResponse {
    pub assignment_id: Uuid,
    pub contact_id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberTagDetailResponse {
    pub tag: MemberTagResponse,
    pub members: Vec<TagAssignedMemberResponse>,
    pub contacts: Vec<TagAssignedContactResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentResponse {
    pub id: Uuid,
    pub tag_id: Uuid,
    pub member_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub project_id: Uuid,
    pub created_at: String,
}

// ── Handlers ──────────────────────────────────────────────────

/// List member tags for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/member-tags",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Tag list", body = MemberTagListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_tags(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<MemberTagListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTags,
    )
    .await?;

    let items = state
        .member_tag_service
        .list_tags(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    let tags = items
        .into_iter()
        .map(|t| MemberTagListItemResponse {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            description: t.description,
            member_count: t.member_count,
            contact_count: t.contact_count,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(MemberTagListResponse { tags }))
}

/// Create a new member tag.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/member-tags",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = CreateTagRequest,
    responses(
        (status = 201, description = "Tag created", body = MemberTagResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<MemberTagResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::CreateTag,
    )
    .await?;

    let tag = state
        .member_tag_service
        .create_tag(project_id, &body.name, body.description.as_deref())
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(tag_to_response(tag))))
}

/// Get a member tag detail (with assigned members and contacts).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
    ),
    responses(
        (status = 200, description = "Tag detail", body = MemberTagDetailResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MemberTagDetailResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTags,
    )
    .await?;

    let (tag, members, contacts) = state
        .member_tag_service
        .get_tag_detail(tag_id)
        .await
        .map_err(ProblemDetails::from)?;

    let members = members
        .into_iter()
        .map(|m| TagAssignedMemberResponse {
            assignment_id: m.assignment_id,
            member_id: m.member_id,
            account_id: m.account_id,
            name: m.name,
            email: m.email,
            avatar_url: m.avatar_url,
        })
        .collect();

    let contacts = contacts
        .into_iter()
        .map(|c| TagAssignedContactResponse {
            assignment_id: c.assignment_id,
            contact_id: c.contact_id,
            name: c.name,
            email: c.email,
        })
        .collect();

    Ok(Json(MemberTagDetailResponse {
        tag: tag_to_response(tag),
        members,
        contacts,
    }))
}

/// Update a member tag.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
    ),
    request_body = UpdateTagRequest,
    responses(
        (status = 200, description = "Tag updated", body = MemberTagResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTagRequest>,
) -> Result<Json<MemberTagResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTag,
    )
    .await?;

    let description = body
        .description
        .map(|d| d.map(|s| -> Box<str> { s.into_boxed_str() }));
    let description_ref = description.as_ref().map(|d| d.as_deref());

    let tag = state
        .member_tag_service
        .update_tag(tag_id, body.name.as_deref(), description_ref)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(tag_to_response(tag)))
}

/// Delete a member tag.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
    ),
    responses(
        (status = 204, description = "Tag deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::DeleteTag,
    )
    .await?;

    state
        .member_tag_service
        .delete_tag(tag_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Assign a tag to a member or contact.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}/assign",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
    ),
    request_body = AssignTagRequest,
    responses(
        (status = 201, description = "Assignment created", body = AssignmentResponse),
        (status = 400, description = "Invalid assignment", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Assignment already exists", body = ProblemDetails)
    )
)]
pub async fn assign_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<AssignTagRequest>,
) -> Result<(StatusCode, Json<AssignmentResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::AssignTag,
    )
    .await?;

    let assignment = state
        .member_tag_service
        .assign(tag_id, body.member_id, body.contact_id, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(AssignmentResponse {
            id: assignment.id,
            tag_id: assignment.tag_id,
            member_id: assignment.member_id,
            contact_id: assignment.contact_id,
            project_id: assignment.project_id,
            created_at: assignment.created_at.to_rfc3339(),
        }),
    ))
}

/// Remove a tag assignment.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}/assignments/{assignmentId}",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
        ("assignmentId" = Uuid, Path, description = "Assignment ID"),
    ),
    responses(
        (status = 204, description = "Assignment removed"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Assignment not found", body = ProblemDetails)
    )
)]
pub async fn remove_assignment(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id, assignment_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UnassignTag,
    )
    .await?;

    state
        .member_tag_service
        .unassign(tag_id, assignment_id, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update external task creation settings for a tag.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/member-tags/{tagId}/external-task-creation",
    tag = "member-tags",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("tagId" = Uuid, Path, description = "Tag ID"),
    ),
    request_body = UpdateExternalTaskCreationRequest,
    responses(
        (status = 200, description = "Settings updated", body = MemberTagResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_external_task_creation(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tag_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateExternalTaskCreationRequest>,
) -> Result<Json<MemberTagResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateExternalTaskCreation,
    )
    .await?;

    let tag = state
        .member_tag_service
        .update_external_task_creation(tag_id, &body.settings)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(tag_to_response(tag)))
}

// ── Helpers ───────────────────────────────────────────────────

fn tag_to_response(t: crate::modules::core::member_tag::models::MemberTag) -> MemberTagResponse {
    MemberTagResponse {
        id: t.id,
        project_id: t.project_id,
        name: t.name,
        description: t.description,
        external_task_creation: t.external_task_creation,
        created_at: t.created_at.to_rfc3339(),
        updated_at: t.updated_at.to_rfc3339(),
    }
}
