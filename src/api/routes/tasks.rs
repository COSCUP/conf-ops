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
use crate::modules::core::task::models::{Task, TaskStatus};

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub task_template_id: Uuid,
    pub owner_tag_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStatusRequest {
    pub status: TaskStatus,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksQuery {
    pub status: Option<TaskStatus>,
    pub tag_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_template_id: Uuid,
    pub owner_tag_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
}

fn task_to_response(task: &Task) -> TaskResponse {
    TaskResponse {
        id: task.id,
        project_id: task.project_id,
        task_template_id: task.task_template_id,
        owner_tag_id: task.owner_tag_id,
        name: task.name.clone(),
        description: task.description.clone(),
        status: task.status.clone(),
        created_by: task.created_by,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }
}

// ── Handlers ──────────────────────────────────────────────────

/// Create a new task from a template.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or permission failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, body = TaskResponse),
        (status = 400, body = ProblemDetails),
        (status = 404, body = ProblemDetails),
    ),
    params(("projectId" = Uuid, Path,)),
    tag = "tasks",
)]
pub async fn create_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::CreateTask,
    )
    .await?;

    let task = state
        .task_service
        .create_task(
            project_id,
            body.task_template_id,
            body.owner_tag_id,
            &body.name,
            body.description.as_deref(),
            user.account_id,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(task_to_response(&task))))
}

/// List tasks for a project.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks",
    responses(
        (status = 200, body = TaskListResponse),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("status" = Option<TaskStatus>, Query,),
        ("tagId" = Option<Uuid>, Query,),
    ),
    tag = "tasks",
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<TaskListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTasks,
    )
    .await?;

    let tasks = state
        .task_service
        .list_tasks(project_id, query.status.as_ref(), query.tag_id)
        .await
        .map_err(ProblemDetails::from)?;

    let response = TaskListResponse {
        tasks: tasks.iter().map(task_to_response).collect(),
    };

    Ok(Json(response))
}

/// Get a task by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}",
    responses(
        (status = 200, body = TaskResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "tasks",
)]
pub async fn get_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TaskResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTasks,
    )
    .await?;

    let task = state
        .task_service
        .get_task(task_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(task_to_response(&task)))
}

/// Update a task's name and/or description.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}",
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, body = TaskResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "tasks",
)]
pub async fn update_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTask,
    )
    .await?;

    let task = state
        .task_service
        .update_task(
            task_id,
            body.name.as_deref(),
            body.description.as_ref().map(|d| d.as_deref()),
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(task_to_response(&task)))
}

/// Delete a task (soft-delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "tasks",
)]
pub async fn delete_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTask,
    )
    .await?;

    state
        .task_service
        .delete_task(task_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update a task's status.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid transition or permission failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/status",
    request_body = UpdateTaskStatusRequest,
    responses(
        (status = 200, body = TaskResponse),
        (status = 400, body = ProblemDetails),
        (status = 409, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "tasks",
)]
pub async fn update_task_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTaskStatusRequest>,
) -> Result<Json<TaskResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::UpdateTaskStatus,
    )
    .await?;

    let task = state
        .task_service
        .update_task_status(task_id, &body.status)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(task_to_response(&task)))
}
