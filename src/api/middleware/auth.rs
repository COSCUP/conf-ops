use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::modules::auth::jwt::validate_access_token;

/// Authenticated user identity extracted from JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub account_id: Uuid,
}

/// Middleware that extracts and validates JWT Bearer token.
/// Sets `AuthUser` as a request extension if valid.
/// Does NOT block requests without a token — that's the extractor's job.
pub async fn auth_middleware(
    state: axum::extract::State<crate::app_state::AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if let Ok(claims) = validate_access_token(&state.jwt_config, token) {
                    request.extensions_mut().insert(AuthUser {
                        account_id: claims.sub,
                    });
                }
            }
        }
    }

    next.run(request).await
}
