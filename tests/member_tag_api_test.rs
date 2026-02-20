mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use conf_ops::modules::core::member::models::MemberRole;

fn build_app(ctx: &common::TestContext) -> Router {
    use axum::routing::{delete, get, post, put};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{member_tags, members, organizations, projects};

    let state = ctx.app_state();

    let org_routes = Router::new().route(
        "/",
        post(organizations::create_organization).get(organizations::list_organizations),
    );

    let project_nested = Router::new().route(
        "/",
        post(projects::create_project).get(projects::list_projects),
    );

    let member_routes = Router::new()
        .route("/", get(members::list_members))
        .route("/invite", post(members::invite_member))
        .route(
            "/{memberId}",
            get(members::get_member)
                .put(members::update_member)
                .delete(members::delete_member),
        );

    let member_tag_routes = Router::new()
        .route(
            "/",
            get(member_tags::list_tags).post(member_tags::create_tag),
        )
        .route(
            "/{tagId}",
            get(member_tags::get_tag)
                .put(member_tags::update_tag)
                .delete(member_tags::delete_tag),
        )
        .route("/{tagId}/assign", post(member_tags::assign_tag))
        .route(
            "/{tagId}/assignments/{assignmentId}",
            delete(member_tags::remove_assignment),
        )
        .route(
            "/{tagId}/external-task-creation",
            put(member_tags::update_external_task_creation),
        );

    let project_top = Router::new()
        .route(
            "/{projectId}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        .nest("/{projectId}/members", member_routes)
        .nest("/{projectId}/member-tags", member_tag_routes);

    Router::new()
        .nest("/api/v1/organizations", org_routes)
        .nest("/api/v1/organizations/{orgId}/projects", project_nested)
        .nest("/api/v1/projects", project_top)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn create_tag_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/member-tags"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "speakers",
                        "description": "Conference speakers"
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
    assert_eq!(json["name"], "speakers");
    assert_eq!(json["description"], "Conference speakers");
}

#[tokio::test]
async fn list_tags_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    ctx.create_test_tag(project_id, "tag1").await;
    ctx.create_test_tag(project_id, "tag2").await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/projects/{project_id}/member-tags"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tags"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_tag_detail_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "detail-tag").await;

    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let contact_id = ctx
        .create_test_contact(org_id, "Contact", "contact@test.com")
        .await;
    ctx.assign_tag_to_contact(tag_id, contact_id, project_id)
        .await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}"
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
    assert_eq!(json["tag"]["name"], "detail-tag");
    assert_eq!(json["members"].as_array().unwrap().len(), 1);
    assert_eq!(json["contacts"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn update_tag_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "old-name").await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "new-name"
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
    assert_eq!(json["name"], "new-name");
}

#[tokio::test]
async fn delete_tag_returns_204() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "to-delete").await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}"
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
async fn assign_member_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "assign-tag").await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assign"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "memberId": member_id
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
    assert_eq!(json["memberId"], member_id.to_string());
    assert!(json["contactId"].is_null());
}

#[tokio::test]
async fn assign_contact_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "assign-tag").await;
    let contact_id = ctx
        .create_test_contact(org_id, "Contact", "c@test.com")
        .await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assign"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "contactId": contact_id
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
    assert_eq!(json["contactId"], contact_id.to_string());
    assert!(json["memberId"].is_null());
}

#[tokio::test]
async fn remove_assignment_returns_204() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "remove-tag").await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;
    let assignment_id = ctx
        .assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assignments/{assignment_id}"
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
async fn duplicate_assignment_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "dup-tag").await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assign"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "memberId": member_id
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
async fn invalid_xor_assignment_returns_400() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "xor-tag").await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;
    let contact_id = ctx
        .create_test_contact(org_id, "Contact", "c@test.com")
        .await;

    // Both provided
    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assign"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "memberId": member_id,
                        "contactId": contact_id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Neither provided
    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/assign"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_external_task_creation_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);
    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let tag_id = ctx.create_test_tag(project_id, "ext-tag").await;

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/member-tags/{tag_id}/external-task-creation"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "settings": [{"type": "github", "repo": "org/repo"}]
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
    assert_eq!(
        json["externalTaskCreation"],
        serde_json::json!([{"type": "github", "repo": "org/repo"}])
    );
}
