use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task not found")]
    NotFound,

    #[error("Task template not found")]
    TemplateNotFound,

    #[error("Invalid owner tag: the specified tag is not linked to the task template")]
    InvalidOwnerTag,

    #[error("Invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Cannot complete task: {0} incomplete todo(s) remaining")]
    IncompleteTodos(i64),

    #[error("External task creation not allowed for this tag and template combination")]
    ExternalCreationNotAllowed,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<TaskError> for ProblemDetails {
    fn from(err: TaskError) -> Self {
        match err {
            TaskError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            TaskError::TemplateNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Template Not Found").with_detail(err.to_string())
            }
            TaskError::InvalidOwnerTag => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Owner Tag").with_detail(err.to_string())
            }
            TaskError::InvalidStatusTransition { .. } => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Status Transition")
                    .with_detail(err.to_string())
            }
            TaskError::IncompleteTodos(_) => {
                Self::new(StatusCode::CONFLICT, "Cannot Complete Task").with_detail(err.to_string())
            }
            TaskError::ExternalCreationNotAllowed => {
                Self::new(StatusCode::FORBIDDEN, "External Creation Not Allowed")
                    .with_detail(err.to_string())
            }
            TaskError::Database(_) => Self::internal_server_error(),
        }
    }
}
