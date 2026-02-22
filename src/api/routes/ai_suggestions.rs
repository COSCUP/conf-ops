use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::ai::decision::DecideParams;
use crate::modules::ai::error::AiError;
use crate::modules::ai::models::SuggestionDecision;
use crate::modules::ai::models::SuggestionGroup;
use crate::modules::ai::repository::{AiPipelineRepository, AiSuggestionRepository};

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestionsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionGroupItem {
    pub message_id: Uuid,
    pub suggestion_group: SuggestionGroup,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestionsResponse {
    pub data: Vec<SuggestionGroupItem>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSuggestionGroupResponse {
    pub message_id: Uuid,
    pub suggestion_group: SuggestionGroup,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecideRequest {
    pub decision: SuggestionDecision,
    pub last_seen_message_id: Uuid,
    #[serde(default)]
    pub modified_parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub additional_instructions: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecideResponse {
    pub status: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestSuggestionBody {
    pub last_seen_message_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestSuggestionResponse {
    pub event_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolvePlaceholdersRequest {
    pub text: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolvePlaceholdersResponse {
    pub text: String,
    pub unresolved: Vec<String>,
}

// ── Handlers ──────────────────────────────────────────────────

/// List AI suggestion groups for a task, ordered newest first.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/suggestions",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
        ("cursor" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("limit" = Option<i64>, Query, description = "Number of items per page (max 100)"),
    ),
    responses(
        (status = 200, description = "List of suggestion groups", body = ListSuggestionsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "ai-suggestions",
)]
pub async fn list_suggestions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ListSuggestionsQuery>,
) -> Result<Json<ListSuggestionsResponse>, ProblemDetails> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let limit_usize = usize::try_from(limit).unwrap_or(20);

    let mut groups = AiSuggestionRepository::list_suggestion_groups(
        &state.pool,
        task_id,
        query.cursor,
        limit + 1,
    )
    .await
    .map_err(ProblemDetails::from)?;

    let has_more = groups.len() > limit_usize;
    if has_more {
        groups.truncate(limit_usize);
    }

    let next_cursor = if has_more {
        groups.last().map(|(msg_id, _)| *msg_id)
    } else {
        None
    };

    let data = groups
        .into_iter()
        .map(|(message_id, suggestion_group)| SuggestionGroupItem {
            message_id,
            suggestion_group,
        })
        .collect();

    Ok(Json(ListSuggestionsResponse { data, next_cursor }))
}

/// Get a single AI suggestion group by group ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/{groupId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
        ("groupId" = Uuid, Path, description = "Suggestion group ID"),
    ),
    responses(
        (status = 200, description = "Suggestion group detail", body = GetSuggestionGroupResponse),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "ai-suggestions",
)]
pub async fn get_suggestion_group(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, task_id, group_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<GetSuggestionGroupResponse>, ProblemDetails> {
    // List all suggestion groups for this task and find the one matching group_id
    let groups = AiSuggestionRepository::list_suggestion_groups(&state.pool, task_id, None, 1000)
        .await
        .map_err(ProblemDetails::from)?;

    let (message_id, suggestion_group) = groups
        .into_iter()
        .find(|(_, sg)| sg.id == group_id)
        .ok_or_else(|| {
            ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found")
                .with_detail(format!("Suggestion group {group_id} not found"))
        })?;

    Ok(Json(GetSuggestionGroupResponse {
        message_id,
        suggestion_group,
    }))
}

/// Make a decision on an AI suggestion.
///
/// # Errors
///
/// Returns `ProblemDetails` on stale conversation, already decided, not found,
/// invalid decision, or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/{groupId}/suggestions/{suggestionId}/decide",
    request_body = DecideRequest,
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
        ("groupId" = Uuid, Path, description = "Suggestion group ID"),
        ("suggestionId" = Uuid, Path, description = "Suggestion ID"),
    ),
    responses(
        (status = 200, description = "Decision recorded", body = DecideResponse),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    tag = "ai-suggestions",
)]
pub async fn decide_suggestion(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id, group_id, suggestion_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    Json(body): Json<DecideRequest>,
) -> Result<Json<DecideResponse>, ProblemDetails> {
    // Find the message_id for the given group within this task
    let groups = AiSuggestionRepository::list_suggestion_groups(&state.pool, task_id, None, 1000)
        .await
        .map_err(ProblemDetails::from)?;

    let (message_id, _) = groups
        .into_iter()
        .find(|(_, sg)| sg.id == group_id)
        .ok_or_else(|| {
            ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found")
                .with_detail(format!("Suggestion group {group_id} not found"))
        })?;

    state
        .decision_service
        .decide(DecideParams {
            message_id,
            group_id,
            suggestion_id,
            decision: body.decision,
            decided_by: user.account_id,
            last_seen_message_id: body.last_seen_message_id,
            modified_parameters: body.modified_parameters,
            execution_result: None,
            additional_instructions: body.additional_instructions,
        })
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(DecideResponse {
        status: "ok".to_string(),
    }))
}

/// Manually trigger an AI suggestion pipeline event for a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/request",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
    ),
    responses(
        (status = 202, description = "Pipeline event queued", body = RequestSuggestionResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "ai-suggestions",
)]
pub async fn request_suggestion(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RequestSuggestionBody>,
) -> Result<(StatusCode, Json<RequestSuggestionResponse>), ProblemDetails> {
    let payload = body.last_seen_message_id.map_or_else(
        || serde_json::json!({}),
        |last_seen| serde_json::json!({ "last_seen_message_id": last_seen }),
    );

    let event = AiPipelineRepository::insert_event(&state.pool, task_id, "manual_request", payload)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(RequestSuggestionResponse { event_id: event.id }),
    ))
}

/// Resolve `{{profile.*}}` and `{{data.*}}` placeholders in a text string.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/ai/resolve-placeholders",
    request_body = ResolvePlaceholdersRequest,
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("taskId" = Uuid, Path, description = "Task ID"),
    ),
    responses(
        (status = 200, description = "Resolved text with list of unresolved placeholders",
         body = ResolvePlaceholdersResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "ai-suggestions",
)]
pub async fn resolve_placeholders(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ResolvePlaceholdersRequest>,
) -> Result<Json<ResolvePlaceholdersResponse>, ProblemDetails> {
    let result = state
        .placeholder_resolver
        .resolve(&body.text, task_id, user.account_id)
        .await
        .map_err(|e| ProblemDetails::from(AiError::Database(e)))?;

    Ok(Json(ResolvePlaceholdersResponse {
        text: result.text,
        unresolved: result.unresolved,
    }))
}
