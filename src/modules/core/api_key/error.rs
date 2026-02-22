use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

/// Errors that can occur in the API key module.
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyError {
    #[error("API key not found: {0}")]
    NotFound(String),

    #[error("Invalid API key")]
    InvalidKey,

    #[error("API key limit exceeded: maximum {0} keys per project")]
    LimitExceeded(usize),

    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<ApiKeyError> for ProblemDetails {
    fn from(err: ApiKeyError) -> Self {
        match err {
            ApiKeyError::NotFound(_) => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            ApiKeyError::InvalidKey => Self::new(StatusCode::UNAUTHORIZED, "Invalid API Key")
                .with_detail("The provided API key is invalid or has been revoked"),
            ApiKeyError::LimitExceeded(_) => {
                Self::new(StatusCode::CONFLICT, "API Key Limit Exceeded")
                    .with_detail(err.to_string())
            }
            ApiKeyError::InvalidScope(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Scope").with_detail(err.to_string())
            }
            ApiKeyError::InsufficientPermissions(_) => {
                Self::new(StatusCode::FORBIDDEN, "Insufficient Permissions")
                    .with_detail(err.to_string())
            }
            ApiKeyError::PermissionDenied(_) => {
                Self::new(StatusCode::FORBIDDEN, "Permission Denied").with_detail(err.to_string())
            }
            ApiKeyError::Database(_) => Self::internal_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let err = ApiKeyError::NotFound("test-id".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
    }

    #[test]
    fn invalid_key_maps_to_401() {
        let err = ApiKeyError::InvalidKey;
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 401);
    }

    #[test]
    fn limit_exceeded_maps_to_409() {
        let err = ApiKeyError::LimitExceeded(10);
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 409);
    }

    #[test]
    fn insufficient_permissions_maps_to_403() {
        let err = ApiKeyError::InsufficientPermissions("need write:data".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 403);
    }
}
