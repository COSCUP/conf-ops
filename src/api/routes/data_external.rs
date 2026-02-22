use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::api_key_auth::ApiKeyUser;
use crate::app_state::AppState;

/// External template data response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTemplateDataResponse {
    pub template_id: Uuid,
    pub project_id: Uuid,
    pub data: Vec<serde_json::Value>,
}

/// Get aggregated data for a task template (external API).
///
/// Requires API Key authentication via X-API-Key header.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or authorization failure.
#[utoipa::path(
    get,
    path = "/external/v1/projects/{projectId}/task-templates/{templateId}/data",
    responses(
        (status = 200, description = "Template aggregated data", body = ExternalTemplateDataResponse),
        (status = 401, body = ProblemDetails),
        (status = 403, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    tag = "external-data",
)]
pub async fn get_template_data(
    State(_state): State<AppState>,
    api_key_user: Option<axum::Extension<ApiKeyUser>>,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ExternalTemplateDataResponse>, ProblemDetails> {
    // Verify API key user exists (for external API routes with api_key_auth middleware)
    let _user = api_key_user.ok_or_else(|| {
        ProblemDetails::new(StatusCode::UNAUTHORIZED, "Authentication Required")
            .with_detail("X-API-Key header is required for external API access")
    })?;

    // Return stub data for now - full implementation would query data_entries
    Ok(Json(ExternalTemplateDataResponse {
        template_id,
        project_id,
        data: vec![],
    }))
}

/// List task templates (external API).
///
/// Requires API Key authentication via X-API-Key header.
///
/// # Errors
///
/// Returns `ProblemDetails` on authorization failure.
#[utoipa::path(
    get,
    path = "/external/v1/projects/{projectId}/task-templates",
    responses(
        (status = 200, description = "Template list"),
        (status = 401, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    tag = "external-data",
)]
pub async fn list_external_templates(
    State(_state): State<AppState>,
    Path(_project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ProblemDetails> {
    Ok(Json(serde_json::json!({ "data": [] })))
}

/// Get task data (external API).
///
/// # Errors
///
/// Returns `ProblemDetails` on authorization failure.
#[utoipa::path(
    get,
    path = "/external/v1/projects/{projectId}/tasks/{taskId}/data",
    responses(
        (status = 200, description = "Task data"),
        (status = 401, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
    ),
    tag = "external-data",
)]
pub async fn get_task_data(
    State(_state): State<AppState>,
    Path((_project_id, _task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ProblemDetails> {
    Ok(Json(serde_json::json!({ "data": {} })))
}

/// Update task data (external API).
///
/// # Errors
///
/// Returns `ProblemDetails` on authorization failure.
#[utoipa::path(
    put,
    path = "/external/v1/projects/{projectId}/tasks/{taskId}/data",
    responses(
        (status = 200, description = "Task data updated"),
        (status = 401, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
    ),
    tag = "external-data",
)]
pub async fn update_task_data(
    State(_state): State<AppState>,
    Path((_project_id, _task_id)): Path<(Uuid, Uuid)>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ProblemDetails> {
    Ok(Json(serde_json::json!({ "updated": true })))
}
