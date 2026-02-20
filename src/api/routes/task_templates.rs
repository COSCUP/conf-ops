use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::extractors::permission::require_permission;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::core::permission::service::{Action, Resource};
use crate::modules::core::project::repository::ProjectRepository;
use crate::modules::core::task_template::models::DataSchemaField;

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskTemplateRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskTemplateRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateListResponse {
    pub templates: Vec<TaskTemplateResponse>,
}

// ── Tag link types ──────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkTagRequest {
    pub member_tag_id: Uuid,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateTagResponse {
    pub id: Uuid,
    pub task_template_id: Uuid,
    pub member_tag_id: Uuid,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateTagListResponse {
    pub tags: Vec<TaskTemplateTagResponse>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnlinkTagRequest {
    pub member_tag_id: Uuid,
}

// ── Todo template types ─────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTodoTemplateRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTodoTemplateRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReorderTodoTemplatesRequest {
    pub orders: Vec<ReorderItem>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReorderItem {
    pub id: Uuid,
    pub sort_order: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoTemplateResponse {
    pub id: Uuid,
    pub task_template_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoTemplateListResponse {
    pub todo_templates: Vec<TodoTemplateResponse>,
}

// ── Data schema types ───────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDataSchemaRequest {
    pub name: String,
    pub fields: Vec<DataSchemaField>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataSchemaRequest {
    pub name: Option<String>,
    pub fields: Option<Vec<DataSchemaField>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSchemaResponse {
    pub id: Uuid,
    pub task_template_id: Uuid,
    pub name: String,
    #[schema(value_type = Vec<DataSchemaField>)]
    pub fields: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSchemaListResponse {
    pub data_schemas: Vec<DataSchemaResponse>,
}

// ── Path params ─────────────────────────────────────────────

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Path)]
pub struct TemplatePathParams {
    pub project_id: Uuid,
    pub template_id: Uuid,
}

// ── Task Template Handlers ──────────────────────────────────

/// List task templates for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Template list", body = TaskTemplateListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_templates(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TaskTemplateListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let templates = state
        .task_template_service
        .list_templates(project_id)
        .await
        .map_err(ProblemDetails::from)?;

    let templates = templates.into_iter().map(template_to_response).collect();

    Ok(Json(TaskTemplateListResponse { templates }))
}

/// Create a new task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/task-templates",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(("projectId" = Uuid, Path, description = "Project ID")),
    request_body = CreateTaskTemplateRequest,
    responses(
        (status = 201, description = "Template created", body = TaskTemplateResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateTaskTemplateRequest>,
) -> Result<(StatusCode, Json<TaskTemplateResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::CreateTaskTemplate,
    )
    .await?;

    let template = state
        .task_template_service
        .create_template(
            project_id,
            &body.name,
            body.description.as_deref(),
            user.account_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(template_to_response(template))))
}

/// Get a task template by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template detail", body = TaskTemplateResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TaskTemplateResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let template = state
        .task_template_service
        .get_template(template_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(template_to_response(template)))
}

/// Update a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    request_body = UpdateTaskTemplateRequest,
    responses(
        (status = 200, description = "Template updated", body = TaskTemplateResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTaskTemplateRequest>,
) -> Result<Json<TaskTemplateResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let description = body
        .description
        .map(|d| d.map(|s| -> Box<str> { s.into_boxed_str() }));
    let description_ref = description.as_ref().map(|d| d.as_deref());

    let template = state
        .task_template_service
        .update_template(template_id, body.name.as_deref(), description_ref)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(template_to_response(template)))
}

/// Delete a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::DeleteTaskTemplate,
    )
    .await?;

    state
        .task_template_service
        .delete_template(template_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Tag link handlers ───────────────────────────────────────

/// List tags linked to a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/tags",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Tag list", body = TaskTemplateTagListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_template_tags(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TaskTemplateTagListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let tags = state
        .task_template_service
        .list_tags(template_id)
        .await
        .map_err(ProblemDetails::from)?;

    let tags = tags.iter().map(tag_link_to_response).collect();

    Ok(Json(TaskTemplateTagListResponse { tags }))
}

