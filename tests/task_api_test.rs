mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use conf_ops::modules::core::member::models::MemberRole;

fn build_app(ctx: &common::TestContext) -> Router {
    use axum::routing::{delete, get, post, put};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{
        member_tags, members, organizations, projects, task_templates, tasks,
    };

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

    let task_template_routes = Router::new()
        .route(
            "/",
            get(task_templates::list_templates).post(task_templates::create_template),
        )
        .route(
            "/{templateId}",
            get(task_templates::get_template)
                .put(task_templates::update_template)
                .delete(task_templates::delete_template),
        )
        .route(
            "/{templateId}/tags",
            get(task_templates::list_template_tags).post(task_templates::link_tag),
        )
        .route(
            "/{templateId}/tags/{memberTagId}",
            delete(task_templates::unlink_tag),
        );

    let task_routes = Router::new()
        .route("/", get(tasks::list_tasks).post(tasks::create_task))
        .route(
            "/{taskId}",
            get(tasks::get_task)
                .put(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route("/{taskId}/status", put(tasks::update_task_status));

    let project_top = Router::new()
        .route(
            "/{projectId}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        .nest("/{projectId}/members", member_routes)
        .nest("/{projectId}/member-tags", member_tag_routes)
        .nest("/{projectId}/task-templates", task_template_routes)
        .nest("/{projectId}/tasks", task_routes);

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

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn task_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    // Create a task template and link a tag
    let template_id = ctx
        .create_test_task_template(project_id, "Sponsorship Contact", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Sponsorship Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let app = build_app(&ctx);

    // Create task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "taskTemplateId": template_id,
                        "ownerTagId": tag_id,
                        "name": "Contact Company A",
                        "description": "Reach out to Company A for sponsorship"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let task_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Contact Company A");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["taskTemplateId"], template_id.to_string());
    assert_eq!(body["ownerTagId"], tag_id.to_string());

    // Get task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/tasks/{task_id}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Contact Company A");

    // List tasks
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 1);

    // Update task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/projects/{project_id}/tasks/{task_id}"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"name": "Contact Company A (Updated)"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Contact Company A (Updated)");

    // Delete task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{project_id}/tasks/{task_id}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted — list should be empty
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn task_status_transitions() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let app = build_app(&ctx);

    // Create task (starts as pending)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "taskTemplateId": template_id,
                        "ownerTagId": tag_id,
                        "name": "Status Test Task"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let task_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["status"], "pending");

    // Transition: pending → in_progress
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks/{task_id}/status"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "in_progress"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "in_progress");

    // Transition: in_progress → completed
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks/{task_id}/status"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "completed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "completed");

    // Invalid transition: completed → in_progress (should fail)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks/{task_id}/status"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "in_progress"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn task_cancel_from_pending() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let app = build_app(&ctx);

    // Create task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "taskTemplateId": template_id,
                        "ownerTagId": tag_id,
                        "name": "Cancel Test Task"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    let task_id = body["id"].as_str().unwrap().to_string();

    // Transition: pending → cancelled
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks/{task_id}/status"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "cancelled"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "cancelled");
}

#[tokio::test]
async fn task_invalid_owner_tag_returns_400() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    // Create a tag but DON'T link it to the template
    let unlinked_tag_id = ctx.create_test_tag(project_id, "Unlinked Team").await;

    let app = build_app(&ctx);

    // Try to create task with unlinked tag — should fail
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "taskTemplateId": template_id,
                        "ownerTagId": unlinked_tag_id,
                        "name": "Should Fail"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_tasks_with_filters() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_a = ctx.create_test_tag(project_id, "Team A").await;
    let tag_b = ctx.create_test_tag(project_id, "Team B").await;
    ctx.link_tag_to_template(template_id, tag_a).await;
    ctx.link_tag_to_template(template_id, tag_b).await;

    let app = build_app(&ctx);

    // Create tasks with different tags
    for (name, tag_id) in [("Task A", tag_a), ("Task B", tag_b)] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/projects/{project_id}/tasks"))
                    .header("Authorization", auth_header(&token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "taskTemplateId": template_id,
                            "ownerTagId": tag_id,
                            "name": name
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List all — should have 2
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);

    // Filter by tagId — should have 1
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/tasks?tagId={tag_a}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(body["tasks"][0]["name"], "Task A");

    // Filter by status=pending — should have 2
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks?status=pending"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);

    // Filter by status=in_progress — should have 0
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/tasks?status=in_progress"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 0);
}
