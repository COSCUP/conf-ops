use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::extractors::permission::require_permission;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::conversation::models::{Message, MessageAttachment, MessageSourceType};
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;
use crate::modules::core::task::repository::TaskRepository;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub content: serde_json::Value,
    pub last_seen_message_id: Option<Uuid>,
    pub attachments: Option<Vec<MessageAttachment>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetConversationQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
    pub source_type: Option<MessageSourceType>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLastSeenRequest {
    pub message_id: Uuid,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub source_type: MessageSourceType,
    pub source_id: Option<Uuid>,
    pub content: serde_json::Value,
    pub attachments: Option<serde_json::Value>,
    pub action_result: Option<serde_json::Value>,
    pub last_seen_message_id: Option<Uuid>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    pub has_more: bool,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConversationResponse {
    pub messages: Vec<MessageResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LastSeenResponse {
    pub last_read_message_id: Option<Uuid>,
}

fn message_to_response(msg: &Message) -> MessageResponse {
    MessageResponse {
        id: msg.id,
        task_id: msg.task_id,
        source_type: msg.source_type.clone(),
        source_id: msg.source_id,
        content: msg.content.clone(),
        attachments: msg.attachments.clone(),
        action_result: msg.action_result.clone(),
        last_seen_message_id: msg.last_seen_message_id,
        created_at: msg.created_at.to_rfc3339(),
    }
}

/// Helper to resolve `project_id` from a `task_id`, then get `org_id`.
async fn resolve_project_org(
    state: &AppState,
    task_id: Uuid,
) -> Result<(Uuid, Uuid), ProblemDetails> {
    let task = TaskRepository::get_by_id(&state.pool, task_id)
        .await
        .map_err(ProblemDetails::from)?;
    let org_id = ProjectRepository::get_organization_id(&state.pool, task.project_id)
        .await
        .map_err(ProblemDetails::from)?;
    Ok((task.project_id, org_id))
}

// ── Handlers ──────────────────────────────────────────────────

/// Get conversation messages with cursor-based pagination.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure or not found.
pub async fn get_conversation(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetConversationQuery>,
) -> Result<Json<ConversationResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let (messages, has_more) = state
        .conversation_service
        .get_conversation(task_id, query.cursor, limit, query.source_type.as_ref())
        .await
        .map_err(ProblemDetails::from)?;

    let next_cursor = if has_more {
        messages.last().map(|m| m.id)
    } else {
        None
    };

    Ok(Json(ConversationResponse {
        messages: messages.iter().map(message_to_response).collect(),
        pagination: PaginationInfo {
            has_more,
            next_cursor,
        },
    }))
}

/// Send a message in a task conversation.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure, stale conversation, or validation error.
pub async fn send_message(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    // Resolve source_id from the authenticated user's member record
    let member =
        crate::modules::core::member::repository::MemberRepository::get_by_project_and_account(
            &state.pool,
            project_id,
            user.account_id,
        )
        .await
        .map_err(|_| ProblemDetails::internal_server_error())?;

    let source_id = member.map(|m| m.id);

    let message = state
        .conversation_service
        .send_message(
            task_id,
            MessageSourceType::Member,
            source_id,
            body.content,
            body.attachments,
            body.last_seen_message_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(message_to_response(&message))))
}

/// Update last-seen position for the current user.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
pub async fn update_last_seen(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateLastSeenRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    state
        .conversation_service
        .update_last_seen(task_id, user.account_id, body.message_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get last-seen position for the current user.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
pub async fn get_last_seen(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<LastSeenResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    let last_read_message_id = state
        .conversation_service
        .get_last_seen(task_id, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(LastSeenResponse {
        last_read_message_id,
    }))
}

// ── Last Seen Position (CRDT y_clock) ────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLastSeenPositionRequest {
    pub y_clock: i64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LastSeenPositionResponse {
    pub y_clock: Option<i64>,
}

/// Update CRDT last-seen position (`y_clock`) for the current user.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
pub async fn update_last_seen_position(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateLastSeenPositionRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    state
        .conversation_service
        .update_last_seen_position(task_id, user.account_id, body.y_clock)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get CRDT last-seen position (`y_clock`) for the current user.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
pub async fn get_last_seen_position(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<LastSeenPositionResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    let y_clock = state
        .conversation_service
        .get_last_seen_position(task_id, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(LastSeenPositionResponse { y_clock }))
}