/// Link a tag to a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/tags",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    request_body = LinkTagRequest,
    responses(
        (status = 201, description = "Tag linked", body = TaskTemplateTagResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 409, description = "Tag already linked", body = ProblemDetails)
    )
)]
pub async fn link_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<LinkTagRequest>,
) -> Result<(StatusCode, Json<TaskTemplateTagResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let tag_link = state
        .task_template_service
        .link_tag(template_id, body.member_tag_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(tag_link_to_response(&tag_link))))
}

/// Unlink a tag from a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/tags/{memberTagId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("memberTagId" = Uuid, Path, description = "Member Tag ID"),
    ),
    responses(
        (status = 204, description = "Tag unlinked"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Tag link not found", body = ProblemDetails)
    )
)]
pub async fn unlink_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id, member_tag_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    state
        .task_template_service
        .unlink_tag(template_id, member_tag_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Todo template handlers ──────────────────────────────────

/// List todo templates for a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Todo template list", body = TodoTemplateListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_todo_templates(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TodoTemplateListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let todo_templates = state
        .task_template_service
        .list_todo_templates(template_id)
        .await
        .map_err(ProblemDetails::from)?;

    let todo_templates = todo_templates
        .into_iter()
        .map(todo_template_to_response)
        .collect();

    Ok(Json(TodoTemplateListResponse { todo_templates }))
}

/// Create a todo template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    request_body = CreateTodoTemplateRequest,
    responses(
        (status = 201, description = "Todo template created", body = TodoTemplateResponse),
        (status = 400, description = "Nesting limit exceeded", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_todo_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateTodoTemplateRequest>,
) -> Result<(StatusCode, Json<TodoTemplateResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let todo_template = state
        .task_template_service
        .create_todo_template(
            template_id,
            body.parent_id,
            &body.name,
            body.description.as_deref(),
            body.sort_order,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((
        StatusCode::CREATED,
        Json(todo_template_to_response(todo_template)),
    ))
}

/// Get a todo template by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/{todoTemplateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("todoTemplateId" = Uuid, Path, description = "Todo Template ID"),
    ),
    responses(
        (status = 200, description = "Todo template detail", body = TodoTemplateResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_todo_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, todo_template_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<TodoTemplateResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let todo_template = state
        .task_template_service
        .get_todo_template(todo_template_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(todo_template_to_response(todo_template)))
}

/// Update a todo template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/{todoTemplateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("todoTemplateId" = Uuid, Path, description = "Todo Template ID"),
    ),
    request_body = UpdateTodoTemplateRequest,
    responses(
        (status = 200, description = "Todo template updated", body = TodoTemplateResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_todo_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, todo_template_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateTodoTemplateRequest>,
) -> Result<Json<TodoTemplateResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let description = body
        .description
        .map(|d| d.map(|s| -> Box<str> { s.into_boxed_str() }));
    let description_ref = description.as_ref().map(|d| d.as_deref());

    let todo_template = state
        .task_template_service
        .update_todo_template(todo_template_id, body.name.as_deref(), description_ref)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(todo_template_to_response(todo_template)))
}

