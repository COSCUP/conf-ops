use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

/// Errors that can occur in the webhook module.
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Webhook not found: {0}")]
    NotFound(String),

    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Webhook delivery failed: {0}")]
    DeliveryFailed(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<WebhookError> for ProblemDetails {
    fn from(err: WebhookError) -> Self {
        match err {
            WebhookError::NotFound(_) => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            WebhookError::InvalidEventType(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Event Type")
                    .with_detail(err.to_string())
            }
            WebhookError::PermissionDenied(_) => {
                Self::new(StatusCode::FORBIDDEN, "Permission Denied").with_detail(err.to_string())
            }
            WebhookError::DeliveryFailed(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "Webhook Delivery Failed")
                    .with_detail(err.to_string())
            }
            WebhookError::Database(_) => Self::internal_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let err = WebhookError::NotFound("test-id".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
    }

    #[test]
    fn invalid_event_type_maps_to_400() {
        let err = WebhookError::InvalidEventType("bad.event".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 400);
    }

    #[test]
    fn permission_denied_maps_to_403() {
        let err = WebhookError::PermissionDenied("not owner".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 403);
    }

    #[test]
    fn delivery_failed_maps_to_502() {
        let err = WebhookError::DeliveryFailed("timeout".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 502);
    }
}
