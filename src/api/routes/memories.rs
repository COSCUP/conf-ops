use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::ai::memory::models::{
    CreateLibraryDocumentParams, CreateMemoryParams, LibraryDocument, LibraryDocumentVersion,
    Memory, MemorySource, MemoryVersion, ScopeType, UpdateLibraryDocumentParams,
    UpdateMemoryParams,
};

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryRequest {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub content: String,
    pub library_ref: Option<Uuid>,
    pub source: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryRequest {
    pub content: String,
    pub library_ref: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListMemoriesQuery {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemoryResponse {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub content: String,
    pub library_ref: Option<Uuid>,
    pub source: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemoryListResponse {
    pub items: Vec<MemoryResponse>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemoryVersionResponse {
    pub id: Uuid,
    pub memory_id: Uuid,
    pub content: String,
    pub changed_by: Uuid,
    pub created_at: DateTime<Utc>,
}

// ── Library Document Request/Response Types ──────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryDocumentRequest {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub title: String,
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLibraryDocumentRequest {
    pub title: Option<String>,
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListLibraryDocumentsQuery {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocumentResponse {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocumentListResponse {
    pub items: Vec<LibraryDocumentResponse>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocumentVersionResponse {
    pub id: Uuid,
    pub document_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub changed_by: Uuid,
    pub created_at: DateTime<Utc>,
}

// ── Converters ───────────────────────────────────────────────

fn memory_to_response(m: &Memory) -> MemoryResponse {
    MemoryResponse {
        id: m.id,
        scope_type: m.scope_type.clone(),
        scope_id: m.scope_id,
        content: m.content.clone(),
        library_ref: m.library_ref,
        source: m.source.clone(),
        created_by: m.created_by,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn version_to_response(v: &MemoryVersion) -> MemoryVersionResponse {
    MemoryVersionResponse {
        id: v.id,
        memory_id: v.memory_id,
        content: v.content.clone(),
        changed_by: v.changed_by,
        created_at: v.created_at,
    }
}

fn doc_to_response(d: &LibraryDocument) -> LibraryDocumentResponse {
    LibraryDocumentResponse {
        id: d.id,
        scope_type: d.scope_type.clone(),
        scope_id: d.scope_id,
        title: d.title.clone(),
        content: d.content.clone(),
        created_by: d.created_by,
        created_at: d.created_at,
        updated_at: d.updated_at,
    }
}

fn doc_version_to_response(v: &LibraryDocumentVersion) -> LibraryDocumentVersionResponse {
    LibraryDocumentVersionResponse {
        id: v.id,
        document_id: v.document_id,
        title: v.title.clone(),
        content: v.content.clone(),
        changed_by: v.changed_by,
        created_at: v.created_at,
    }
}

// ── Memory Handlers ─────────────────────────────────────────

/// List memories by scope.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid scope or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/memories",
    params(
        ("scopeType" = String, Query, description = "Scope type"),
        ("scopeId" = Uuid, Query, description = "Scope ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Number of items per page"),
    ),
    responses(
        (status = 200, description = "List of memories", body = MemoryListResponse),
    ),
    tag = "memories"
)]
pub async fn list_memories(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListMemoriesQuery>,
) -> Result<Json<MemoryListResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let limit_usize = usize::try_from(limit).unwrap_or(20);
    let mut memories = state
        .memory_service
        .list_memories(&query.scope_type, query.scope_id, query.cursor, limit + 1)
        .await
        .map_err(ProblemDetails::from)?;

    let has_more = memories.len() > limit_usize;
    if has_more {
        memories.truncate(limit_usize);
    }

    let next_cursor = if has_more {
        memories.last().map(|m| m.id)
    } else {
        None
    };

    let items = memories.iter().map(memory_to_response).collect();
    Ok(Json(MemoryListResponse { items, next_cursor }))
}

/// Create a new memory.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid scope, source, or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/memories",
    request_body = CreateMemoryRequest,
    responses(
        (status = 201, description = "Memory created", body = MemoryResponse),
    ),
    tag = "memories"
)]
pub async fn create_memory(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<MemoryResponse>), ProblemDetails> {
    let scope_type: ScopeType = body.scope_type.parse().map_err(|_| {
        ProblemDetails::new(StatusCode::BAD_REQUEST, "Bad Request")
            .with_detail(format!("Invalid scope type: {}", body.scope_type))
    })?;
    let source: MemorySource = body.source.parse().map_err(|_| {
        ProblemDetails::new(StatusCode::BAD_REQUEST, "Bad Request")
            .with_detail(format!("Invalid memory source: {}", body.source))
    })?;

    let params = CreateMemoryParams {
        id: Uuid::now_v7(),
        scope_type,
        scope_id: body.scope_id,
        content: body.content,
        library_ref: body.library_ref,
        source,
        created_by: user.account_id,
    };

    let memory = state
        .memory_service
        .create_memory(&params)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(memory_to_response(&memory))))
}

/// Get a memory by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/memories/{memoryId}",
    params(("memoryId" = Uuid, Path, description = "Memory ID")),
    responses(
        (status = 200, description = "Memory details", body = MemoryResponse),
    ),
    tag = "memories"
)]
pub async fn get_memory(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(memory_id): Path<Uuid>,
) -> Result<Json<MemoryResponse>, ProblemDetails> {
    let memory = state
        .memory_service
        .get_memory(memory_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(memory_to_response(&memory)))
}

/// Update a memory.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    put,
    path = "/api/v1/memories/{memoryId}",
    params(("memoryId" = Uuid, Path, description = "Memory ID")),
    request_body = UpdateMemoryRequest,
    responses(
        (status = 200, description = "Memory updated", body = MemoryResponse),
    ),
    tag = "memories"
)]
pub async fn update_memory(
    State(state): State<AppState>,
    user: AuthUser,
    Path(memory_id): Path<Uuid>,
    Json(body): Json<UpdateMemoryRequest>,
) -> Result<Json<MemoryResponse>, ProblemDetails> {
    let params = UpdateMemoryParams {
        content: body.content,
        library_ref: body.library_ref,
    };

    let memory = state
        .memory_service
        .update_memory(memory_id, &params, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(memory_to_response(&memory)))
}

