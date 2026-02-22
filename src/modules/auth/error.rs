use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Account not found")]
    AccountNotFound,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Token has already been used")]
    TokenAlreadyUsed,

    #[error("Refresh token revoked")]
    RefreshTokenRevoked,

    #[error("Credential not found")]
    CredentialNotFound,

    #[error("WebAuthn error: {0}")]
    WebAuthn(String),

    #[error("Email sending failed: {0}")]
    EmailSend(#[from] crate::modules::email::error::EmailError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<AuthError> for ProblemDetails {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::AccountNotFound | AuthError::CredentialNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            AuthError::EmailAlreadyExists => {
                Self::new(StatusCode::CONFLICT, "Email Already Exists").with_detail(err.to_string())
            }
            AuthError::InvalidToken
            | AuthError::TokenAlreadyUsed
            | AuthError::RefreshTokenRevoked => {
                Self::new(StatusCode::UNAUTHORIZED, "Unauthorized").with_detail(err.to_string())
            }
            AuthError::WebAuthn(_) => {
                Self::new(StatusCode::BAD_REQUEST, "WebAuthn Error").with_detail(err.to_string())
            }
            AuthError::EmailSend(_) | AuthError::Database(_) => Self::internal_server_error(),
        }
    }
}
