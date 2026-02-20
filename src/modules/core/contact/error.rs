use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum ContactError {
    #[error("Contact not found")]
    NotFound,

    #[error("Contact has already been merged")]
    AlreadyMerged,

    #[error("Merge cycle detected")]
    MergeCycle,

    #[error("Source IDs must not contain the target ID")]
    SourceContainsTarget,

    #[error("All contacts must belong to the same organization")]
    CrossOrgMerge,

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<ContactError> for ProblemDetails {
    fn from(err: ContactError) -> Self {
        match err {
            ContactError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            ContactError::AlreadyMerged => {
                Self::new(StatusCode::CONFLICT, "Already Merged").with_detail(err.to_string())
            }
            ContactError::MergeCycle => {
                Self::new(StatusCode::CONFLICT, "Merge Cycle").with_detail(err.to_string())
            }
            ContactError::SourceContainsTarget => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Merge Request")
                    .with_detail(err.to_string())
            }
            ContactError::CrossOrgMerge => {
                Self::new(StatusCode::BAD_REQUEST, "Cross-Organization Merge")
                    .with_detail(err.to_string())
            }
            ContactError::Forbidden => {
                Self::new(StatusCode::FORBIDDEN, "Forbidden").with_detail(err.to_string())
            }
            ContactError::Database(_) => Self::internal_server_error(),
        }
    }
}
