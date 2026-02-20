use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum MemberError {
    #[error("Member not found")]
    NotFound,

    #[error("Member already exists in this project")]
    AlreadyExists,

    #[error("Account is not a member of the organization")]
    NotOrgMember,

    #[error("Cannot remove or demote the last owner")]
    LastOwnerRemoval,

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<MemberError> for ProblemDetails {
    fn from(err: MemberError) -> Self {
        match err {
            MemberError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            MemberError::AlreadyExists => Self::new(StatusCode::CONFLICT, "Member Already Exists")
                .with_detail(err.to_string()),
            MemberError::NotOrgMember => {
                Self::new(StatusCode::BAD_REQUEST, "Not Organization Member")
                    .with_detail(err.to_string())
            }
            MemberError::LastOwnerRemoval => {
                Self::new(StatusCode::CONFLICT, "Last Owner Removal").with_detail(err.to_string())
            }
            MemberError::Forbidden => {
                Self::new(StatusCode::FORBIDDEN, "Forbidden").with_detail(err.to_string())
            }
            MemberError::Database(_) => Self::internal_server_error(),
        }
    }
}
