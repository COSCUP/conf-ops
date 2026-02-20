use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::{middleware, Router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::{accounts, auth, health, organizations, projects};
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::db;
use conf_ops::events::EventBus;
use conf_ops::modules::auth::jwt::JwtConfig;
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::core::organization::service::OrganizationService;
use conf_ops::modules::core::permission::service::PermissionService;
use conf_ops::modules::core::project::service::ProjectService;
use conf_ops::modules::email::smtp::SmtpEmailService;

fn build_app_state(config: &AppConfig, pool: sqlx::PgPool) -> AppState {
    let event_bus = EventBus::default();
    let jwt_config = JwtConfig {
        secret: config.jwt_secret.clone(),
        issuer: config.jwt_issuer.clone(),
        access_token_expiry_secs: config.jwt_access_expiry_secs,
        refresh_token_expiry_secs: config.jwt_refresh_expiry_secs,
    };

    let email_service =
        Arc::new(SmtpEmailService::new(config).expect("Failed to create email service"));

    let webauthn = build_webauthn(config).expect("Failed to build WebAuthn");

    let auth_service = Arc::new(AuthService::new(
        pool.clone(),
        jwt_config.clone(),
        email_service.clone(),
        webauthn,
        event_bus.clone(),
        config,
    ));

    let org_service = Arc::new(OrganizationService::new(
        pool.clone(),
        event_bus.clone(),
        email_service,
        config.frontend_url.clone(),
    ));

    let project_service = Arc::new(ProjectService::new(pool.clone(), event_bus.clone()));

    let permission_service = Arc::new(PermissionService::new(pool.clone()));

    AppState {
        pool,
        event_bus,
        jwt_config,
        app_base_url: config.app_base_url.clone(),
        auth_service,
        org_service,
        project_service,
        permission_service,
    }
}

fn build_router(state: AppState) -> Router {
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

    let org_routes = Router::new()
        .route(
            "/",
            post(organizations::create_organization).get(organizations::list_organizations),
        )
        .route(
            "/{orgId}",
            get(organizations::get_organization)
                .put(organizations::update_organization)
                .delete(organizations::delete_organization),
        )
        .route("/{orgId}/members", get(organizations::list_members))
        .route(
            "/{orgId}/members/invite",
            post(organizations::invite_member),
        )
        .route(
            "/{orgId}/members/{memberId}",
            put(organizations::update_member_role).delete(organizations::remove_member),
        );

    let project_nested = Router::new()
        .route(
            "/",
            post(projects::create_project).get(projects::list_projects),
        )
        .route("/copy", post(projects::copy_project));

    let project_top = Router::new()
        .route(
            "/{projectId}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        .route("/{projectId}/status", put(projects::update_project_status))
        .route(
            "/{projectId}/permission-settings",
            get(projects::get_permission_settings).put(projects::update_permission_settings),
        );

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/accounts", account_routes)
        .nest("/organizations", org_routes)
        .nest("/organizations/{orgId}/projects", project_nested)
        .nest("/projects", project_top);

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

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

    let state = build_app_state(&config, pool);
    let app = build_router(state);

    let addr: SocketAddr = format!("{}:{}", config.app_host, config.app_port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Starting server on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server error");
}
