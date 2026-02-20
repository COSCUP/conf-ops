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
use crate::modules::core::member::models::MemberRole;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteMemberRequest {
    pub account_id: Uuid,
    pub role: MemberRole,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRoleRequest {
    pub role: MemberRole,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct MemberListQuery {
    pub role: Option<MemberRole>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: Uuid,
    pub role: MemberRole,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberDetailResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: Uuid,
    pub role: MemberRole,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberListResponse {
    pub members: Vec<MemberDetailResponse>,
}

// ── Path extraction ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProjectMemberPath {
    #[serde(rename = "projectId")]
    pub project_id: Uuid,
    #[serde(rename = "memberId")]
    pub member_id: Uuid,
}

// ── Handlers ──────────────────────────────────────────────────

/// List members of a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/members",
    tag = "members",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        MemberListQuery,
    ),
    responses(
        (status = 200, description = "Member list", body = MemberListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<MemberListQuery>,
) -> Result<Json<MemberListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewProjectMembers,
    )
    .await?;

    let items = state
        .member_service
        .list_members(project_id, query.role)
        .await
        .map_err(ProblemDetails::from)?;

    let members = items
        .into_iter()
        .map(|m| MemberDetailResponse {
            id: m.id,
            project_id: m.project_id,
            account_id: m.account_id,
            role: m.role,
            name: m.name,
            email: m.email,
            avatar_url: m.avatar_url,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(MemberListResponse { members }))
}

/// Invite a member to a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/members/invite",
    tag = "members",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = InviteMemberRequest,
    responses(
        (status = 201, description = "Member invited", body = MemberResponse),
        (status = 400, description = "Not an organization member", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Member already exists", body = ProblemDetails)
    )
)]
pub async fn invite_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<InviteMemberRequest>,
) -> Result<(StatusCode, Json<MemberResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::InviteProjectMember,
    )
    .await?;

    let member = state
        .member_service
        .invite_member(project_id, body.account_id, body.role, body.tag_ids)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(member_to_response(&member))))
}

/// Get a project member by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/members/{memberId}",
    tag = "members",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("memberId" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Member details", body = MemberResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<ProjectMemberPath>,
) -> Result<Json<MemberResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, path.project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped {
            org_id,
            project_id: path.project_id,
        },
        Action::ViewProjectMembers,
    )
    .await?;

    let member = state
        .member_service
        .get_member(path.member_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(member_to_response(&member)))
}

/// Update a project member's role.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/members/{memberId}",
    tag = "members",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("memberId" = Uuid, Path, description = "Member ID"),
    ),
    request_body = UpdateMemberRoleRequest,
    responses(
        (status = 200, description = "Member updated", body = MemberResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails),
        (status = 409, description = "Last owner removal", body = ProblemDetails)
    )
)]
pub async fn update_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<ProjectMemberPath>,
    Json(body): Json<UpdateMemberRoleRequest>,
) -> Result<Json<MemberResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, path.project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped {
            org_id,
            project_id: path.project_id,
        },
        Action::UpdateProjectMemberRole,
    )
    .await?;

    let member = state
        .member_service
        .update_member_role(path.member_id, body.role)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(member_to_response(&member)))
}

/// Remove a member from a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/members/{memberId}",
    tag = "members",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("memberId" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails),
        (status = 409, description = "Last owner removal", body = ProblemDetails)
    )
)]
pub async fn delete_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<ProjectMemberPath>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, path.project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped {
            org_id,
            project_id: path.project_id,
        },
        Action::RemoveProjectMember,
    )
    .await?;

    state
        .member_service
        .remove_member(path.member_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ───────────────────────────────────────────────────

fn member_to_response(m: &crate::modules::core::member::models::Member) -> MemberResponse {
    MemberResponse {
        id: m.id,
        project_id: m.project_id,
        account_id: m.account_id,
        role: m.role,
        created_at: m.created_at.to_rfc3339(),
        updated_at: m.updated_at.to_rfc3339(),
    }
}
