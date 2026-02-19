use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

/// RFC 7807 Problem Details error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// Error type URI for programmatic identification.
    #[serde(rename = "type")]
    pub error_type: String,

    /// Human-readable error summary.
    pub title: String,

    /// HTTP status code.
    pub status: u16,

    /// Detailed error description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// URI of the specific resource where the error occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Field-level validation errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ValidationError>>,
}

/// A single field validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field path (e.g. "name", "config.apiKey").
    pub field: String,

    /// Human-readable error message.
    pub message: String,

    /// Error code for programmatic handling (e.g. "required", "`too_long`").
    pub code: String,
}

impl ProblemDetails {
    pub fn new(status: StatusCode, title: impl Into<String>) -> Self {
        Self {
            error_type: format!(
                "https://api.conf-ops.dev/errors/{}",
                slug_from_status(status)
            ),
            title: title.into(),
            status: status.as_u16(),
            detail: None,
            instance: None,
            errors: None,
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn internal_server_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            .with_detail("An unexpected error occurred")
    }

    pub fn service_unavailable(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable").with_detail(detail)
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

const fn slug_from_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad-request",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not-found",
        StatusCode::CONFLICT => "conflict",
        StatusCode::INTERNAL_SERVER_ERROR => "internal-server-error",
        StatusCode::SERVICE_UNAVAILABLE => "service-unavailable",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_details_serializes_correctly() {
        let problem = ProblemDetails::new(StatusCode::BAD_REQUEST, "Validation Error")
            .with_detail("Request validation failed");

        let json = serde_json::to_value(&problem).expect("should serialize");

        assert_eq!(json["type"], "https://api.conf-ops.dev/errors/bad-request");
        assert_eq!(json["title"], "Validation Error");
        assert_eq!(json["status"], 400);
        assert_eq!(json["detail"], "Request validation failed");
        assert!(json.get("instance").is_none());
        assert!(json.get("errors").is_none());
    }

    #[test]
    fn problem_details_omits_none_fields() {
        let problem = ProblemDetails::new(StatusCode::NOT_FOUND, "Not Found");
        let json = serde_json::to_value(&problem).expect("should serialize");

        assert!(json.get("detail").is_none());
        assert!(json.get("instance").is_none());
        assert!(json.get("errors").is_none());
    }
}
