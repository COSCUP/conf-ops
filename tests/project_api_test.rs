mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn build_app(ctx: &common::TestContext) -> Router {
    use axum::routing::{get, post, put};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{organizations, projects};

    let state = ctx.app_state();

    let org_routes = Router::new().route(
        "/",
        post(organizations::create_organization).get(organizations::list_organizations),
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
async fn create_project_returns_201() {
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
                .uri(format!("/api/v1/organizations/{}/projects", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "My Project",
                        "description": "Test project"
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
    assert_eq!(json["name"], "My Project");
    assert_eq!(json["status"], "preparing");
}

#[tokio::test]
async fn get_project() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let project = state
        .project_service
        .create_project(org.id, "Test Project", None, owner_id)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/projects/{}", project.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn copy_project_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let source = state
        .project_service
        .create_project(org.id, "Source Project", Some("Desc"), owner_id)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/organizations/{}/projects/copy", org.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "sourceProjectId": source.id,
                        "name": "Copied Project",
                        "description": "Copied desc"
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
    assert_eq!(json["name"], "Copied Project");
    assert_eq!(json["status"], "preparing");
    assert_eq!(json["sourceProjectId"], source.id.to_string());
}

#[tokio::test]
async fn update_project_status() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let project = state
        .project_service
        .create_project(org.id, "Test", None, owner_id)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/projects/{}/status", project.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "status": "active"
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
    assert_eq!(json["status"], "active");
}

#[tokio::test]
async fn invalid_status_transition_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let project = state
        .project_service
        .create_project(org.id, "Test", None, owner_id)
        .await
        .unwrap();

    // Trying to go from preparing -> completed (invalid)
    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/projects/{}/status", project.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "status": "completed"
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
async fn delete_project_returns_204() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = ctx.issue_test_token(owner_id);

    let state = ctx.app_state();
    let org = state
        .org_service
        .create_organization(owner_id, "Test Org", None, None)
        .await
        .unwrap();

    let project = state
        .project_service
        .create_project(org.id, "To Delete", None, owner_id)
        .await
        .unwrap();

    let app = build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{}", project.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
