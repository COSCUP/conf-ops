use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

/// Errors that can occur in the audit module.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Audit log not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<AuditError> for ProblemDetails {
    fn from(err: AuditError) -> Self {
        match err {
            AuditError::NotFound(_) => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            AuditError::PermissionDenied(_) => {
                Self::new(StatusCode::FORBIDDEN, "Permission Denied").with_detail(err.to_string())
            }
            AuditError::Database(_) => Self::internal_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let err = AuditError::NotFound("test-id".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
    }

    #[test]
    fn permission_denied_maps_to_403() {
        let err = AuditError::PermissionDenied("not admin".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 403);
    }
}
