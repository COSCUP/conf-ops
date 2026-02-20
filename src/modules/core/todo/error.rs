use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum TodoError {
    #[error("Todo not found")]
    NotFound,

    #[error("Nesting limit exceeded: todos support at most one level of nesting")]
    NestingLimitExceeded,

    #[error("Cannot complete todo: linked task is not completed")]
    LinkedTaskNotCompleted,

    #[error("Assignee already exists for this todo")]
    AssigneeAlreadyExists,

    #[error("Assignee not found")]
    AssigneeNotFound,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<TodoError> for ProblemDetails {
    fn from(err: TodoError) -> Self {
        match err {
            TodoError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            TodoError::NestingLimitExceeded => {
                Self::new(StatusCode::BAD_REQUEST, "Nesting Limit Exceeded")
                    .with_detail(err.to_string())
            }
            TodoError::LinkedTaskNotCompleted => {
                Self::new(StatusCode::CONFLICT, "Linked Task Not Completed")
                    .with_detail(err.to_string())
            }
            TodoError::AssigneeAlreadyExists => {
                Self::new(StatusCode::CONFLICT, "Assignee Already Exists")
                    .with_detail(err.to_string())
            }
            TodoError::AssigneeNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Assignee Not Found").with_detail(err.to_string())
            }
            TodoError::Database(_) => Self::internal_server_error(),
        }
    }
}
