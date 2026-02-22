use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;

// ── Request/Response Types ──────────────────────────────────

/// A notification item in the API response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
    pub is_read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<String>,
    pub delivered_channels: serde_json::Value,
    pub created_at: String,
}

/// Response for listing notifications.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsResponse {
    pub data: Vec<NotificationResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Response for unread count.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    pub count: i64,
}

/// Query parameters for listing notifications.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
    pub unread_only: Option<bool>,
}

/// Web Push subscription request body.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscribeRequest {
    pub subscription: WebPushSubscriptionData,
    pub device_name: Option<String>,
}

/// Web Push subscription data.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionData {
    pub endpoint: String,
    pub keys: WebPushKeys,
}

/// Web Push subscription keys.
#[derive(Deserialize, ToSchema)]
pub struct WebPushKeys {
    pub p256dh: String,
    pub auth: String,
}

/// Web Push subscription response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebPushSubscriptionResponse {
    pub id: Uuid,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    pub created_at: String,
}

/// Mark all as read response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkAllReadResponse {
    pub updated: i64,
}

// ── Handlers ────────────────────────────────────────────────

/// List notifications for the current user.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    params(
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Max items to return"),
        ("unreadOnly" = Option<bool>, Query, description = "Filter unread only"),
    ),
    responses(
        (status = 200, description = "Notification list", body = ListNotificationsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn list_notifications(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<ListNotificationsResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(50).min(100);
    let unread_only = query.unread_only.unwrap_or(false);

    let notifications = state
        .notification_service
        .list_notifications(user.account_id, query.cursor, limit + 1, unread_only)
        .await
        .map_err(ProblemDetails::from)?;

    let has_more = i64::try_from(notifications.len()).unwrap_or(0) > limit;
    let items: Vec<_> = notifications
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    let next_cursor = if has_more {
        items.last().map(|n| n.id.to_string())
    } else {
        None
    };

    let data = items.into_iter().map(notification_to_response).collect();

    Ok(Json(ListNotificationsResponse { data, next_cursor }))
}

/// Get unread notification count.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread-count",
    responses(
        (status = 200, description = "Unread count", body = UnreadCountResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn get_unread_count(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<UnreadCountResponse>, ProblemDetails> {
    let count = state
        .notification_service
        .get_unread_count(user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(UnreadCountResponse { count }))
}

/// Mark a notification as read.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    put,
    path = "/api/v1/notifications/{notificationId}/read",
    params(
        ("notificationId" = Uuid, Path, description = "Notification ID"),
    ),
    responses(
        (status = 204, description = "Notification marked as read"),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn mark_as_read(
    State(state): State<AppState>,
    user: AuthUser,
    Path(notification_id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .notification_service
        .mark_as_read(user.account_id, notification_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Mark all notifications as read.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    put,
    path = "/api/v1/notifications/read-all",
    responses(
        (status = 200, description = "All notifications marked as read", body = MarkAllReadResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn mark_all_as_read(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<MarkAllReadResponse>, ProblemDetails> {
    let updated = state
        .notification_service
        .mark_all_as_read(user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(MarkAllReadResponse { updated }))
}

/// Register a Web Push subscription.
///
/// # Errors
///
/// Returns `ProblemDetails` on duplicate or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/notifications/web-push/subscribe",
    request_body = WebPushSubscribeRequest,
    responses(
        (status = 201, description = "Subscription created", body = WebPushSubscriptionResponse),
        (status = 401, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn subscribe_web_push(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<WebPushSubscribeRequest>,
) -> Result<(StatusCode, Json<WebPushSubscriptionResponse>), ProblemDetails> {
    let sub = state
        .notification_service
        .subscribe_web_push(
            user.account_id,
            body.subscription.endpoint,
            body.subscription.keys.p256dh,
            body.subscription.keys.auth,
            body.device_name,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(WebPushSubscriptionResponse {
            id: sub.id,
            endpoint: sub.endpoint,
            device_name: sub.device_name,
            created_at: sub.created_at.to_rfc3339(),
        }),
    ))
}

/// Unsubscribe a Web Push endpoint.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    delete,
    path = "/api/v1/notifications/web-push/subscriptions/{endpoint}",
    params(
        ("endpoint" = String, Path, description = "Web Push endpoint URL (URL-encoded)"),
    ),
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "notifications",
)]
pub async fn unsubscribe_web_push(
    State(state): State<AppState>,
    user: AuthUser,
    Path(endpoint): Path<String>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .notification_service
        .unsubscribe_web_push(user.account_id, &endpoint)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────

fn notification_to_response(
    n: crate::modules::notifications::models::Notification,
) -> NotificationResponse {
    NotificationResponse {
        id: n.id,
        account_id: n.account_id,
        notification_type: n.notification_type,
        title: n.title,
        body: n.body,
        reference_type: n.reference_type,
        reference_id: n.reference_id,
        project_id: n.project_id,
        is_read: n.is_read,
        read_at: n.read_at.map(|t| t.to_rfc3339()),
        delivered_channels: n.delivered_channels,
        created_at: n.created_at.to_rfc3339(),
    }
}
