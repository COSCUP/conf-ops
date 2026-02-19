use std::collections::BTreeMap;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::events::DomainEvent;
use crate::modules::auth::repository::{AccountRepository, PasskeyCredentialRepository};

// ── Account ────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub avatar_url: Option<Option<String>>,
    pub bio: Option<Option<String>>,
    pub locale: Option<String>,
}

// ── Profile ────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    #[schema(value_type = Object)]
    pub profile_data: serde_json::Value,
    #[schema(value_type = Object)]
    pub profile_schema: serde_json::Value,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[schema(value_type = Object)]
    pub profile_data: serde_json::Value,
    #[schema(value_type = Option<Object>)]
    pub profile_schema: Option<serde_json::Value>,
}

// ── Passkey ────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PasskeyListResponse {
    pub passkeys: Vec<PasskeyResponse>,
}

// ── Notification Preferences ───────────────────────────────────

pub type ChannelCategories = BTreeMap<String, bool>;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct ChannelPreference {
    pub enabled: bool,
    pub categories: ChannelCategories,
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannels {
    pub email: ChannelPreference,
    pub web_push: ChannelPreference,
    pub in_app: ChannelPreference,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NotificationPreferencesResponse {
    pub channels: NotificationChannels,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateNotificationPreferencesRequest {
    pub channels: NotificationChannels,
}

// ── Handlers ───────────────────────────────────────────────────

/// Get the current authenticated user's account.
///
/// # Errors
///
/// Returns `ProblemDetails` if account not found.
#[utoipa::path(
    get,
    path = "/api/v1/accounts/me",
    tag = "accounts",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user account", body = AccountResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
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
        name: account.name,
        avatar_url: account.avatar_url,
        bio: account.bio,
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
#[utoipa::path(
    patch,
    path = "/api/v1/accounts/me",
    tag = "accounts",
    security(("bearer_auth" = [])),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Account updated", body = AccountResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, ProblemDetails> {
    let avatar_url = body.avatar_url;
    let bio = body.bio;

    let account = AccountRepository::update(
        &state.pool,
        user.account_id,
        body.name.as_deref(),
        avatar_url.as_ref().map(|v| v.as_deref()),
        bio.as_ref().map(|v| v.as_deref()),
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
        name: account.name,
        avatar_url: account.avatar_url,
        bio: account.bio,
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
#[utoipa::path(
    get,
    path = "/api/v1/accounts/me/profile",
    tag = "accounts",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User profile data", body = ProfileResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ProfileResponse>, ProblemDetails> {
    let account = AccountRepository::get_by_id(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    Ok(Json(ProfileResponse {
        profile_data: account.profile_data,
        profile_schema: account.profile_schema,
    }))
}

/// Update the current user's profile data.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/accounts/me/profile",
    tag = "accounts",
    security(("bearer_auth" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = ProfileResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, ProblemDetails> {
    let account = AccountRepository::update_profile(
        &state.pool,
        user.account_id,
        &body.profile_data,
        body.profile_schema.as_ref(),
    )
    .await
    .map_err(ProblemDetails::from)?;

    state.event_bus.publish(DomainEvent::AccountUpdated {
        account_id: account.id,
    });

    Ok(Json(ProfileResponse {
        profile_data: account.profile_data,
        profile_schema: account.profile_schema,
    }))
}

/// List the current user's passkeys.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/auth/passkeys",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of passkeys", body = PasskeyListResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn list_passkeys(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<PasskeyListResponse>, ProblemDetails> {
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

    Ok(Json(PasskeyListResponse { passkeys }))
}

/// Delete a passkey credential.
///
/// # Errors
///
/// Returns `ProblemDetails` if credential not found.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/passkeys/{id}",
    tag = "auth",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Passkey credential ID")
    ),
    responses(
        (status = 204, description = "Passkey deleted"),
        (status = 401, description = "Unauthorized", body = ProblemDetails),
        (status = 404, description = "Passkey not found", body = ProblemDetails)
    )
)]
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

fn default_notification_channels() -> NotificationChannels {
    let default_categories: ChannelCategories = [
        ("taskUpdates".to_string(), true),
        ("todoAssignments".to_string(), true),
        ("aiSuggestions".to_string(), true),
        ("mentions".to_string(), true),
        ("systemAnnouncements".to_string(), true),
    ]
    .into_iter()
    .collect();

    NotificationChannels {
        email: ChannelPreference {
            enabled: true,
            categories: default_categories.clone(),
        },
        web_push: ChannelPreference {
            enabled: true,
            categories: default_categories.clone(),
        },
        in_app: ChannelPreference {
            enabled: true,
            categories: default_categories,
        },
    }
}

/// Get notification preferences.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    get,
    path = "/api/v1/accounts/me/notification-preferences",
    tag = "accounts",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Notification preferences", body = NotificationPreferencesResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn get_notification_preferences(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<NotificationPreferencesResponse>, ProblemDetails> {
    let account = AccountRepository::get_by_id(&state.pool, user.account_id)
        .await
        .map_err(ProblemDetails::from)?;

    let prefs: NotificationPreferencesResponse =
        serde_json::from_value(account.notification_preferences).unwrap_or_else(|_| {
            NotificationPreferencesResponse {
                channels: default_notification_channels(),
            }
        });

    Ok(Json(prefs))
}

/// Update notification preferences.
///
/// # Errors
///
/// Returns `ProblemDetails` on failure.
#[utoipa::path(
    put,
    path = "/api/v1/accounts/me/notification-preferences",
    tag = "accounts",
    security(("bearer_auth" = [])),
    request_body = UpdateNotificationPreferencesRequest,
    responses(
        (status = 200, description = "Notification preferences updated", body = NotificationPreferencesResponse),
        (status = 401, description = "Unauthorized", body = ProblemDetails)
    )
)]
pub async fn update_notification_preferences(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<NotificationPreferencesResponse>, ProblemDetails> {
    let preferences =
        serde_json::to_value(&body).map_err(|_| ProblemDetails::internal_server_error())?;

    let account = AccountRepository::update_notification_preferences(
        &state.pool,
        user.account_id,
        &preferences,
    )
    .await
    .map_err(ProblemDetails::from)?;

    let prefs: NotificationPreferencesResponse =
        serde_json::from_value(account.notification_preferences).unwrap_or_else(|_| {
            NotificationPreferencesResponse {
                channels: default_notification_channels(),
            }
        });

    Ok(Json(prefs))
}
