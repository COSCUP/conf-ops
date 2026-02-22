use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

/// Errors that can occur in the notifications module.
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Notification not found: {0}")]
    NotFound(String),

    #[error("Notification preferences not found")]
    PreferencesNotFound,

    #[error("Web Push subscription not found")]
    SubscriptionNotFound,

    #[error("Duplicate Web Push subscription")]
    DuplicateSubscription,

    #[error("Web Push delivery failed: {0}")]
    WebPushFailed(String),

    #[error("Email delivery failed: {0}")]
    EmailFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<NotificationError> for ProblemDetails {
    fn from(err: NotificationError) -> Self {
        match err {
            NotificationError::NotFound(_)
            | NotificationError::PreferencesNotFound
            | NotificationError::SubscriptionNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            NotificationError::DuplicateSubscription => {
                Self::new(StatusCode::CONFLICT, "Duplicate Subscription")
                    .with_detail("A subscription with this endpoint already exists")
            }
            NotificationError::PermissionDenied(_) => {
                Self::new(StatusCode::FORBIDDEN, "Permission Denied").with_detail(err.to_string())
            }
            NotificationError::WebPushFailed(_) | NotificationError::EmailFailed(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "Delivery Failed").with_detail(err.to_string())
            }
            NotificationError::Database(_) => Self::internal_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let err = NotificationError::NotFound("test-id".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
    }

    #[test]
    fn duplicate_subscription_maps_to_409() {
        let err = NotificationError::DuplicateSubscription;
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 409);
    }

    #[test]
    fn permission_denied_maps_to_403() {
        let err = NotificationError::PermissionDenied("not owner".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 403);
    }

    #[test]
    fn web_push_failed_maps_to_502() {
        let err = NotificationError::WebPushFailed("timeout".to_string());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 502);
    }
}
