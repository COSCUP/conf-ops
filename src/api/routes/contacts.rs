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

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactRequest {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MergeContactsRequest {
    pub source_ids: Vec<Uuid>,
    pub target_id: Uuid,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ContactListQuery {
    pub search: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContactResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub email: String,
    pub merged_into_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContactListResponse {
    pub contacts: Vec<ContactResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MergeContactsResponse {
    pub merged_count: usize,
}

// ── Route Handlers ────────────────────────────────────────────

/// List contacts for an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/contacts",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ContactListQuery,
    ),
    responses(
        (status = 200, description = "Contact list", body = ContactListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_contacts(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ContactListQuery>,
) -> Result<Json<ContactListResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::ViewOrganization,
    )
    .await?;

    let contacts = state
        .contact_service
        .list_contacts(org_id, query.search.as_deref())
        .await
        .map_err(ProblemDetails::from)?;

    let contacts = contacts.into_iter().map(contact_to_response).collect();

    Ok(Json(ContactListResponse { contacts }))
}

/// Create a new contact within an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{orgId}/contacts",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = CreateContactRequest,
    responses(
        (status = 201, description = "Contact created", body = ContactResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_contact(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CreateContactRequest>,
) -> Result<(StatusCode, Json<ContactResponse>), ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::CreateContact,
    )
    .await?;

    let contact = state
        .contact_service
        .create_contact(org_id, &body.name, &body.email)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(contact_to_response(contact))))
}

/// Get contact details by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/contacts/{contactId}",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("contactId" = Uuid, Path, description = "Contact ID"),
    ),
    responses(
        (status = 200, description = "Contact details", body = ContactResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_contact(
    State(state): State<AppState>,
    user: AuthUser,
    Path((org_id, contact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ContactResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::ViewOrganization,
    )
    .await?;

    let contact = state
        .contact_service
        .get_contact(contact_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(contact_to_response(contact)))
}

/// Update an existing contact.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{orgId}/contacts/{contactId}",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("contactId" = Uuid, Path, description = "Contact ID"),
    ),
    request_body = UpdateContactRequest,
    responses(
        (status = 200, description = "Contact updated", body = ContactResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_contact(
    State(state): State<AppState>,
    user: AuthUser,
    Path((org_id, contact_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateContactRequest>,
) -> Result<Json<ContactResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::UpdateContact,
    )
    .await?;

    let contact = state
        .contact_service
        .update_contact(contact_id, body.name.as_deref(), body.email.as_deref())
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(contact_to_response(contact)))
}

/// Delete a contact (soft delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{orgId}/contacts/{contactId}",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("contactId" = Uuid, Path, description = "Contact ID"),
    ),
    responses(
        (status = 204, description = "Contact deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_contact(
    State(state): State<AppState>,
    user: AuthUser,
    Path((org_id, contact_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::DeleteContact,
    )
    .await?;

    state
        .contact_service
        .delete_contact(contact_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Merge multiple contacts into one target contact.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{orgId}/contacts/merge",
    tag = "contacts",
    security(("bearer_auth" = [])),
    params(("orgId" = Uuid, Path, description = "Organization ID")),
    request_body = MergeContactsRequest,
    responses(
        (status = 200, description = "Contacts merged", body = MergeContactsResponse),
        (status = 400, description = "Invalid merge request", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Conflict", body = ProblemDetails)
    )
)]
pub async fn merge_contacts(
    State(state): State<AppState>,
    user: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(body): Json<MergeContactsRequest>,
) -> Result<Json<MergeContactsResponse>, ProblemDetails> {
    require_permission(
        &state,
        user.account_id,
        Resource::Organization { org_id },
        Action::MergeContacts,
    )
    .await?;

    let merged_count = state
        .contact_service
        .merge_contacts(org_id, body.source_ids, body.target_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(MergeContactsResponse { merged_count }))
}

// ── Helpers ───────────────────────────────────────────────────

fn contact_to_response(c: crate::modules::core::contact::models::Contact) -> ContactResponse {
    ContactResponse {
        id: c.id,
        organization_id: c.organization_id,
        name: c.name,
        email: c.email,
        merged_into_id: c.merged_into_id,
        created_at: c.created_at.to_rfc3339(),
        updated_at: c.updated_at.to_rfc3339(),
    }
}
