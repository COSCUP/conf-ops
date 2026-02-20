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
use crate::modules::core::organization::models::OrgRole;
use crate::modules::core::permission::service::{Action, Resource};

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub logo_url: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteOrgMemberRequest {
    pub email: String,
    pub role: OrgRole,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgMemberRequest {
    pub role: OrgRole,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: String,
    pub role: OrgRole,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationListResponse {
    pub organizations: Vec<OrganizationListItem>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrgMemberResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub role: OrgRole,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrgMemberListResponse {
    pub members: Vec<OrgMemberResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteMemberResponse {
    pub member_id: Uuid,
}

// ── Handlers ─────────────────────────────────────────────────

/// Create a new organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations",
    tag = "organizations",
    security(("bearer_auth" = [])),
    request_body = CreateOrganizationRequest,
    responses(
        (status = 201, description = "Organization created", body = OrganizationResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn create_organization(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateOrganizationRequest>,
) -> Result<(StatusCode, Json<OrganizationResponse>), ProblemDetails> {
    let org = state
        .org_service
        .create_organization(
            user.account_id,
            &body.name,
            body.description.as_deref(),
            body.logo_url.as_deref(),
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(OrganizationResponse {
            id: org.id,
            name: org.name,
            description: org.description,
            logo_url: org.logo_url,
            created_by: org.created_by,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        }),
    ))
}

/// List organizations the current user belongs to.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations",
    tag = "organizations",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of user's organizations", body = OrganizationListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn list_organizations(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<OrganizationListResponse>, ProblemDetails> {
    let orgs = state
        .org_service
        .list_my_organizations(user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    let organizations = orgs
        .into_iter()
        .map(|o| OrganizationListItem {
            id: o.id,
            name: o.name,
            description: o.description,
            logo_url: o.logo_url,
            created_at: o.created_at.to_rfc3339(),
            role: o.role,
        })
        .collect();

    Ok(Json(OrganizationListResponse { organizations }))
}

/// Get organization details by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization details", body = OrganizationResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_organization(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<OrganizationResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::ViewOrganization,
    )
    .await?;

    let org = state
        .org_service
        .get_organization(org_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(OrganizationResponse {
        id: org.id,
        name: org.name,
        description: org.description,
        logo_url: org.logo_url,
        created_by: org.created_by,
        created_at: org.created_at.to_rfc3339(),
        updated_at: org.updated_at.to_rfc3339(),
    }))
}

/// Update an existing organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{orgId}",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = UpdateOrganizationRequest,
    responses(
        (status = 200, description = "Organization updated", body = OrganizationResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_organization(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::UpdateOrganization,
    )
    .await?;

    let description = body.description.as_ref().map(|d| d.as_deref());
    let logo_url = body.logo_url.as_ref().map(|l| l.as_deref());

    let org = state
        .org_service
        .update_organization(org_id, body.name.as_deref(), description, logo_url)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(OrganizationResponse {
        id: org.id,
        name: org.name,
        description: org.description,
        logo_url: org.logo_url,
        created_by: org.created_by,
        created_at: org.created_at.to_rfc3339(),
        updated_at: org.updated_at.to_rfc3339(),
    }))
}

/// Delete an organization (soft delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{orgId}",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 204, description = "Organization deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Has active projects", body = ProblemDetails)
    )
)]
pub async fn delete_organization(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::DeleteOrganization,
    )
    .await?;

    state
        .org_service
        .delete_organization(org_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List members of an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/members",
    operation_id = "list_org_members",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Member list", body = OrgMemberListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<OrgMemberListResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::ViewOrganization,
    )
    .await?;

    let members = state
        .org_service
        .list_members(org_id)
        .await
        .map_err(ProblemDetails::from)?;

    let members = members
        .into_iter()
        .map(|m| OrgMemberResponse {
            id: m.id,
            account_id: m.account_id,
            role: m.role,
            name: m.name,
            email: m.email,
            avatar_url: m.avatar_url,
            created_at: m.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OrgMemberListResponse { members }))
}

/// Invite a new member to an organization by email.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{orgId}/members/invite",
    operation_id = "invite_org_member",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = InviteOrgMemberRequest,
    responses(
        (status = 201, description = "Member invited", body = InviteMemberResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Already a member", body = ProblemDetails)
    )
)]
pub async fn invite_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<InviteOrgMemberRequest>,
) -> Result<(StatusCode, Json<InviteMemberResponse>), ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::InviteMember,
    )
    .await?;

    let member_id = state
        .org_service
        .invite_member(org_id, &body.email, body.role)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(InviteMemberResponse { member_id }),
    ))
}

/// Update a member's role within an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{orgId}/members/{memberId}",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("memberId" = Uuid, Path, description = "Member ID")
    ),
    request_body = UpdateOrgMemberRequest,
    responses(
        (status = 204, description = "Member role updated"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Last owner removal", body = ProblemDetails)
    )
)]
pub async fn update_member_role(
    State(state): State<AppState>,
    user: AuthUser,
    Path((org_id, member_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateOrgMemberRequest>,
) -> Result<StatusCode, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::UpdateMemberRole,
    )
    .await?;

    state
        .org_service
        .update_member_role(org_id, member_id, body.role)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Remove a member from an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{orgId}/members/{memberId}",
    tag = "organizations",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("memberId" = Uuid, Path, description = "Member ID")
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Last owner removal", body = ProblemDetails)
    )
)]
pub async fn remove_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path((org_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::RemoveMember,
    )
    .await?;

    state
        .org_service
        .remove_member(org_id, member_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}
