mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::post;
use axum::Router;
use common::TestContext;
use tower::ServiceExt;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::auth;
use conf_ops::modules::auth::jwt::validate_access_token;
use conf_ops::modules::auth::repository::AccountRepository;

fn build_auth_app(state: conf_ops::app_state::AppState) -> Router {
    Router::new()
        .route("/auth/magic-link/request", post(auth::request_magic_link))
        .route("/auth/magic-link/verify", post(auth::verify_magic_link))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

async fn extract_token_from_email(ctx: &TestContext) -> String {
    let body = {
        let sent = ctx.email_service.sent.lock().await;
        sent.last().unwrap().2.clone()
    };
    body.split("token=")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn request_magic_link_sends_email() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/magic-link/request")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"newuser@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    {
        let sent = ctx.email_service.sent.lock().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "newuser@example.com");
        assert!(sent[0].2.contains("magic-link"));
    }
}

#[tokio::test]
async fn verify_magic_link_creates_account_and_returns_tokens() {
    let ctx = TestContext::new().await;
    let jwt_config = TestContext::test_jwt_config();
    let state = ctx.app_state();

    state
        .auth_service
        .request_magic_link("verify@example.com")
        .await
        .expect("should send magic link");

    let token = extract_token_from_email(&ctx).await;

    let (access_token, _refresh_token, account_id) = state
        .auth_service
        .verify_magic_link(&token)
        .await
        .expect("should verify magic link");

    let claims =
        validate_access_token(&jwt_config, &access_token).expect("should validate access token");
    assert_eq!(claims.sub, account_id);

    let account = AccountRepository::get_by_id(&ctx.pool, account_id)
        .await
        .expect("should find account");
    assert_eq!(account.email, "verify@example.com");
}

#[tokio::test]
async fn verify_magic_link_with_existing_account() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, email) = ctx.create_test_account().await;

    state
        .auth_service
        .request_magic_link(&email)
        .await
        .expect("should send magic link");

    let token = extract_token_from_email(&ctx).await;

    let (_access_token, _refresh_token, returned_id) = state
        .auth_service
        .verify_magic_link(&token)
        .await
        .expect("should verify");

    assert_eq!(returned_id, account_id);
}

#[tokio::test]
async fn verify_magic_link_token_cannot_be_reused() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();

    state
        .auth_service
        .request_magic_link("reuse@example.com")
        .await
        .expect("should send");

    let token = extract_token_from_email(&ctx).await;

    state
        .auth_service
        .verify_magic_link(&token)
        .await
        .expect("first verify should succeed");

    let result = state.auth_service.verify_magic_link(&token).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn verify_invalid_token_fails() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();

    let result = state.auth_service.verify_magic_link("bogus-token").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn consistent_response_for_unknown_email() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/magic-link/request")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"nonexistent@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
