use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Memory not found")]
    NotFound,

    #[error("Invalid scope type: {0}")]
    InvalidScopeType(String),

    #[error("Invalid scope ID: scope {scope_type} with ID {scope_id} does not exist")]
    InvalidScopeId {
        scope_type: String,
        scope_id: uuid::Uuid,
    },

    #[error("Library document not found")]
    LibraryDocumentNotFound,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<MemoryError> for ProblemDetails {
    fn from(err: MemoryError) -> Self {
        match err {
            MemoryError::NotFound | MemoryError::LibraryDocumentNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            MemoryError::InvalidScopeType(_) | MemoryError::InvalidScopeId { .. } => {
                Self::new(StatusCode::BAD_REQUEST, "Bad Request").with_detail(err.to_string())
            }
            MemoryError::Database(_) => Self::internal_server_error(),
        }
    }
}
