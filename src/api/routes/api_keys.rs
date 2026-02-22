use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;

// ── Request/Response Types ──────────────────────────────────

/// Create API key request body.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: serde_json::Value,
}

/// API key response item (without raw key).
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub permissions: serde_json::Value,
    pub created_by: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// API key created response (includes raw key, shown only once).
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreatedResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub permissions: serde_json::Value,
    pub created_at: String,
}

/// List API keys response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListApiKeysResponse {
    pub data: Vec<ApiKeyResponse>,
}

// ── Handlers ────────────────────────────────────────────────

/// Create an API key for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/api-keys",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created (raw key shown only once)", body = ApiKeyCreatedResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    tag = "api-keys",
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyCreatedResponse>), ProblemDetails> {
    let result = state
        .api_key_service
        .create_api_key(project_id, body.name, body.permissions, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreatedResponse {
            id: result.api_key_record.id,
            name: result.api_key_record.name,
            api_key: result.raw_key,
            permissions: result.api_key_record.permissions,
            created_at: result.api_key_record.created_at.to_rfc3339(),
        }),
    ))
}

/// List API keys for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/api-keys",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "API key list", body = ListApiKeysResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "api-keys",
)]
pub async fn list_api_keys(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ListApiKeysResponse>, ProblemDetails> {
    let keys = state
        .api_key_service
        .list_api_keys(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    let data = keys.into_iter().map(api_key_to_response).collect();
    Ok(Json(ListApiKeysResponse { data }))
}

/// Revoke (delete) an API key.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/api-keys/{keyId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("keyId" = Uuid, Path, description = "API Key ID"),
    ),
    responses(
        (status = 204, description = "API key revoked"),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "api-keys",
)]
pub async fn delete_api_key(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, key_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .api_key_service
        .revoke_api_key(key_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────

fn api_key_to_response(k: crate::modules::core::api_key::models::ApiKey) -> ApiKeyResponse {
    ApiKeyResponse {
        id: k.id,
        project_id: k.project_id,
        name: k.name,
        permissions: k.permissions,
        created_by: k.created_by,
        last_used_at: k.last_used_at.map(|t| t.to_rfc3339()),
        created_at: k.created_at.to_rfc3339(),
    }
}
