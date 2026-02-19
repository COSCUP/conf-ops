// utoipa's OpenApi derive macro generates code that triggers clippy::needless_for_each.
// This cannot be fixed without modifying the upstream utoipa crate.
#![allow(clippy::needless_for_each)]

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use super::routes::{accounts, auth, health};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Conf-Ops API",
        version = "0.1.0",
        description = "AI-assisted conference/event project management system"
    ),
    paths(
        health::healthz,
        health::readyz,
        auth::request_magic_link,
        auth::verify_magic_link,
        auth::passkey_register_begin,
        auth::passkey_register_complete,
        auth::passkey_login_begin,
        auth::passkey_login_complete,
        auth::refresh,
        auth::logout,
        accounts::get_me,
        accounts::update_me,
        accounts::get_profile,
        accounts::update_profile,
        accounts::list_passkeys,
        accounts::delete_passkey,
        accounts::get_notification_preferences,
        accounts::update_notification_preferences,
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "accounts", description = "Account management endpoints")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Generate the `OpenAPI` spec as a JSON string.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn generate_openapi_json() -> Result<String, serde_json::Error> {
    let doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&doc)
}
