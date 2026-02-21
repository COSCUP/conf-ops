use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::extractors::permission::require_permission;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;
use crate::modules::core::task::repository::TaskRepository;
use crate::modules::storage::error::StorageError;
use crate::modules::storage::models::FileRecord;
use crate::modules::storage::service::UploadFileParams;

// ── Response Types ───────────────────────────────────────────

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadResponse {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResponse {
    pub removed_count: u64,
}

fn file_to_upload_response(record: &FileRecord) -> FileUploadResponse {
    FileUploadResponse {
        id: record.id,
        filename: record.filename.clone(),
        mime_type: record.mime_type.clone(),
        size: record.file_size,
        created_at: record.created_at.to_rfc3339(),
    }
}

// ── Route builder ────────────────────────────────────────────

pub fn file_routes() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/cleanup", post(trigger_cleanup))
        .route("/{fileId}", get(download_file).delete(delete_file))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
}

// ── Helpers ──────────────────────────────────────────────────

/// Resolved scope information for permission checking and FK fields.
struct ResolvedScope {
    org: Uuid,
    project: Uuid,
    task: Option<Uuid>,
}

/// Resolve scope to `project_id` / `org_id` and check permission.
async fn resolve_and_check_scope(
    state: &AppState,
    account_id: Uuid,
    scope_type: &str,
    scope_id: Uuid,
    action: Action,
) -> Result<ResolvedScope, ProblemDetails> {
    let (project_id, task_id) = match scope_type {
        "task" => {
            let task = TaskRepository::get_by_id(&state.pool, scope_id)
                .await
                .map_err(|_| {
                    ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found")
                        .with_detail("Task not found")
                })?;
            (task.project_id, Some(scope_id))
        }
        "project" => (scope_id, None),
        _ => {
            return Err(
                ProblemDetails::new(StatusCode::BAD_REQUEST, "Invalid Scope Type")
                    .with_detail(format!("Invalid scope type: {scope_type}")),
            );
        }
    };

    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        state,
        account_id,
        Resource::ProjectScoped { org_id, project_id },
        action,
    )
    .await?;

    Ok(ResolvedScope {
        org: org_id,
        project: project_id,
        task: task_id,
    })
}

// ── Handlers ─────────────────────────────────────────────────

/// Upload a file via multipart form.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation, permission, or I/O failure.
#[utoipa::path(
    post,
    path = "/api/v1/files/upload",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, body = FileUploadResponse),
        (status = 400, body = ProblemDetails),
        (status = 413, body = ProblemDetails),
    ),
    tag = "files",
)]
pub async fn upload_file(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<FileUploadResponse>), ProblemDetails> {
    let mut file_data: Option<(String, String, Vec<u8>)> = None;
    let mut scope_type: Option<String> = None;
    let mut scope_id: Option<Uuid> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ProblemDetails::new(StatusCode::BAD_REQUEST, "Invalid Multipart").with_detail(e.to_string())
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("unnamed").to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let data = field.bytes().await.map_err(|e| {
                    ProblemDetails::new(StatusCode::BAD_REQUEST, "Read Error")
                        .with_detail(e.to_string())
                })?;
                file_data = Some((filename, content_type, data.to_vec()));
            }
            "scopeType" => {
                let text = field.text().await.map_err(|e| {
                    ProblemDetails::new(StatusCode::BAD_REQUEST, "Read Error")
                        .with_detail(e.to_string())
                })?;
                scope_type = Some(text);
            }
            "scopeId" => {
                let text = field.text().await.map_err(|e| {
                    ProblemDetails::new(StatusCode::BAD_REQUEST, "Read Error")
                        .with_detail(e.to_string())
                })?;
                let id = Uuid::parse_str(&text).map_err(|_| {
                    ProblemDetails::new(StatusCode::BAD_REQUEST, "Invalid UUID")
                        .with_detail("scopeId must be a valid UUID")
                })?;
                scope_id = Some(id);
            }
            _ => {}
        }
    }

    let (filename, content_type, data) = file_data.ok_or_else(|| StorageError::MissingFile)?;
    let scope_type =
        scope_type.ok_or_else(|| StorageError::MissingField("scopeType".to_string()))?;
    let scope_id = scope_id.ok_or_else(|| StorageError::MissingField("scopeId".to_string()))?;

    // Permission check & resolve scope FKs
    let resolved = resolve_and_check_scope(
        &state,
        user.account_id,
        &scope_type,
        scope_id,
        Action::ViewTasks,
    )
    .await?;

    let record = state
        .file_service
        .upload_file(&UploadFileParams {
            filename: &filename,
            mime_type: &content_type,
            data: &data,
            scope_type: &scope_type,
            scope_id,
            uploaded_by: user.account_id,
            organization_id: Some(resolved.org),
            project_id: Some(resolved.project),
            task_id: resolved.task,
        })
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(file_to_upload_response(&record))))
}

/// Download a file by ID (streaming response).
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/files/{fileId}",
    responses(
        (status = 200, description = "File content"),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("fileId" = Uuid, Path,),
    ),
    tag = "files",
)]
pub async fn download_file(
    State(state): State<AppState>,
    user: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, ProblemDetails> {
    let (record, reader) = state
        .file_service
        .download_file(file_id)
        .await
        .map_err(ProblemDetails::from)?;

    // Permission check based on scope
    resolve_and_check_scope(
        &state,
        user.account_id,
        &record.scope_type,
        record.scope_id,
        Action::ViewTasks,
    )
    .await?;

    let body = Body::from_stream(ReaderStream::new(reader));

    let mut headers = HeaderMap::new();
    if let Ok(val) = record.mime_type.parse() {
        headers.insert(header::CONTENT_TYPE, val);
    }
    if let Ok(val) = format!("attachment; filename=\"{}\"", record.filename).parse() {
        headers.insert(header::CONTENT_DISPOSITION, val);
    }

    Ok((headers, body))
}

/// Trigger cleanup of orphaned files past the grace period (admin only).
///
/// # Errors
///
/// Returns `ProblemDetails` on I/O or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/files/cleanup",
    responses(
        (status = 200, body = CleanupResponse),
    ),
    tag = "files",
)]
pub async fn trigger_cleanup(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<CleanupResponse>, ProblemDetails> {
    let removed_count = state
        .file_service
        .cleanup_orphaned_files()
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(CleanupResponse { removed_count }))
}

/// Delete a file (soft-delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    delete,
    path = "/api/v1/files/{fileId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("fileId" = Uuid, Path,),
    ),
    tag = "files",
)]
pub async fn delete_file(
    State(state): State<AppState>,
    user: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    let record = state
        .file_service
        .get_file(file_id)
        .await
        .map_err(ProblemDetails::from)?;

    // Permission: uploader can delete, or ManageDataEntries permission
    if record.uploaded_by != user.account_id {
        resolve_and_check_scope(
            &state,
            user.account_id,
            &record.scope_type,
            record.scope_id,
            Action::ManageDataEntries,
        )
        .await?;
    }

    state
        .file_service
        .delete_file(file_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}
