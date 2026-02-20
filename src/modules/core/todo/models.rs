use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "todo_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Open,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "todo_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TodoType {
    Template,
    AdHoc,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Todo {
    pub id: Uuid,
    pub task_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    #[sqlx(rename = "type")]
    pub todo_type: TodoType,
    pub source_template_id: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub sort_order: i32,
    pub linked_task_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TodoAssignee {
    pub id: Uuid,
    pub todo_id: Uuid,
    pub member_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A todo item with project context, used for cross-project "my todos" queries.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MyTodoItem {
    pub id: Uuid,
    pub task_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    #[sqlx(rename = "type")]
    pub todo_type: TodoType,
    pub due_date: Option<DateTime<Utc>>,
    pub project_id: Uuid,
    pub project_name: String,
    pub task_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
