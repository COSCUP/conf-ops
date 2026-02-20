use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum TaskTemplateError {
    #[error("Task template not found")]
    NotFound,

    #[error("Todo template not found")]
    TodoTemplateNotFound,

    #[error("Data schema not found")]
    DataSchemaNotFound,

    #[error("Nesting limit exceeded: todo templates support at most one level of nesting")]
    NestingLimitExceeded,

    #[error("Duplicate field key: {0}")]
    DuplicateFieldKey(String),

    #[error("Invalid field constraints: {0}")]
    InvalidFieldConstraints(String),

    #[error("Tag already linked to this template")]
    TagAlreadyLinked,

    #[error("Tag link not found")]
    TagLinkNotFound,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<TaskTemplateError> for ProblemDetails {
    fn from(err: TaskTemplateError) -> Self {
        match err {
            TaskTemplateError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            TaskTemplateError::TodoTemplateNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Todo Template Not Found")
                    .with_detail(err.to_string())
            }
            TaskTemplateError::DataSchemaNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Data Schema Not Found")
                    .with_detail(err.to_string())
            }
            TaskTemplateError::NestingLimitExceeded => {
                Self::new(StatusCode::BAD_REQUEST, "Nesting Limit Exceeded")
                    .with_detail(err.to_string())
            }
            TaskTemplateError::DuplicateFieldKey(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Duplicate Field Key")
                    .with_detail(err.to_string())
            }
            TaskTemplateError::InvalidFieldConstraints(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Field Constraints")
                    .with_detail(err.to_string())
            }
            TaskTemplateError::TagAlreadyLinked => {
                Self::new(StatusCode::CONFLICT, "Tag Already Linked").with_detail(err.to_string())
            }
            TaskTemplateError::TagLinkNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Tag Link Not Found").with_detail(err.to_string())
            }
            TaskTemplateError::Database(_) => Self::internal_server_error(),
        }
    }
}
