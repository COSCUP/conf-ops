use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::tools::error::ToolError;
use crate::modules::tools::models::{
    CreateToolConfigParams, ToolCategory, ToolContext, ToolDefinition, ToolScopeType, ToolType,
};
use crate::modules::tools::repository::ToolConfigRepository;

// ── Request/Response Types ────────────────────────────────────

/// Summary of an available tool.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub category: ToolCategory,
    pub description: String,
    pub requires_confirmation: bool,
}

/// Response for listing available tools.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResponse {
    pub tools: Vec<ToolSummary>,
}

/// Detailed tool definition response.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetToolDetailsResponse {
    #[serde(flatten)]
    pub definition: ToolDefinition,
}

/// Request to execute a tool.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolRequest {
    pub task_id: Uuid,
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub suggestion_id: Option<Uuid>,
}

/// Response after executing a tool.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolResponse {
    pub success: bool,
    pub result: serde_json::Value,
    pub duration_ms: u64,
}

/// Tool configuration item for API responses.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfigResponse {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub tool_type: String,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_config: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for listing tool configs.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListToolConfigsResponse {
    pub tool_configs: Vec<ToolConfigResponse>,
}

/// Request to create a tool configuration.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateToolConfigRequest {
    pub tool_type: Option<String>,
    pub tool_name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_empty_object")]
    pub config: serde_json::Value,
    #[serde(default)]
    pub mcp_server_config: Option<serde_json::Value>,
}

fn default_empty_object() -> serde_json::Value {
    serde_json::json!({})
}

/// Request to update a tool configuration.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateToolConfigRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp_server_config: Option<serde_json::Value>,
}

/// Query for listing tools with optional cursor.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsQuery {
    pub member_tag_id: Option<Uuid>,
}

// ── Handlers ──────────────────────────────────────────────────

/// List available tools for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tools",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "List of available tools", body = ListToolsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "tools",
)]
pub async fn list_tools(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(_query): Query<ListToolsQuery>,
) -> Result<Json<ListToolsResponse>, ProblemDetails> {
    // Get the project's organization_id for config inheritance
    let org_id = get_project_organization_id(&state, project_id).await?;

    let definitions = state
        .tool_service
        .list_available_tools(project_id, org_id, &[])
        .await
        .map_err(ProblemDetails::from)?;

    let tools = definitions
        .into_iter()
        .map(|d| ToolSummary {
            name: d.name,
            display_name: d.display_name,
            category: d.category,
            description: d.description,
            requires_confirmation: d.requires_confirmation,
        })
        .collect();

    Ok(Json(ListToolsResponse { tools }))
}

/// Get detailed definition of a specific tool.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tools/{toolName}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("toolName" = String, Path, description = "Tool name"),
    ),
    responses(
        (status = 200, description = "Tool details", body = GetToolDetailsResponse),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "tools",
)]
pub async fn get_tool_details(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((project_id, tool_name)): Path<(Uuid, String)>,
) -> Result<Json<GetToolDetailsResponse>, ProblemDetails> {
    let definitions = state.tool_service.builtin_tool_definitions();

    if let Some(def) = definitions.get(&tool_name) {
        return Ok(Json(GetToolDetailsResponse {
            definition: def.clone(),
        }));
    }

    // Check configurable/external tools from config
    let org_id = get_project_organization_id(&state, project_id).await?;

    let all_tools = state
        .tool_service
        .list_available_tools(project_id, org_id, &[])
        .await
        .map_err(ProblemDetails::from)?;

    let definition = all_tools
        .into_iter()
        .find(|d| d.name == tool_name)
        .ok_or_else(|| {
            ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found")
                .with_detail(format!("Tool {tool_name} not found"))
        })?;

    Ok(Json(GetToolDetailsResponse { definition }))
}

/// Execute a tool.
///
/// # Errors
///
/// Returns `ProblemDetails` on various failure conditions.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tools/{toolName}/execute",
    request_body = ExecuteToolRequest,
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("toolName" = String, Path, description = "Tool name"),
    ),
    responses(
        (status = 200, description = "Tool executed successfully", body = ExecuteToolResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
        (status = 403, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "tools",
)]
pub async fn execute_tool(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, tool_name)): Path<(Uuid, String)>,
    Json(body): Json<ExecuteToolRequest>,
) -> Result<Json<ExecuteToolResponse>, ProblemDetails> {
    let org_id = get_project_organization_id(&state, project_id).await?;

    let context = ToolContext {
        task_id: body.task_id,
        project_id,
        organization_id: org_id,
        actor_id: user.account_id,
        actor_tags: vec![],
    };

    let result = state
        .tool_service
        .execute_tool(&tool_name, body.parameters, &context, body.suggestion_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(ExecuteToolResponse {
        success: !result.is_error,
        result: result.content,
        duration_ms: result.duration_ms,
    }))
}

