use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::core::webhook::models::UpdateWebhookParams;

// ── Request/Response Types ──────────────────────────────────

/// Create webhook request body.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub event_types: Vec<String>,
}

/// Update webhook request body.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Webhook response item.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub url: String,
    pub has_secret: bool,
    pub event_types: serde_json::Value,
    pub enabled: bool,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

/// List webhooks response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWebhooksResponse {
    pub data: Vec<WebhookResponse>,
}

/// Webhook event log response item.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookLogResponse {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<i32>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

/// List webhook logs response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWebhookLogsResponse {
    pub data: Vec<WebhookLogResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Query parameters for listing webhook logs.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWebhookLogsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

/// Test webhook response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestWebhookResponse {
    pub event_log_id: Uuid,
    pub status: String,
}

// ── Handlers ────────────────────────────────────────────────

/// List webhooks for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/webhooks",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "Webhook list", body = ListWebhooksResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ListWebhooksResponse>, ProblemDetails> {
    let webhooks = state
        .webhook_service
        .list_webhooks(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    let data = webhooks.into_iter().map(webhook_to_response).collect();
    Ok(Json(ListWebhooksResponse { data }))
}

/// Create a new webhook.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/webhooks",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created", body = WebhookResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), ProblemDetails> {
    let webhook = state
        .webhook_service
        .register_webhook(
            project_id,
            body.name,
            body.url,
            body.secret,
            body.event_types,
            user.account_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(webhook_to_response(webhook))))
}

/// Get a webhook by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/webhooks/{webhookId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("webhookId" = Uuid, Path, description = "Webhook ID"),
    ),
    responses(
        (status = 200, description = "Webhook details", body = WebhookResponse),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn get_webhook(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WebhookResponse>, ProblemDetails> {
    let webhook = state
        .webhook_service
        .get_webhook(webhook_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(webhook_to_response(webhook)))
}

/// Update a webhook.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or not found.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/webhooks/{webhookId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("webhookId" = Uuid, Path, description = "Webhook ID"),
    ),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Webhook updated", body = WebhookResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn update_webhook(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, webhook_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateWebhookRequest>,
) -> Result<Json<WebhookResponse>, ProblemDetails> {
    let params = UpdateWebhookParams {
        name: body.name,
        url: body.url,
        secret: body.secret,
        events: body.event_types.map(|et| serde_json::json!(et)),
        enabled: body.enabled,
    };

    let webhook = state
        .webhook_service
        .update_webhook(webhook_id, params)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(webhook_to_response(webhook)))
}

/// Delete a webhook.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/webhooks/{webhookId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("webhookId" = Uuid, Path, description = "Webhook ID"),
    ),
    responses(
        (status = 204, description = "Webhook deleted"),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn delete_webhook(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .webhook_service
        .delete_webhook(webhook_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Send a test event to a webhook.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/webhooks/{webhookId}/test",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("webhookId" = Uuid, Path, description = "Webhook ID"),
    ),
    responses(
        (status = 200, description = "Test event sent", body = TestWebhookResponse),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn test_webhook(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TestWebhookResponse>, ProblemDetails> {
    let event_log = state
        .webhook_service
        .send_test_event(webhook_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(TestWebhookResponse {
        event_log_id: event_log.id,
        status: event_log.status,
    }))
}

/// List event logs for a webhook.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/webhooks/{webhookId}/logs",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("webhookId" = Uuid, Path, description = "Webhook ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Max items to return"),
    ),
    responses(
        (status = 200, description = "Webhook event logs", body = ListWebhookLogsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "webhooks",
)]
pub async fn list_webhook_logs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, webhook_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ListWebhookLogsQuery>,
) -> Result<Json<ListWebhookLogsResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(50).min(100);

    let logs = state
        .webhook_service
        .list_event_logs(webhook_id, query.cursor, limit + 1)
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

    let data = items.into_iter().map(event_log_to_response).collect();

    Ok(Json(ListWebhookLogsResponse { data, next_cursor }))
}

// ── Helpers ─────────────────────────────────────────────────

fn webhook_to_response(w: crate::modules::core::webhook::models::Webhook) -> WebhookResponse {
    WebhookResponse {
        id: w.id,
        project_id: w.project_id,
        name: w.name,
        url: w.url,
        has_secret: w.secret.is_some(),
        event_types: w.events,
        enabled: w.enabled,
        created_by: w.created_by,
        created_at: w.created_at.to_rfc3339(),
        updated_at: w.updated_at.to_rfc3339(),
    }
}

fn event_log_to_response(
    l: crate::modules::core::webhook::models::WebhookEventLog,
) -> WebhookLogResponse {
    WebhookLogResponse {
        id: l.id,
        webhook_id: l.webhook_id,
        event_type: l.event_type,
        payload: l.payload,
        status: l.status,
        response_status: l.response_status,
        attempts: l.attempts,
        max_attempts: l.max_attempts,
        created_at: l.created_at.to_rfc3339(),
        completed_at: l.completed_at.map(|t| t.to_rfc3339()),
    }
}
