use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::app_state::AppState;

/// Authenticated external API client extracted from X-API-Key header.
#[derive(Debug, Clone)]
pub struct ApiKeyUser {
    pub api_key_id: Uuid,
    pub project_id: Uuid,
    pub permissions: serde_json::Value,
}

/// Middleware that authenticates requests using the X-API-Key header.
///
/// # Errors
///
/// Returns 401 if the API key is missing or invalid.
pub async fn api_key_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let api_key_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let raw_key = api_key_header.ok_or_else(|| {
        ProblemDetails::new(StatusCode::UNAUTHORIZED, "Missing API Key")
            .with_detail("X-API-Key header is required")
    })?;

    let api_key = state
        .api_key_service
        .validate_key(&raw_key)
        .await
        .map_err(|_| {
            ProblemDetails::new(StatusCode::UNAUTHORIZED, "Invalid API Key")
                .with_detail("The provided API key is invalid or has been revoked")
        })?;

    let api_key_user = ApiKeyUser {
        api_key_id: api_key.id,
        project_id: api_key.project_id,
        permissions: api_key.permissions,
    };

    request.extensions_mut().insert(api_key_user);

    Ok(next.run(request).await)
}
