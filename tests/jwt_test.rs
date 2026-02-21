mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::get;
use axum::Router;
use common::TestContext;
use tower::ServiceExt;

use conf_ops::api::middleware::auth::{auth_middleware, AuthUser};
use conf_ops::modules::auth::jwt::{
    issue_access_token, issue_refresh_token, rotate_refresh_token, validate_access_token,
};

async fn protected_handler(user: AuthUser) -> String {
    format!("Hello, {}", user.account_id)
}

async fn optional_handler(user: Option<axum::extract::Extension<AuthUser>>) -> String {
    match user {
        Some(ext) => format!("Hello, {}", ext.account_id),
        None => "Anonymous".to_string(),
    }
}

fn build_app_with_middleware(state: conf_ops::app_state::AppState) -> Router {
    Router::new()
        .route("/protected", get(protected_handler))
        .route("/optional", get(optional_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn middleware_sets_auth_user_with_valid_token() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(account_id);

    let app = build_app_with_middleware(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body).contains(&account_id.to_string()));
}

#[tokio::test]
async fn middleware_returns_401_for_missing_token() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_app_with_middleware(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn middleware_returns_401_for_invalid_token() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_app_with_middleware(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("authorization", "Bearer invalid-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_rotation() {
    let ctx = TestContext::new().await;
    let config = TestContext::test_jwt_config();
    let (account_id, _) = ctx.create_test_account().await;

    let raw_refresh = issue_refresh_token(&ctx.pool, &config, account_id)
        .await
        .expect("should issue refresh token");

    let (access_token, new_refresh) = rotate_refresh_token(&ctx.pool, &config, &raw_refresh)
        .await
        .expect("should rotate");

    let claims = validate_access_token(&config, &access_token).expect("should validate");
    assert_eq!(claims.sub, account_id);

    // Old token should be revoked
    let result = rotate_refresh_token(&ctx.pool, &config, &raw_refresh).await;
    assert!(result.is_err());

    // New token should work
    let (access2, _) = rotate_refresh_token(&ctx.pool, &config, &new_refresh)
        .await
        .expect("should rotate new token");
    let claims2 = validate_access_token(&config, &access2).expect("should validate");
    assert_eq!(claims2.sub, account_id);
}

#[tokio::test]
async fn wrong_issuer_fails() {
    let config = TestContext::test_jwt_config();
    let account_id = conf_ops::id::generate_id();
    let token = issue_access_token(&config, account_id).expect("should issue");

    let wrong_issuer_config = conf_ops::modules::auth::jwt::JwtConfig {
        issuer: "wrong-issuer".to_string(),
        ..config
    };
    let result = validate_access_token(&wrong_issuer_config, &token);
    assert!(result.is_err());
}
