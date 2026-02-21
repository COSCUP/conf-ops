use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("File not found")]
    NotFound,

    #[error("File too large: {0}")]
    FileTooLarge(String),

    #[error("Unsupported MIME type: {0}")]
    UnsupportedMimeType(String),

    #[error("Missing file in upload")]
    MissingFile,

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid scope type: {0}")]
    InvalidScopeType(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<StorageError> for ProblemDetails {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            StorageError::FileTooLarge(_) => {
                Self::new(StatusCode::PAYLOAD_TOO_LARGE, "File Too Large")
                    .with_detail(err.to_string())
            }
            StorageError::UnsupportedMimeType(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Unsupported MIME Type")
                    .with_detail(err.to_string())
            }
            StorageError::MissingFile | StorageError::MissingField(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Bad Request").with_detail(err.to_string())
            }
            StorageError::InvalidScopeType(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Scope Type")
                    .with_detail(err.to_string())
            }
            StorageError::Io(_) | StorageError::Database(_) => Self::internal_server_error(),
        }
    }
}
