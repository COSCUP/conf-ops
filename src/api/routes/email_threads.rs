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
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;
use crate::modules::core::task::repository::TaskRepository;
use crate::modules::email::models::{EmailMessage, EmailThread};
use crate::modules::email::repository::{EmailMessageRepository, EmailThreadRepository};
use crate::modules::email::service::SendEmailParams;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmailThreadRequest {
    pub subject: String,
    pub participants: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmailThreadRequest {
    pub subject: Option<String>,
    pub participants: Option<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendThreadEmailRequest {
    pub to_addresses: Vec<String>,
    pub cc_addresses: Option<Vec<String>>,
    pub subject: Option<String>,
    pub html_body: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListEmailThreadsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailThreadResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub subject: String,
    pub participants: serde_json::Value,
    pub message_ids: serde_json::Value,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailMessageResponse {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub from_address: String,
    pub to_addresses: serde_json::Value,
    pub cc_addresses: serde_json::Value,
    pub subject: String,
    pub direction: String,
    pub send_status: String,
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
pub struct EmailThreadListResponse {
    pub threads: Vec<EmailThreadResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailMessageListResponse {
    pub messages: Vec<EmailMessageResponse>,
    pub pagination: PaginationInfo,
}

fn thread_to_response(thread: &EmailThread) -> EmailThreadResponse {
    EmailThreadResponse {
        id: thread.id,
        task_id: thread.task_id,
        subject: thread.subject.clone(),
        participants: thread.participants.clone(),
        message_ids: thread.message_ids.clone(),
        last_message_at: thread.last_message_at.map(|t| t.to_rfc3339()),
        created_at: thread.created_at.to_rfc3339(),
        updated_at: thread.updated_at.to_rfc3339(),
    }
}

fn message_to_response(msg: &EmailMessage) -> EmailMessageResponse {
    EmailMessageResponse {
        id: msg.id,
        thread_id: msg.thread_id,
        message_id: msg.message_id.clone(),
        in_reply_to: msg.in_reply_to.clone(),
        from_address: msg.from_address.clone(),
        to_addresses: msg.to_addresses.clone(),
        cc_addresses: msg.cc_addresses.clone(),
        subject: msg.subject.clone(),
        direction: msg.direction.clone(),
        send_status: msg.send_status.clone(),
        created_at: msg.created_at.to_rfc3339(),
    }
}

async fn resolve_task_project_org(
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

/// List email threads for a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure or not found.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads",
    responses(
        (status = 200, body = EmailThreadListResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("cursor" = Option<Uuid>, Query,),
        ("limit" = Option<i64>, Query,),
    ),
    tag = "email-threads",
)]
pub async fn list_email_threads(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ListEmailThreadsQuery>,
) -> Result<Json<EmailThreadListResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let limit_usize = usize::try_from(limit).unwrap_or(50);
    let mut threads =
        EmailThreadRepository::list_by_task(&state.pool, task_id, query.cursor, limit + 1)
            .await
            .map_err(ProblemDetails::from)?;

    let has_more = threads.len() > limit_usize;
    if has_more {
        threads.truncate(limit_usize);
    }

    let next_cursor = if has_more {
        threads.last().map(|t| t.id)
    } else {
        None
    };

    Ok(Json(EmailThreadListResponse {
        threads: threads.iter().map(thread_to_response).collect(),
        pagination: PaginationInfo {
            has_more,
            next_cursor,
        },
    }))
}

/// Create an email thread for a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads",
    request_body = CreateEmailThreadRequest,
    responses(
        (status = 201, body = EmailThreadResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "email-threads",
)]
pub async fn create_email_thread(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateEmailThreadRequest>,
) -> Result<(StatusCode, Json<EmailThreadResponse>), ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    let thread_id = crate::id::generate_id();
    let thread = EmailThreadRepository::create(
        &state.pool,
        &crate::modules::email::models::CreateEmailThreadParams {
            id: thread_id,
            task_id,
            subject: body.subject,
            participants: serde_json::json!(body.participants),
        },
    )
    .await
    .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(thread_to_response(&thread))))
}

/// Update an email thread.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure or not found.
#[utoipa::path(
    patch,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}",
    request_body = UpdateEmailThreadRequest,
    responses(
        (status = 200, body = EmailThreadResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("threadId" = Uuid, Path,),
    ),
    tag = "email-threads",
)]
pub async fn update_email_thread(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id, thread_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateEmailThreadRequest>,
) -> Result<Json<EmailThreadResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    let thread = EmailThreadRepository::update(
        &state.pool,
        thread_id,
        &crate::modules::email::models::UpdateEmailThreadParams {
            subject: body.subject,
            participants: body.participants.map(|p| serde_json::json!(p)),
        },
    )
    .await
    .map_err(ProblemDetails::from)?;

    Ok(Json(thread_to_response(&thread)))
}

/// Delete an email thread.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure or not found.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("threadId" = Uuid, Path,),
    ),
    tag = "email-threads",
)]
pub async fn delete_email_thread(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id, thread_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    EmailThreadRepository::delete(&state.pool, thread_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List messages in an email thread.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure or not found.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages",
    responses(
        (status = 200, body = EmailMessageListResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("threadId" = Uuid, Path,),
        ("cursor" = Option<Uuid>, Query,),
        ("limit" = Option<i64>, Query,),
    ),
    tag = "email-threads",
)]
pub async fn list_thread_messages(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id, thread_id)): Path<(Uuid, Uuid, Uuid)>,
    Query(_query): Query<ListEmailThreadsQuery>,
) -> Result<Json<EmailMessageListResponse>, ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewConversation,
    )
    .await?;

    let messages = EmailMessageRepository::list_by_thread(&state.pool, thread_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(EmailMessageListResponse {
        messages: messages.iter().map(message_to_response).collect(),
        pagination: PaginationInfo {
            has_more: false,
            next_cursor: None,
        },
    }))
}

/// Send an email in a thread.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure, SMTP error, or not found.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages",
    request_body = SendThreadEmailRequest,
    responses(
        (status = 201, body = EmailMessageResponse),
        (status = 404, body = ProblemDetails),
        (status = 502, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("threadId" = Uuid, Path,),
    ),
    tag = "email-threads",
)]
pub async fn send_thread_email(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id, thread_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<SendThreadEmailRequest>,
) -> Result<(StatusCode, Json<EmailMessageResponse>), ProblemDetails> {
    let (project_id, org_id) = resolve_task_project_org(&state, task_id).await?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    // Get thread to use its subject if not provided
    let thread = EmailThreadRepository::get_by_id(&state.pool, thread_id)
        .await
        .map_err(ProblemDetails::from)?;

    let subject = body
        .subject
        .unwrap_or_else(|| format!("Re: {}", thread.subject));

    // Resolve the sender's email from their account
    let account = crate::modules::auth::repository::AccountRepository::get_by_id(
        &state.pool,
        user.account_id,
    )
    .await
    .map_err(ProblemDetails::from)?;

    let email_msg = state
        .email_outbound_service
        .send_email(SendEmailParams {
            thread_id: Some(thread_id),
            task_id,
            from_address: account.email,
            to_addresses: body.to_addresses,
            cc_addresses: body.cc_addresses.unwrap_or_default(),
            subject,
            html_body: body.html_body,
        })
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(message_to_response(&email_msg))))
}
