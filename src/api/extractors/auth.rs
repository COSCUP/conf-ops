use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;

/// Extractor that requires an authenticated user.
/// Returns 401 if no valid token was provided.
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ProblemDetails;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            ProblemDetails::new(StatusCode::UNAUTHORIZED, "Unauthorized")
                .with_detail("Authentication required")
        })
    }
}

/// Optional authenticated user extractor.
/// Returns `None` if no valid token was provided (never rejects).
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = ProblemDetails;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<AuthUser>().cloned()))
    }
}
