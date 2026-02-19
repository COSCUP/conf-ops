use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::{middleware, Router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::{accounts, auth, health};
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::db;
use conf_ops::events::EventBus;
use conf_ops::modules::auth::jwt::JwtConfig;
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::email::smtp::SmtpEmailService;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env().expect("Failed to load configuration");

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.app_log_level))
        .init();

    let pool = db::create_pool(&config)
        .await
        .expect("Failed to create database pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let event_bus = EventBus::default();
    let jwt_config = JwtConfig {
        secret: config.jwt_secret.clone(),
        issuer: config.jwt_issuer.clone(),
        access_token_expiry_secs: config.jwt_access_expiry_secs,
        refresh_token_expiry_secs: config.jwt_refresh_expiry_secs,
    };

    let email_service =
        Arc::new(SmtpEmailService::new(&config).expect("Failed to create email service"));

    let webauthn = build_webauthn(&config).expect("Failed to build WebAuthn");

    let auth_service = Arc::new(AuthService::new(
        pool.clone(),
        jwt_config.clone(),
        email_service,
        webauthn,
        event_bus.clone(),
        &config,
    ));

    let state = AppState {
        pool,
        event_bus,
        jwt_config,
        app_base_url: config.app_base_url.clone(),
        auth_service,
    };

    let auth_routes = Router::new()
        .route("/magic-link/request", post(auth::request_magic_link))
        .route("/magic-link/verify", get(auth::verify_magic_link))
        .route("/refresh", post(auth::refresh))
        .route("/logout", post(auth::logout))
        .route(
            "/passkey/register/begin",
            post(auth::passkey_register_begin),
        )
        .route(
            "/passkey/register/complete",
            post(auth::passkey_register_complete),
        )
        .route("/passkey/login/begin", post(auth::passkey_login_begin))
        .route(
            "/passkey/login/complete",
            post(auth::passkey_login_complete),
        )
        .route("/passkeys", get(accounts::list_passkeys))
        .route("/passkeys/{id}", delete(accounts::delete_passkey));

    let account_routes = Router::new()
        .route("/me", get(accounts::get_me).patch(accounts::update_me))
        .route(
            "/me/profile",
            get(accounts::get_profile).put(accounts::update_profile),
        )
        .route(
            "/me/notification-preferences",
            get(accounts::get_notification_preferences)
                .put(accounts::update_notification_preferences),
        );

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/accounts", account_routes);

    let app = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.app_host, config.app_port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Starting server on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server error");
}
