use axum::extract::{Path, Query, State};
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
use crate::modules::core::todo::models::{MyTodoItem, Todo, TodoAssignee, TodoStatus, TodoType};

// ── Request/Response Types ────────────────────────────────────

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTodoRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub due_date: Option<Option<String>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTodoStatusRequest {
    pub status: TodoStatus,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignTodoRequest {
    pub member_id: Uuid,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    pub todo_type: TodoType,
    pub source_template_id: Option<Uuid>,
    pub due_date: Option<String>,
    pub sort_order: i32,
    pub linked_task_id: Option<Uuid>,
    pub completed_at: Option<String>,
    pub completed_by: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoListResponse {
    pub todos: Vec<TodoResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoAssigneeResponse {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub member_id: Uuid,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodoAssigneeListResponse {
    pub assignees: Vec<TodoAssigneeResponse>,
}

fn todo_to_response(todo: &Todo) -> TodoResponse {
    TodoResponse {
        id: todo.id,
        task_id: todo.task_id,
        parent_id: todo.parent_id,
        title: todo.title.clone(),
        description: todo.description.clone(),
        status: todo.status.clone(),
        todo_type: todo.todo_type.clone(),
        source_template_id: todo.source_template_id,
        due_date: todo.due_date.map(|d| d.to_rfc3339()),
        sort_order: todo.sort_order,
        linked_task_id: todo.linked_task_id,
        completed_at: todo.completed_at.map(|d| d.to_rfc3339()),
        completed_by: todo.completed_by,
        created_at: todo.created_at.to_rfc3339(),
        updated_at: todo.updated_at.to_rfc3339(),
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyTodoResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    pub todo_type: TodoType,
    pub due_date: Option<String>,
    pub project_id: Uuid,
    pub project_name: String,
    pub task_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyTodoListResponse {
    pub items: Vec<MyTodoResponse>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct MyTodosQuery {
    pub status: Option<TodoStatus>,
    pub project_id: Option<Uuid>,
}

fn my_todo_to_response(item: &MyTodoItem) -> MyTodoResponse {
    MyTodoResponse {
        id: item.id,
        task_id: item.task_id,
        title: item.title.clone(),
        description: item.description.clone(),
        status: item.status.clone(),
        todo_type: item.todo_type.clone(),
        due_date: item.due_date.map(|d| d.to_rfc3339()),
        project_id: item.project_id,
        project_name: item.project_name.clone(),
        task_name: item.task_name.clone(),
        created_at: item.created_at.to_rfc3339(),
        updated_at: item.updated_at.to_rfc3339(),
    }
}

fn assignee_to_response(a: &TodoAssignee) -> TodoAssigneeResponse {
    TodoAssigneeResponse {
        id: a.id,
        todo_id: a.todo_id,
        member_id: a.member_id,
        created_at: a.created_at.to_rfc3339(),
    }
}

// ── Handlers ──────────────────────────────────────────────────

/// List todos for a task.
///
/// # Errors
///
/// Returns `ProblemDetails` on permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos",
    responses(
        (status = 200, body = TodoListResponse),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn list_todos(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TodoListResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTodos,
    )
    .await?;

    let todos = state
        .todo_service
        .list_todos(task_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(TodoListResponse {
        todos: todos.iter().map(todo_to_response).collect(),
    }))
}

/// Create a new ad-hoc todo.
///
/// # Errors
///
/// Returns `ProblemDetails` on validation or permission failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos",
    request_body = CreateTodoRequest,
    responses(
        (status = 201, body = TodoResponse),
        (status = 400, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn create_todo(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateTodoRequest>,
) -> Result<(StatusCode, Json<TodoResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    let todo = state
        .todo_service
        .create_todo(
            task_id,
            body.parent_id,
            &body.title,
            body.description.as_deref(),
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(todo_to_response(&todo))))
}

/// Get a todo by ID.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}",
    responses(
        (status = 200, body = TodoResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn get_todo(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<TodoResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ViewTodos,
    )
    .await?;

    let todo = state
        .todo_service
        .get_todo(todo_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(todo_to_response(&todo)))
}

/// Update a todo.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}",
    request_body = UpdateTodoRequest,
    responses(
        (status = 200, body = TodoResponse),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn update_todo(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateTodoRequest>,
) -> Result<Json<TodoResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    let due_date = body.due_date.as_ref().map(|d| {
        d.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let todo = state
        .todo_service
        .update_todo(
            todo_id,
            body.title.as_deref(),
            body.description.as_ref().map(|d| d.as_deref()),
            due_date,
        )
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(todo_to_response(&todo)))
}

/// Delete a todo (soft-delete).
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn delete_todo(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    state
        .todo_service
        .delete_todo(todo_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update a todo's status.
///
/// # Errors
///
/// Returns `ProblemDetails` on conflict or permission failure.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/status",
    request_body = UpdateTodoStatusRequest,
    responses(
        (status = 200, body = TodoResponse),
        (status = 409, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn update_todo_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateTodoStatusRequest>,
) -> Result<Json<TodoResponse>, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    let todo = state
        .todo_service
        .update_todo_status(todo_id, &body.status, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(todo_to_response(&todo)))
}

/// Add an assignee to a todo.
///
/// # Errors
///
/// Returns `ProblemDetails` on conflict or permission failure.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/assignees",
    request_body = AssignTodoRequest,
    responses(
        (status = 201, body = TodoAssigneeResponse),
        (status = 409, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn add_assignee(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<AssignTodoRequest>,
) -> Result<(StatusCode, Json<TodoAssigneeResponse>), ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    let assignee = state
        .todo_service
        .add_assignee(todo_id, body.member_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok((StatusCode::CREATED, Json(assignee_to_response(&assignee))))
}

/// Remove an assignee from a todo.
///
/// # Errors
///
/// Returns `ProblemDetails` on not found or permission failure.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/assignees/{memberId}",
    responses(
        (status = 204),
        (status = 404, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("taskId" = Uuid, Path,),
        ("todoId" = Uuid, Path,),
        ("memberId" = Uuid, Path,),
    ),
    tag = "todos",
)]
pub async fn remove_assignee(
    State(state): State<AppState>,
    user: AuthUser,
    Path((project_id, _task_id, todo_id, member_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    let org_id = ProjectRepository::get_organization_id(&state.pool, project_id)
        .await
        .map_err(ProblemDetails::from)?;

    require_permission(
        &state,
        user.account_id,
        Resource::ProjectScoped { org_id, project_id },
        Action::ManageTodos,
    )
    .await?;

    state
        .todo_service
        .remove_assignee(todo_id, member_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// List todos assigned to the current user across all projects.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
#[utoipa::path(
    get,
    path = "/api/v1/accounts/me/todos",
    responses(
        (status = 200, body = MyTodoListResponse),
    ),
    params(MyTodosQuery),
    tag = "todos",
    operation_id = "list_my_todos",
)]
pub async fn list_my_todos(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<MyTodosQuery>,
) -> Result<Json<MyTodoListResponse>, ProblemDetails> {
    let items = state
        .todo_service
        .list_my_todos(user.account_id, query.status.as_ref(), query.project_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(MyTodoListResponse {
        items: items.iter().map(my_todo_to_response).collect(),
    }))
}
