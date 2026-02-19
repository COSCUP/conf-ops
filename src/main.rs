use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use conf_ops::api::routes::health;
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::db;
use conf_ops::events::EventBus;
use conf_ops::modules::auth::jwt::JwtConfig;
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

    let auth_service = Arc::new(AuthService::new(
        pool.clone(),
        jwt_config.clone(),
        email_service,
        &config,
    ));

    let state = AppState {
        pool,
        event_bus,
        jwt_config,
        app_base_url: config.app_base_url.clone(),
        auth_service,
    };

    let app = Router::new()
        .route("/healthz", axum::routing::get(health::healthz))
        .route("/readyz", axum::routing::get(health::readyz))
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
