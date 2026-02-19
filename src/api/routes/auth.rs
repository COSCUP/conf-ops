use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::auth::jwt::rotate_refresh_token;
use crate::modules::auth::repository::RefreshTokenRepository;

#[derive(Deserialize)]
pub struct MagicLinkRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct MagicLinkResponse {
    pub message: String,
}

#[derive(Deserialize)]
pub struct MagicLinkVerifyRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Request a magic link email.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn request_magic_link(
    State(state): State<AppState>,
    Json(body): Json<MagicLinkRequest>,
) -> Result<Json<MagicLinkResponse>, ProblemDetails> {
    state
        .auth_service
        .request_magic_link(&body.email)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(MagicLinkResponse {
        message: "If your email is registered, you will receive a login link shortly.".to_string(),
    }))
}

/// Verify a magic link token and return access/refresh tokens.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid or expired token.
pub async fn verify_magic_link(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<MagicLinkVerifyRequest>,
) -> Result<(CookieJar, Json<AuthTokenResponse>), ProblemDetails> {
    let (access_token, refresh_token, _account_id) = state
        .auth_service
        .verify_magic_link(&body.token)
        .await
        .map_err(ProblemDetails::from)?;

    let cookie = build_refresh_cookie(refresh_token, state.jwt_config.refresh_token_expiry_secs);

    Ok((
        jar.add(cookie),
        Json(AuthTokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_config.access_token_expiry_secs,
        }),
    ))
}

/// Rotate the refresh token and return new tokens.
///
/// # Errors
///
/// Returns `ProblemDetails` if no refresh token cookie or token is invalid.
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AuthTokenResponse>), ProblemDetails> {
    let raw_refresh = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            ProblemDetails::new(StatusCode::UNAUTHORIZED, "Unauthorized")
                .with_detail("No refresh token provided")
        })?;

    let (access_token, new_refresh) =
        rotate_refresh_token(&state.pool, &state.jwt_config, &raw_refresh)
            .await
            .map_err(ProblemDetails::from)?;

    let cookie = build_refresh_cookie(new_refresh, state.jwt_config.refresh_token_expiry_secs);

    Ok((
        jar.add(cookie),
        Json(AuthTokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_config.access_token_expiry_secs,
        }),
    ))
}

/// Logout: revoke all refresh tokens and clear cookie.
///
/// # Errors
///
/// Returns `ProblemDetails` on database failure.
pub async fn logout(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
) -> Result<impl IntoResponse, ProblemDetails> {
    RefreshTokenRepository::revoke_all_for_account(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    let removal = Cookie::build("refresh_token")
        .path("/api/v1/auth")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::ZERO)
        .build();

    Ok((jar.add(removal), StatusCode::NO_CONTENT))
}

fn build_refresh_cookie(token: String, max_age_secs: i64) -> Cookie<'static> {
    Cookie::build(("refresh_token", token))
        .path("/api/v1/auth")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::seconds(max_age_secs))
        .build()
}