/// Delete a memory.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    delete,
    path = "/api/v1/memories/{memoryId}",
    params(("memoryId" = Uuid, Path, description = "Memory ID")),
    responses(
        (status = 204, description = "Memory deleted"),
    ),
    tag = "memories"
)]
pub async fn delete_memory(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(memory_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .memory_service
        .delete_memory(memory_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List version history for a memory.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/memories/{memoryId}/versions",
    params(("memoryId" = Uuid, Path, description = "Memory ID")),
    responses(
        (status = 200, description = "Memory version history", body = Vec<MemoryVersionResponse>),
    ),
    tag = "memories"
)]
pub async fn list_memory_versions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(memory_id): Path<Uuid>,
) -> Result<Json<Vec<MemoryVersionResponse>>, ProblemDetails> {
    let versions = state
        .memory_service
        .list_memory_versions(memory_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(versions.iter().map(version_to_response).collect()))
}

// ── Library Document Handlers ───────────────────────────────

/// List library documents by scope.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid scope or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/library-documents",
    params(
        ("scopeType" = String, Query, description = "Scope type"),
        ("scopeId" = Uuid, Query, description = "Scope ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Number of items per page"),
    ),
    responses(
        (status = 200, description = "List of library documents", body = LibraryDocumentListResponse),
    ),
    tag = "library-documents"
)]
pub async fn list_library_documents(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListLibraryDocumentsQuery>,
) -> Result<Json<LibraryDocumentListResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let limit_usize = usize::try_from(limit).unwrap_or(20);
    let mut docs = state
        .memory_service
        .list_library_documents(&query.scope_type, query.scope_id, query.cursor, limit + 1)
        .await
        .map_err(ProblemDetails::from)?;

    let has_more = docs.len() > limit_usize;
    if has_more {
        docs.truncate(limit_usize);
    }

    let next_cursor = if has_more {
        docs.last().map(|d| d.id)
    } else {
        None
    };

    let items = docs.iter().map(doc_to_response).collect();
    Ok(Json(LibraryDocumentListResponse { items, next_cursor }))
}

/// Create a new library document.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid scope or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/library-documents",
    request_body = CreateLibraryDocumentRequest,
    responses(
        (status = 201, description = "Library document created", body = LibraryDocumentResponse),
    ),
    tag = "library-documents"
)]
pub async fn create_library_document(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateLibraryDocumentRequest>,
) -> Result<(StatusCode, Json<LibraryDocumentResponse>), ProblemDetails> {
    let scope_type: ScopeType = body.scope_type.parse().map_err(|_| {
        ProblemDetails::new(StatusCode::BAD_REQUEST, "Bad Request")
            .with_detail(format!("Invalid scope type: {}", body.scope_type))
    })?;

    let params = CreateLibraryDocumentParams {
        id: Uuid::now_v7(),
        scope_type,
        scope_id: body.scope_id,
        title: body.title,
        content: body.content,
        created_by: user.account_id,
    };

    let doc = state
        .memory_service
        .create_library_document(&params)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(doc_to_response(&doc))))
}

/// Get a library document by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/library-documents/{documentId}",
    params(("documentId" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Library document details", body = LibraryDocumentResponse),
    ),
    tag = "library-documents"
)]
pub async fn get_library_document(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<LibraryDocumentResponse>, ProblemDetails> {
    let doc = state
        .memory_service
        .get_library_document(document_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(doc_to_response(&doc)))
}

/// Update a library document.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    put,
    path = "/api/v1/library-documents/{documentId}",
    params(("documentId" = Uuid, Path, description = "Document ID")),
    request_body = UpdateLibraryDocumentRequest,
    responses(
        (status = 200, description = "Library document updated", body = LibraryDocumentResponse),
    ),
    tag = "library-documents"
)]
pub async fn update_library_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(body): Json<UpdateLibraryDocumentRequest>,
) -> Result<Json<LibraryDocumentResponse>, ProblemDetails> {
    let params = UpdateLibraryDocumentParams {
        title: body.title,
        content: body.content,
    };

    let doc = state
        .memory_service
        .update_library_document(document_id, &params, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(doc_to_response(&doc)))
}

/// Delete a library document.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    delete,
    path = "/api/v1/library-documents/{documentId}",
    params(("documentId" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 204, description = "Library document deleted"),
    ),
    tag = "library-documents"
)]
pub async fn delete_library_document(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .memory_service
        .delete_library_document(document_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List version history for a library document.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/library-documents/{documentId}/versions",
    params(("documentId" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document version history", body = Vec<LibraryDocumentVersionResponse>),
    ),
    tag = "library-documents"
)]
pub async fn list_library_document_versions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<LibraryDocumentVersionResponse>>, ProblemDetails> {
    let versions = state
        .memory_service
        .list_library_document_versions(document_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(versions.iter().map(doc_version_to_response).collect()))
}
