use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::events::DomainEvent;
use crate::modules::auth::repository::{AccountRepository, PasskeyCredentialRepository};

#[derive(Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct UpdateAccountRequest {
    pub display_name: Option<String>,
    pub avatar_url: Option<Option<String>>,
    pub locale: Option<String>,
}

#[derive(Serialize)]
pub struct ProfileResponse {
    pub profile: serde_json::Value,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub profile: serde_json::Value,
}

#[derive(Serialize)]
pub struct PasskeyResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Serialize)]
pub struct NotificationPreferencesResponse {
    pub email_notifications: bool,
    pub push_notifications: bool,
}

#[derive(Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub email_notifications: Option<bool>,
    pub push_notifications: Option<bool>,
}

/// Get the current authenticated user's account.
///
/// # Errors
///
/// Returns `ProblemDetails` if account not found.
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<AccountResponse>, ProblemDetails> {
    let account = AccountRepository::get_by_id(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(AccountResponse {
        id: account.id,
        email: account.email,
        display_name: account.display_name,
        avatar_url: account.avatar_url,
        locale: account.locale,
        created_at: account.created_at.to_rfc3339(),
        updated_at: account.updated_at.to_rfc3339(),
    }))
}

/// Update the current user's account.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, ProblemDetails> {
    let avatar_url = body.avatar_url;

    let account = AccountRepository::update(
        &state.pool,
        user.account_id,
        body.display_name.as_deref(),
        avatar_url.as_ref().map(|v| v.as_deref()),
        body.locale.as_deref(),
    )
    .await
    .map_err(ProblemDetails::from)?;

    state.event_bus.publish(DomainEvent::AccountUpdated {
        account_id: account.id,
    });

    Ok(Json(AccountResponse {
        id: account.id,
        email: account.email,
        display_name: account.display_name,
        avatar_url: account.avatar_url,
        locale: account.locale,
        created_at: account.created_at.to_rfc3339(),
        updated_at: account.updated_at.to_rfc3339(),
    }))
}

/// Get the current user's profile data.
///
/// # Errors
///
/// Returns `ProblemDetails` if account not found.
pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ProfileResponse>, ProblemDetails> {
    let account = AccountRepository::get_by_id(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(ProfileResponse {
        profile: account.profile,
    }))
}

/// Update the current user's profile data.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, ProblemDetails> {
    let account = AccountRepository::update_profile(&state.pool, user.account_id, &body.profile)
        .await
        .map_err(ProblemDetails::from)?;

    state.event_bus.publish(DomainEvent::AccountUpdated {
        account_id: account.id,
    });

    Ok(Json(ProfileResponse {
        profile: account.profile,
    }))
}

/// List the current user's passkeys.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn list_passkeys(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<PasskeyResponse>>, ProblemDetails> {
    let creds = PasskeyCredentialRepository::list_by_account(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    let passkeys: Vec<PasskeyResponse> = creds
        .into_iter()
        .map(|c| PasskeyResponse {
            id: c.id,
            name: c.name,
            created_at: c.created_at.to_rfc3339(),
            last_used_at: c.last_used_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(passkeys))
}

/// Delete a passkey credential.
///
/// # Errors
///
/// Returns `ProblemDetails` if credential not found.
pub async fn delete_passkey(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(passkey_id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    PasskeyCredentialRepository::delete(&state.pool, passkey_id, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get notification preferences (stub).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn get_notification_preferences(
    _user: AuthUser,
) -> Result<Json<NotificationPreferencesResponse>, ProblemDetails> {
    Ok(Json(NotificationPreferencesResponse {
        email_notifications: true,
        push_notifications: false,
    }))
}

/// Update notification preferences (stub).
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
pub async fn update_notification_preferences(
    _user: AuthUser,
    Json(_body): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<NotificationPreferencesResponse>, ProblemDetails> {
    Ok(Json(NotificationPreferencesResponse {
        email_notifications: true,
        push_notifications: false,
    }))
}
