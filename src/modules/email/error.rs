use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Email thread not found")]
    ThreadNotFound,

    #[error("Email message not found")]
    MessageNotFound,

    #[error("Unassigned email not found")]
    UnassignedEmailNotFound,

    #[error("Invalid email format: {0}")]
    InvalidEmailFormat(String),

    #[error("MIME parse error: {0}")]
    MimeParseError(String),

    #[error("SMTP error: {0}")]
    SmtpError(String),

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Email already assigned")]
    AlreadyAssigned,

    #[error("Duplicate email (message-id already exists)")]
    DuplicateEmail,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<EmailError> for ProblemDetails {
    fn from(err: EmailError) -> Self {
        match err {
            EmailError::ThreadNotFound
            | EmailError::MessageNotFound
            | EmailError::UnassignedEmailNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            EmailError::InvalidEmailFormat(_) | EmailError::MimeParseError(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Bad Request").with_detail(err.to_string())
            }
            EmailError::SmtpError(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "SMTP Error").with_detail(err.to_string())
            }
            EmailError::InvalidApiKey => {
                Self::new(StatusCode::UNAUTHORIZED, "Unauthorized").with_detail(err.to_string())
            }
            EmailError::AlreadyAssigned => {
                Self::new(StatusCode::CONFLICT, "Already Assigned").with_detail(err.to_string())
            }
            EmailError::DuplicateEmail => {
                Self::new(StatusCode::CONFLICT, "Duplicate Email").with_detail(err.to_string())
            }
            EmailError::Database(_) => Self::internal_server_error(),
        }
    }
}
