mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use common::TestContext;
use tower::ServiceExt;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::auth;

fn build_auth_app(state: conf_ops::app_state::AppState) -> Router {
    Router::new()
        .route("/auth/magic-link/request", post(auth::request_magic_link))
        .route("/auth/magic-link/verify", get(auth::verify_magic_link))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/passkey/login/begin", post(auth::passkey_login_begin))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

async fn extract_token_from_email(ctx: &TestContext) -> String {
    let sent = ctx.email_service.sent.lock().await;
    let body = &sent[sent.len() - 1].2;
    body.split("token=")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn full_magic_link_login_flow() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state.clone());

    // Request magic link
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/magic-link/request")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"flow@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify magic link
    let token = extract_token_from_email(&ctx).await;
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/auth/magic-link/verify?token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["accessToken"].is_string());
    assert_eq!(json["tokenType"], "Bearer");
}

#[tokio::test]
async fn logout_requires_auth() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_with_auth_succeeds() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn passkey_login_begin_returns_challenge() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/passkey/login/begin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["challenge_id"].is_string());
}

#[tokio::test]
async fn logout_invalidates_refresh_token() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();

    // 1. Login via magic link to get a real refresh token cookie
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/magic-link/request")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"logout-test@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let token = extract_token_from_email(&ctx).await;
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/auth/magic-link/verify?token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Extract refresh_token cookie and access token from login response
    let refresh_cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .find(|v| v.to_str().unwrap().starts_with("refresh_token="))
        .expect("should have refresh_token cookie")
        .to_str()
        .unwrap()
        .to_string();
    let refresh_token_value = refresh_cookie
        .split('=')
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["accessToken"].as_str().unwrap();

    // 2. Verify refresh works before logout
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("cookie", format!("refresh_token={refresh_token_value}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Logout
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 4. Refresh with the old cookie should fail after logout
    let app = build_auth_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("cookie", format!("refresh_token={refresh_token_value}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_without_cookie_returns_401() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_auth_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
