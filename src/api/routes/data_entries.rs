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
use crate::modules::core::data_sheet::models::DataEntry;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpsertDataEntryRequest {
    pub values: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataEntryResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub data_schema_id: Uuid,
    pub values: serde_json::Value,
    pub source_links: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataEntryListResponse {
    pub entries: Vec<DataEntryResponse>,
}

fn entry_to_response(entry: &DataEntry) -> DataEntryResponse {
    DataEntryResponse {
        id: entry.id,
        task_id: entry.task_id,
        data_schema_id: entry.data_schema_id,
        values: entry.values.clone(),
        source_links: entry.source_links.clone(),
        created_at: entry.created_at.to_rfc3339(),
        updated_at: entry.updated_at.to_rfc3339(),
    }
}

// ── Handlers ──────────────────────────────────────────────────

/// List data entries for a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/data-entries",
    responses(
        (status = 200, body = DataEntryListResponse),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "data-entries",
)]
pub async fn list_entries(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DataEntryListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewDataEntries,
    )
    .await?;

    let entries = state
        .data_sheet_service
        .list_entries(task_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(DataEntryListResponse {
        entries: entries.iter().map(entry_to_response).collect(),
    }))
}

/// Get or upsert a data entry by schema ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or permission failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/data-entries/{schemaId}",
    request_body = UpsertDataEntryRequest,
    responses(
        (status = 200, body = DataEntryResponse),
        (status = 400, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("schemaId" = Uuid, Path,),
    ),
    tag = "data-entries",
)]
pub async fn upsert_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpsertDataEntryRequest>,
) -> Result<Json<DataEntryResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageDataEntries,
    )
    .await?;

    let entry = state
        .data_sheet_service
        .upsert_entry(task_id, schema_id, &body.values)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(entry_to_response(&entry)))
}

/// Get a data entry by schema ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/data-entries/{schemaId}",
    responses(
        (status = 200, body = DataEntryResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("schemaId" = Uuid, Path,),
    ),
    tag = "data-entries",
)]
pub async fn get_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DataEntryResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewDataEntries,
    )
    .await?;

    let entry = state
        .data_sheet_service
        .get_entry(task_id, schema_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(entry_to_response(&entry)))
}

/// Delete a data entry (soft-delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/data-entries/{schemaId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("schemaId" = Uuid, Path,),
    ),
    tag = "data-entries",
)]
pub async fn delete_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageDataEntries,
    )
    .await?;

    state
        .data_sheet_service
        .delete_entry(task_id, schema_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get aggregated data sheet for a template's schema.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-sheets/{schemaId}",
    responses(
        (status = 200, body = DataEntryListResponse),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("templateId" = Uuid, Path,),
        ("schemaId" = Uuid, Path,),
    ),
    tag = "data-entries",
)]
pub async fn get_aggregated_sheet(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DataEntryListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewDataEntries,
    )
    .await?;

    let entries = state
        .data_sheet_service
        .get_aggregated(schema_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(DataEntryListResponse {
        entries: entries.iter().map(entry_to_response).collect(),
    }))
}
