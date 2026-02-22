use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;

// ── Request/Response Types ──────────────────────────────────

/// Audit log response item.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub actor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub created_at: String,
}

/// List audit logs response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsResponse {
    pub data: Vec<AuditLogResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Query parameters for listing audit logs.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
    pub actor_type: Option<String>,
    pub action: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────

/// List audit logs for an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/audit-logs",
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Max items to return"),
        ("actorType" = Option<String>, Query, description = "Filter by actor type"),
        ("action" = Option<String>, Query, description = "Filter by action"),
    ),
    responses(
        (status = 200, description = "Organization audit logs", body = ListAuditLogsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "audit",
)]
pub async fn list_org_audit_logs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<ListAuditLogsResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(50).min(100);

    let logs = state
        .audit_service
        .query_org_logs(org_id, query.cursor, limit + 1)
        .await
        .map_err(ProblemDetails::from)?;

    let has_more = i64::try_from(logs.len()).unwrap_or(0) > limit;
    let items: Vec<_> = logs
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    let next_cursor = if has_more {
        items.last().map(|l| l.id.to_string())
    } else {
        None
    };

    let data = items.into_iter().map(audit_log_to_response).collect();

    Ok(Json(ListAuditLogsResponse { data, next_cursor }))
}

/// List audit logs for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/audit-logs",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Max items to return"),
        ("actorType" = Option<String>, Query, description = "Filter by actor type"),
        ("action" = Option<String>, Query, description = "Filter by action"),
    ),
    responses(
        (status = 200, description = "Project audit logs", body = ListAuditLogsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "audit",
)]
pub async fn list_project_audit_logs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<ListAuditLogsResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(50).min(100);

    let logs = state
        .audit_service
        .query_project_logs(project_id, query.cursor, limit + 1)
        .await
        .map_err(ProblemDetails::from)?;

    let has_more = i64::try_from(logs.len()).unwrap_or(0) > limit;
    let items: Vec<_> = logs
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    let next_cursor = if has_more {
        items.last().map(|l| l.id.to_string())
    } else {
        None
    };

    let data = items.into_iter().map(audit_log_to_response).collect();

    Ok(Json(ListAuditLogsResponse { data, next_cursor }))
}

// ── Helpers ─────────────────────────────────────────────────

fn audit_log_to_response(l: crate::modules::audit::models::AuditLog) -> AuditLogResponse {
    AuditLogResponse {
        id: l.id,
        actor_type: l.actor_type,
        actor_id: l.actor_id,
        action: l.action,
        resource_type: l.resource_type,
        resource_id: l.resource_id,
        context_type: l.context_type,
        context_id: l.context_id,
        details: l.details,
        created_at: l.created_at.to_rfc3339(),
    }
}
