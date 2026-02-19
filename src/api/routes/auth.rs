use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs_proto::{PublicKeyCredential, RegisterPublicKeyCredential};

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::auth::jwt::rotate_refresh_token;
use crate::modules::auth::repository::RefreshTokenRepository;

// ── Request/Response types ──────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct MagicLinkRequest {
    pub email: String,
}

#[derive(Serialize, ToSchema)]
pub struct MagicLinkResponse {
    pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct MagicLinkVerifyRequest {
    pub token: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct PasskeyRegisterCompleteRequest {
    #[schema(value_type = Object)]
    pub credential: RegisterPublicKeyCredential,
    #[serde(default = "default_passkey_name")]
    pub name: String,
}

fn default_passkey_name() -> String {
    "My Passkey".to_string()
}

#[derive(Serialize, ToSchema)]
pub struct PasskeyLoginBeginResponse {
    pub challenge_id: Uuid,
    #[schema(value_type = Object)]
    #[serde(flatten)]
    pub options: serde_json::Value,
}

#[derive(Deserialize, ToSchema)]
pub struct PasskeyLoginCompleteRequest {
    pub challenge_id: Uuid,
    #[schema(value_type = Object)]
    pub credential: PublicKeyCredential,
}

// ── Magic Link ──────────────────────────────────────────────────

/// Request a magic link email.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    post,
    path = "/api/v1/auth/magic-link/request",
    tag = "auth",
    request_body = MagicLinkRequest,
    responses(
        (status = 200, description = "Magic link email sent", body = MagicLinkResponse),
        (status = 400, description = "Invalid request", body = ProblemDetails)
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/auth/magic-link/verify",
    tag = "auth",
    params(
        ("token" = String, Query, description = "Magic link token")
    ),
    responses(
        (status = 200, description = "Authentication successful", body = AuthTokenResponse),
        (status = 401, description = "Invalid or expired token", body = ProblemDetails)
    )
)]
pub async fn verify_magic_link(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(params): Query<MagicLinkVerifyRequest>,
) -> Result<(CookieJar, Json<AuthTokenResponse>), ProblemDetails> {
    let (access_token, refresh_token, _account_id) = state
        .auth_service
        .verify_magic_link(&params.token)
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

// ── Passkey ─────────────────────────────────────────────────────

/// Begin passkey registration (requires authentication).
///
/// # Errors
///
/// Returns `ProblemDetails` on `WebAuthn` or database failure.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/register/begin",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Registration challenge created", body = Object),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn passkey_register_begin(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ProblemDetails> {
    let ccr = state
        .auth_service
        .passkey_register_begin(user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    let json = serde_json::to_value(ccr).map_err(|e| {
        ProblemDetails::new(StatusCode::INTERNAL_SERVER_ERROR, "Serialization error")
            .with_detail(e.to_string())
    })?;

    Ok(Json(json))
}

/// Complete passkey registration.
///
/// # Errors
///
/// Returns `ProblemDetails` on `WebAuthn` verification failure.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/register/complete",
    tag = "auth",
    security(("bearer_auth" = [])),
    request_body = PasskeyRegisterCompleteRequest,
    responses(
        (status = 201, description = "Passkey registered"),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn passkey_register_complete(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PasskeyRegisterCompleteRequest>,
) -> Result<StatusCode, ProblemDetails> {
    state
        .auth_service
        .passkey_register_complete(user.account_id, &body.credential, &body.name)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::CREATED)
}

/// Begin passkey login (public, no auth required).
///
/// # Errors
///
/// Returns `ProblemDetails` on `WebAuthn` failure.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/login/begin",
    tag = "auth",
    responses(
        (status = 200, description = "Login challenge created", body = PasskeyLoginBeginResponse),
        (status = 500, description = "Internal error", body = ProblemDetails)
    )
)]
pub async fn passkey_login_begin(
    State(state): State<AppState>,
) -> Result<Json<PasskeyLoginBeginResponse>, ProblemDetails> {
    let (rcr, challenge_id) = state
        .auth_service
        .passkey_login_begin()
        .map_err(ProblemDetails::from)?;

    let options = serde_json::to_value(rcr).map_err(|e| {
        ProblemDetails::new(StatusCode::INTERNAL_SERVER_ERROR, "Serialization error")
            .with_detail(e.to_string())
    })?;

    Ok(Json(PasskeyLoginBeginResponse {
        challenge_id,
        options,
    }))
}

/// Complete passkey login and return tokens.
///
/// # Errors
///
/// Returns `ProblemDetails` on `WebAuthn` verification or credential failure.
#[utoipa::path(
    post,
    path = "/api/v1/auth/passkey/login/complete",
    tag = "auth",
    request_body = PasskeyLoginCompleteRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthTokenResponse),
        (status = 401, description = "Invalid credentials", body = ProblemDetails)
    )
)]
pub async fn passkey_login_complete(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<PasskeyLoginCompleteRequest>,
) -> Result<(CookieJar, Json<AuthTokenResponse>), ProblemDetails> {
    let (access_token, refresh_token, _account_id) = state
        .auth_service
        .passkey_login_complete(body.challenge_id, &body.credential)
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

// ── Refresh & Logout ────────────────────────────────────────────

/// Rotate the refresh token and return new tokens.
///
/// # Errors
///
/// Returns `ProblemDetails` if no refresh token cookie or token is invalid.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Tokens refreshed", body = AuthTokenResponse),
        (status = 401, description = "Invalid refresh token", body = ProblemDetails)
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Logged out successfully"),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
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
