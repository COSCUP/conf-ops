use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
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
use crate::modules::email::error::EmailError;
use crate::modules::email::models::UnassignedEmail;
use crate::modules::email::repository::UnassignedEmailRepository;

// ── Inbound Source Detection ──────────────────────────────────

enum InboundSource {
    AwsSes {
        spam_verdict: Option<String>,
        virus_verdict: Option<String>,
    },
    Cloudflare,
    Unknown,
}

impl std::fmt::Display for InboundSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsSes { .. } => write!(f, "AWS SES"),
            Self::Cloudflare => write!(f, "Cloudflare"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

fn detect_source(headers: &HeaderMap) -> InboundSource {
    if headers.get("x-ses-message-id").is_some() {
        return InboundSource::AwsSes {
            spam_verdict: headers
                .get("x-ses-spam-verdict")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
            virus_verdict: headers
                .get("x-ses-virus-verdict")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
        };
    }

    if headers.get("x-cf-mail-from").is_some() {
        return InboundSource::Cloudflare;
    }

    InboundSource::Unknown
}

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignEmailRequest {
    pub task_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListUnassignedQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookResponse {
    pub status: String,
    pub task_id: Option<Uuid>,
    pub thread_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedEmailResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_name: Option<String>,
    pub from_address: String,
    pub subject: String,
    pub snippet: Option<String>,
    pub has_attachments: bool,
    pub received_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    pub has_more: bool,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedEmailListResponse {
    pub emails: Vec<UnassignedEmailResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignEmailResponse {
    pub task_id: Uuid,
    pub thread_id: Option<Uuid>,
}

fn unassigned_to_response(email: &UnassignedEmail) -> UnassignedEmailResponse {
    UnassignedEmailResponse {
        id: email.id,
        project_id: email.project_id,
        from_name: email.from_name.clone(),
        from_address: email.from_address.clone(),
        subject: email.subject.clone(),
        snippet: email.snippet.clone(),
        has_attachments: email.has_attachments,
        received_at: email.received_at.to_rfc3339(),
    }
}

// ── Inbound Webhook Handler (outside auth middleware) ─────────

/// Receive an inbound email via webhook.
///
/// Expects `Authorization: Bearer {API_KEY}` header and raw MIME body
/// with `Content-Type: message/rfc822`.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid API key, parse error, or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/email/inbound",
    responses(
        (status = 200, body = InboundWebhookResponse),
        (status = 401, body = ProblemDetails),
        (status = 400, body = ProblemDetails),
    ),
    tag = "email-inbound",
)]
pub async fn receive_inbound_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<InboundWebhookResponse>, ProblemDetails> {
    // Validate API key
    let expected_key = state
        .email_inbound_api_key
        .as_deref()
        .ok_or_else(|| ProblemDetails::from(EmailError::InvalidApiKey))?;

    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ProblemDetails::from(EmailError::InvalidApiKey))?;

    if auth_header != expected_key {
        return Err(ProblemDetails::from(EmailError::InvalidApiKey));
    }

    // Detect inbound source (AWS SES, Cloudflare, or Unknown)
    let source = detect_source(&headers);
    tracing::info!(source = %source, "Inbound email source detected");

    // SES security checks
    if let InboundSource::AwsSes {
        ref spam_verdict,
        ref virus_verdict,
        ..
    } = source
    {
        if spam_verdict.as_deref() == Some("FAIL") || virus_verdict.as_deref() == Some("FAIL") {
            return Err(
                ProblemDetails::new(axum::http::StatusCode::BAD_REQUEST, "Email rejected")
                    .with_detail("Email failed spam or virus check."),
            );
        }
    }

    // Parse X-Project-Id header for default project, fallback to Uuid::nil()
    let default_project_id = headers
        .get("x-project-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Uuid::parse_str(v).ok())
        .unwrap_or_else(Uuid::nil);

    let result = state
        .inbound_email_service
        .process_inbound(&body, default_project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(InboundWebhookResponse {
        status: format!("{:?}", result.status),
        task_id: result.task_id,
        thread_id: result.thread_id,
    }))
}

// ── Unassigned Inbox Handlers (inside auth middleware) ─────────

/// List unassigned emails for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/unassigned-inbox",
    responses(
        (status = 200, body = UnassignedEmailListResponse),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("cursor" = Option<Uuid>, Query,),
        ("limit" = Option<i64>, Query,),
    ),
    tag = "email-inbound",
)]
pub async fn list_unassigned_inbox(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListUnassignedQuery>,
) -> Result<Json<UnassignedEmailListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let limit_usize = usize::try_from(limit).unwrap_or(50);
    let mut emails = UnassignedEmailRepository::list_by_project(
        &state.pool,
        project_id,
        query.cursor,
        limit + 1,
    )
    .await
    .map_err(ProblemDetails::from)?;

    let has_more = emails.len() > limit_usize;
    if has_more {
        emails.truncate(limit_usize);
    }

    let next_cursor = if has_more {
        emails.last().map(|e| e.id)
    } else {
        None
    };

    Ok(Json(UnassignedEmailListResponse {
        emails: emails.iter().map(unassigned_to_response).collect(),
        pagination: PaginationInfo {
            has_more,
            next_cursor,
        },
    }))
}

/// Assign an unassigned email to a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure, not found, or already assigned.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/unassigned-inbox/{emailId}/assign",
    request_body = AssignEmailRequest,
    responses(
        (status = 200, body = AssignEmailResponse),
        (status = 404, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("emailId" = Uuid, Path,),
    ),
    tag = "email-inbound",
)]
pub async fn assign_unassigned_email(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, email_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<AssignEmailRequest>,
) -> Result<Json<AssignEmailResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageConversation,
    )
    .await?;

    let result = state
        .inbound_email_service
        .assign_email(email_id, body.task_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(AssignEmailResponse {
        task_id: body.task_id,
        thread_id: result.thread_id,
    }))
}