/// Delete a todo template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/{todoTemplateId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("todoTemplateId" = Uuid, Path, description = "Todo Template ID"),
    ),
    responses(
        (status = 204, description = "Todo template deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_todo_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, todo_template_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    state
        .task_template_service
        .delete_todo_template(todo_template_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reorder todo templates.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/reorder",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    request_body = ReorderTodoTemplatesRequest,
    responses(
        (status = 204, description = "Reordered"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn reorder_todo_templates(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReorderTodoTemplatesRequest>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let orders: Vec<(Uuid, i32)> = body.orders.iter().map(|o| (o.id, o.sort_order)).collect();

    state
        .task_template_service
        .reorder_todo_templates(&orders)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Data schema handlers ────────────────────────────────────

/// List data schemas for a task template.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Data schema list", body = DataSchemaListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn list_data_schemas(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DataSchemaListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let schemas = state
        .task_template_service
        .list_data_schemas(template_id)
        .await
        .map_err(ProblemDetails::from)?;

    let data_schemas = schemas.into_iter().map(schema_to_response).collect();

    Ok(Json(DataSchemaListResponse { data_schemas }))
}

/// Create a data schema.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
    ),
    request_body = CreateDataSchemaRequest,
    responses(
        (status = 201, description = "Data schema created", body = DataSchemaResponse),
        (status = 400, description = "Validation error", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails)
    )
)]
pub async fn create_data_schema(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, template_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateDataSchemaRequest>,
) -> Result<(StatusCode, Json<DataSchemaResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let schema = state
        .task_template_service
        .create_data_schema(template_id, &body.name, body.fields)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(schema_to_response(schema))))
}

/// Get a data schema by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas/{schemaId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("schemaId" = Uuid, Path, description = "Data Schema ID"),
    ),
    responses(
        (status = 200, description = "Data schema detail", body = DataSchemaResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_data_schema(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DataSchemaResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTaskTemplates,
    )
    .await?;

    let schema = state
        .task_template_service
        .get_data_schema(schema_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(schema_to_response(schema)))
}

/// Update a data schema.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas/{schemaId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("schemaId" = Uuid, Path, description = "Data Schema ID"),
    ),
    request_body = UpdateDataSchemaRequest,
    responses(
        (status = 200, description = "Data schema updated", body = DataSchemaResponse),
        (status = 400, description = "Validation error", body = ProblemDetails),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn update_data_schema(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateDataSchemaRequest>,
) -> Result<Json<DataSchemaResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    let schema = state
        .task_template_service
        .update_data_schema(schema_id, body.name.as_deref(), body.fields)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(schema_to_response(schema)))
}

/// Delete a data schema.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas/{schemaId}",
    tag = "task-templates",
    security(("bearer_auth" = [])),
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("templateId" = Uuid, Path, description = "Template ID"),
        ("schemaId" = Uuid, Path, description = "Data Schema ID"),
    ),
    responses(
        (status = 204, description = "Data schema deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 403, description = "Forbidden", body = ProblemDetails),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn delete_data_schema(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _template_id, schema_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskTemplate,
    )
    .await?;

    state
        .task_template_service
        .delete_data_schema(schema_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────

fn template_to_response(
    t: crate::modules::core::task_template::models::TaskTemplate,
) -> TaskTemplateResponse {
    TaskTemplateResponse {
        id: t.id,
        project_id: t.project_id,
        name: t.name,
        description: t.description,
        created_by: t.created_by,
        created_at: t.created_at.to_rfc3339(),
        updated_at: t.updated_at.to_rfc3339(),
    }
}

fn tag_link_to_response(
    t: &crate::modules::core::task_template::models::TaskTemplateTag,
) -> TaskTemplateTagResponse {
    TaskTemplateTagResponse {
        id: t.id,
        task_template_id: t.task_template_id,
        member_tag_id: t.member_tag_id,
        created_at: t.created_at.to_rfc3339(),
    }
}

fn todo_template_to_response(
    t: crate::modules::core::task_template::models::TodoTemplate,
) -> TodoTemplateResponse {
    TodoTemplateResponse {
        id: t.id,
        task_template_id: t.task_template_id,
        parent_id: t.parent_id,
        name: t.name,
        description: t.description,
        sort_order: t.sort_order,
        created_at: t.created_at.to_rfc3339(),
        updated_at: t.updated_at.to_rfc3339(),
    }
}

fn schema_to_response(
    s: crate::modules::core::task_template::models::DataSchema,
) -> DataSchemaResponse {
    DataSchemaResponse {
        id: s.id,
        task_template_id: s.task_template_id,
        name: s.name,
        fields: s.fields,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
    }
}
