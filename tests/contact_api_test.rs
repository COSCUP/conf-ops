mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn build_app(ctx: &common::TestContext) -> Router {
    use axum::routing::{get, post};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{contacts, organizations};

    let state = ctx.app_state();

    let org_routes = Router::new().route(
        "/",
        post(organizations::create_organization).get(organizations::list_organizations),
    );

    let contact_routes = Router::new()
        .route(
            "/",
            post(contacts::create_contact).get(contacts::list_contacts),
        )
        .route("/merge", post(contacts::merge_contacts))
        .route(
            "/{contactId}",
            get(contacts::get_contact)
                .put(contacts::update_contact)
                .delete(contacts::delete_contact),
        );

    Router::new()
        .nest("/api/v1/organizations", org_routes)
        .nest("/api/v1/organizations/{orgId}/contacts", contact_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn create_contact_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{}/contacts", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "Alice",
                        "email": "alice@example.com"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Alice");
    assert_eq!(json["email"], "alice@example.com");
    assert!(json["id"].is_string());
}

#[tokio::test]
async fn list_contacts_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    state
        .contact_service
        .create_contact(org.id, "Alice", "alice@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/organizations/{}/contacts", org.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["contacts"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn list_contacts_with_search() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    state
        .contact_service
        .create_contact(org.id, "Alice", "alice@example.com")
        .await
        .unwrap();
    state
        .contact_service
        .create_contact(org.id, "Bob", "bob@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/organizations/{}/contacts?search=alice",
                    org.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["contacts"].as_array().unwrap().len(), 1);
    assert_eq!(json["contacts"][0]["name"], "Alice");
}

#[tokio::test]
async fn get_contact_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let contact = state
        .contact_service
        .create_contact(org.id, "Alice", "alice@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/organizations/{}/contacts/{}",
                    org.id, contact.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Alice");
}

#[tokio::test]
async fn update_contact_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let contact = state
        .contact_service
        .create_contact(org.id, "Alice", "alice@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/organizations/{}/contacts/{}",
                    org.id, contact.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "Alice Updated",
                        "email": "alice-new@example.com"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Alice Updated");
    assert_eq!(json["email"], "alice-new@example.com");
}

#[tokio::test]
async fn delete_contact_returns_204() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let contact = state
        .contact_service
        .create_contact(org.id, "Alice", "alice@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/organizations/{}/contacts/{}",
                    org.id, contact.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn merge_contacts_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let target = state
        .contact_service
        .create_contact(org.id, "Target", "target@example.com")
        .await
        .unwrap();

    let source1 = state
        .contact_service
        .create_contact(org.id, "Source1", "source1@example.com")
        .await
        .unwrap();

    let source2 = state
        .contact_service
        .create_contact(org.id, "Source2", "source2@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{}/contacts/merge", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "sourceIds": [source1.id, source2.id],
                        "targetId": target.id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["mergedCount"], 2);
}

#[tokio::test]
async fn merge_source_contains_target_returns_400() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let target = state
        .contact_service
        .create_contact(org.id, "Target", "target@example.com")
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{}/contacts/merge", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "sourceIds": [target.id],
                        "targetId": target.id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn merge_already_merged_target_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let final_target = state
        .contact_service
        .create_contact(org.id, "FinalTarget", "final@example.com")
        .await
        .unwrap();

    let merged_target = state
        .contact_service
        .create_contact(org.id, "MergedTarget", "merged@example.com")
        .await
        .unwrap();

    let source = state
        .contact_service
        .create_contact(org.id, "Source", "source@example.com")
        .await
        .unwrap();

    // First merge merged_target into final_target
    state
        .contact_service
        .merge_contacts(org.id, vec![merged_target.id], final_target.id)
        .await
        .unwrap();

    // Now try to merge source into merged_target (which is already merged)
    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{}/contacts/merge", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "sourceIds": [source.id],
                        "targetId": merged_target.id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn get_merged_contact_follows_chain() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let target = state
        .contact_service
        .create_contact(org.id, "Target", "target@example.com")
        .await
        .unwrap();

    let source = state
        .contact_service
        .create_contact(org.id, "Source", "source@example.com")
        .await
        .unwrap();

    state
        .contact_service
        .merge_contacts(org.id, vec![source.id], target.id)
        .await
        .unwrap();

    // Getting the source contact should return the target
    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/organizations/{}/contacts/{}",
                    org.id, source.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Target");
    assert_eq!(json["id"], target.id.to_string());
}
