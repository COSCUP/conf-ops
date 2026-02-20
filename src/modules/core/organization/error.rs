use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum OrgError {
    #[error("Organization not found")]
    NotFound,

    #[error("Not a member of this organization")]
    NotMember,

    #[error("Member not found")]
    MemberNotFound,

    #[error("Account is already a member of this organization")]
    MemberAlreadyExists,

    #[error("Cannot remove the last owner of the organization")]
    LastOwnerRemoval,

    #[error("Organization has active projects and cannot be deleted")]
    HasActiveProjects,

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<OrgError> for ProblemDetails {
    fn from(err: OrgError) -> Self {
        match err {
            OrgError::NotFound | OrgError::MemberNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            OrgError::NotMember | OrgError::Forbidden => {
                Self::new(StatusCode::FORBIDDEN, "Forbidden").with_detail(err.to_string())
            }
            OrgError::MemberAlreadyExists
            | OrgError::LastOwnerRemoval
            | OrgError::HasActiveProjects => {
                Self::new(StatusCode::CONFLICT, "Conflict").with_detail(err.to_string())
            }
            OrgError::Database(_) => Self::internal_server_error(),
        }
    }
}
