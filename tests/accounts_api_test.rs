mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::{delete, get};
use axum::Router;
use common::TestContext;
use tower::ServiceExt;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::accounts;

fn build_accounts_app(state: conf_ops::app_state::AppState) -> Router {
    Router::new()
        .route(
            "/accounts/me",
            get(accounts::get_me).patch(accounts::update_me),
        )
        .route(
            "/accounts/me/profile",
            get(accounts::get_profile).put(accounts::update_profile),
        )
        .route("/accounts/me/passkeys", get(accounts::list_passkeys))
        .route(
            "/accounts/me/passkeys/{id}",
            delete(accounts::delete_passkey),
        )
        .route(
            "/accounts/me/notification-preferences",
            get(accounts::get_notification_preferences)
                .put(accounts::update_notification_preferences),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn get_me_requires_auth() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let app = build_accounts_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/accounts/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_me_returns_account() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, email) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let app = build_accounts_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/accounts/me")
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
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email"], email);
    assert_eq!(json["id"], account_id.to_string());
}

#[tokio::test]
async fn update_me_changes_display_name() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let app = build_accounts_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/accounts/me")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"display_name":"Updated Name"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["display_name"], "Updated Name");
}

#[tokio::test]
async fn profile_crud() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    // Get empty profile
    let app = build_accounts_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/accounts/me/profile")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Update profile
    let app = build_accounts_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/accounts/me/profile")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"profile":{"bio":"Hello"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["profile"]["bio"], "Hello");
}

#[tokio::test]
async fn notification_preferences_stub() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let app = build_accounts_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/accounts/me/notification-preferences")
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
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email_notifications"], true);
}

#[tokio::test]
async fn list_passkeys_empty() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let app = build_accounts_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/accounts/me/passkeys")
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
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}
