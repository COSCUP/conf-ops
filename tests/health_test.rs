mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use common::TestContext;
use tower::ServiceExt;

use conf_ops::api::routes::health;
use conf_ops::app_state::AppState;

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", axum::routing::get(health::healthz))
        .route("/readyz", axum::routing::get(health::readyz))
        .with_state(state)
}

#[tokio::test]
async fn healthz_returns_200() {
    let ctx = TestContext::new().await;
    let app = build_app(ctx.app_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
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
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn readyz_returns_200_when_db_is_healthy() {
    let ctx = TestContext::new().await;
    let app = build_app(ctx.app_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
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
    assert_eq!(json["status"], "ok");
    assert_eq!(json["checks"]["database"], "ok");
}

#[tokio::test]
async fn readyz_returns_503_when_db_is_down() {
    let ctx = TestContext::new().await;
    let state = ctx.app_state();
    let pool = state.pool.clone();
    pool.close().await;

    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["checks"]["database"], "error");
}
