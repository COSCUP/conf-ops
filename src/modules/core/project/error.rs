use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Project not found")]
    NotFound,

    #[error("Invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<ProjectError> for ProblemDetails {
    fn from(err: ProjectError) -> Self {
        match err {
            ProjectError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            ProjectError::InvalidStatusTransition { .. } => {
                Self::new(StatusCode::CONFLICT, "Invalid Status Transition")
                    .with_detail(err.to_string())
            }
            ProjectError::Forbidden => {
                Self::new(StatusCode::FORBIDDEN, "Forbidden").with_detail(err.to_string())
            }
            ProjectError::Database(_) => Self::internal_server_error(),
        }
    }
}
