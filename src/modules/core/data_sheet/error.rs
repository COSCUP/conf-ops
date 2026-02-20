use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum DataSheetError {
    #[error("Data entry not found")]
    NotFound,

    #[error("Data schema not found")]
    SchemaNotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<DataSheetError> for ProblemDetails {
    fn from(err: DataSheetError) -> Self {
        match err {
            DataSheetError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            DataSheetError::SchemaNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Schema Not Found").with_detail(err.to_string())
            }
            DataSheetError::ValidationFailed(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Validation Error").with_detail(err.to_string())
            }
            DataSheetError::Database(_) => Self::internal_server_error(),
        }
    }
}
