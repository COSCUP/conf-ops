use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum MemberTagError {
    #[error("Tag not found")]
    NotFound,

    #[error("Assignment not found")]
    AssignmentNotFound,

    #[error("Assignment already exists for this tag")]
    AssignmentAlreadyExists,

    #[error("Invalid assignment: {0}")]
    InvalidAssignment(String),

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<MemberTagError> for ProblemDetails {
    fn from(err: MemberTagError) -> Self {
        match err {
            MemberTagError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            MemberTagError::AssignmentNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Assignment Not Found")
                    .with_detail(err.to_string())
            }
            MemberTagError::AssignmentAlreadyExists => {
                Self::new(StatusCode::CONFLICT, "Assignment Already Exists")
                    .with_detail(err.to_string())
            }
            MemberTagError::InvalidAssignment(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Assignment")
                    .with_detail(err.to_string())
            }
            MemberTagError::Forbidden => {
                Self::new(StatusCode::FORBIDDEN, "Forbidden").with_detail(err.to_string())
            }
            MemberTagError::Database(_) => Self::internal_server_error(),
        }
    }
}
