mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use conf_ops::id::generate_id;
use conf_ops::modules::core::organization::models::OrgRole;
use conf_ops::modules::core::organization::repository::OrgMemberRepository;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn build_app(ctx: &common::TestContext) -> Router {
    use axum::routing::{get, post, put};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{organizations, projects};

    let state = ctx.app_state();

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

    let project_nested = Router::new().route(
        "/",
        post(projects::create_project).get(projects::list_projects),
    );

    Router::new()
        .nest("/api/v1/organizations", org_routes)
        .nest("/api/v1/organizations/{orgId}/projects", project_nested)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn unauthenticated_returns_401() {
    let ctx = common::TestContext::new().await;
    let app = build_app(&ctx);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_organization_returns_201() {
    let ctx = common::TestContext::new().await;
    let app = build_app(&ctx);
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/organizations")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "My Org",
                        "description": "Test org"
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
    assert_eq!(json["name"], "My Org");
}

#[tokio::test]
async fn list_organizations() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(account_id);

    // Create org via service
    let state = ctx.app_state();
    state
        .org_service
        .create_organization(account_id, "Org 1", None, None)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/organizations")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["organizations"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn invite_existing_account_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (_, invitee_email) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

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
                .uri(format!("/api/v1/organizations/{}/members/invite", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": invitee_email,
                        "role": "org_member"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn invite_new_email_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

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
                .uri(format!("/api/v1/organizations/{}/members/invite", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "newuser@example.com",
                        "role": "org_member"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn member_forbidden_from_creating_project() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (member_id, _) = ctx.create_test_account().await;
    let member_token = ctx.issue_test_token(member_id);

    let org_id = ctx.create_test_org(owner_id).await;

    // Add member as org_member
    OrgMemberRepository::create(
        &ctx.pool,
        generate_id(),
        org_id,
        member_id,
        OrgRole::OrgMember,
    )
    .await
    .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{org_id}/projects"))
                .header("authorization", format!("Bearer {member_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "Should Fail"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_org_with_active_projects_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Org With Projects", None, None)
        .await
        .unwrap();

    state
        .project_service
        .create_project(org.id, "Active Project", None, owner_id)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/organizations/{}", org.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn remove_last_owner_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let members = state.org_service.list_members(org.id).await.unwrap();
    let owner_member = &members[0];

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/organizations/{}/members/{}",
                    org.id, owner_member.id
                ))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}