/// List tool configurations for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tool-configs",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "Tool configurations", body = ListToolConfigsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "tool-configs",
)]
pub async fn list_project_tool_configs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ListToolConfigsResponse>, ProblemDetails> {
    let configs = ToolConfigRepository::list_by_scope(&state.pool, "project", project_id)
        .await
        .map_err(ProblemDetails::from)?;

    let tool_configs = configs.into_iter().map(config_to_response).collect();

    Ok(Json(ListToolConfigsResponse { tool_configs }))
}

/// Create a tool configuration for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on duplicate, validation failure, or database error.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tool-configs",
    request_body = CreateToolConfigRequest,
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 201, description = "Tool config created", body = ToolConfigResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    tag = "tool-configs",
)]
pub async fn create_project_tool_config(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateToolConfigRequest>,
) -> Result<(StatusCode, Json<ToolConfigResponse>), ProblemDetails> {
    let tool_type = body
        .tool_type
        .as_deref()
        .unwrap_or("builtin")
        .parse::<ToolType>()
        .map_err(|e| ProblemDetails::new(StatusCode::BAD_REQUEST, "Bad Request").with_detail(e))?;

    let params = CreateToolConfigParams {
        id: Uuid::now_v7(),
        scope_type: ToolScopeType::Project,
        scope_id: project_id,
        tool_type,
        tool_name: body.tool_name,
        display_name: body.display_name,
        description: body.description,
        enabled: body.enabled,
        config: body.config,
        mcp_server_config: body.mcp_server_config,
    };

    let config = ToolConfigRepository::create(&state.pool, &params)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(config_to_response(config))))
}

/// Update a tool configuration for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tool-configs/{configId}",
    request_body = UpdateToolConfigRequest,
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("configId" = Uuid, Path, description = "Tool config ID"),
    ),
    responses(
        (status = 200, description = "Tool config updated", body = ToolConfigResponse),
        (status = 400, body = ProblemDetails),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "tool-configs",
)]
pub async fn update_project_tool_config(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, config_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateToolConfigRequest>,
) -> Result<Json<ToolConfigResponse>, ProblemDetails> {
    let config = ToolConfigRepository::update(
        &state.pool,
        config_id,
        body.display_name.as_deref(),
        body.description.as_deref(),
        body.enabled,
        body.config.as_ref(),
        body.mcp_server_config.as_ref(),
    )
    .await
    .map_err(ProblemDetails::from)?;

    Ok(Json(config_to_response(config)))
}

/// Delete a tool configuration for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or database failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tool-configs/{configId}",
    params(
        ("projectId" = Uuid, Path, description = "Project ID"),
        ("configId" = Uuid, Path, description = "Tool config ID"),
    ),
    responses(
        (status = 204, description = "Tool config deleted"),
        (status = 401, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    tag = "tool-configs",
)]
pub async fn delete_project_tool_config(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_project_id, config_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    ToolConfigRepository::soft_delete(&state.pool, config_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List tool configurations for an organization.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{orgId}/tool-configs",
    params(
        ("orgId" = Uuid, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization tool configurations", body = ListToolConfigsResponse),
        (status = 401, body = ProblemDetails),
    ),
    tag = "tool-configs",
)]
pub async fn list_org_tool_configs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ListToolConfigsResponse>, ProblemDetails> {
    let configs = ToolConfigRepository::list_by_scope(&state.pool, "organization", org_id)
        .await
        .map_err(ProblemDetails::from)?;

    let tool_configs = configs.into_iter().map(config_to_response).collect();

    Ok(Json(ListToolConfigsResponse { tool_configs }))
}

// ── Helpers ──────────────────────────────────────────────────

/// Fetch the `organization_id` for a given project.
async fn get_project_organization_id(
    state: &AppState,
    project_id: Uuid,
) -> Result<Uuid, ProblemDetails> {
    let row =
        sqlx::query("SELECT organization_id FROM projects WHERE id = $1 AND deleted_at IS NULL")
            .bind(project_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ProblemDetails::from(ToolError::Database(e)))?
            .ok_or_else(|| {
                ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found")
                    .with_detail(format!("Project {project_id} not found"))
            })?;

    row.try_get("organization_id")
        .map_err(|e| ProblemDetails::from(ToolError::Database(e)))
}

fn config_to_response(config: crate::modules::tools::models::ToolConfig) -> ToolConfigResponse {
    ToolConfigResponse {
        id: config.id,
        scope_type: config.scope_type,
        scope_id: config.scope_id,
        tool_type: config.tool_type,
        tool_name: config.tool_name,
        display_name: config.display_name,
        description: config.description,
        enabled: config.enabled,
        config: config.config,
        mcp_server_config: config.mcp_server_config,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}
