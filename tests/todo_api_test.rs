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
        member_tags, members, organizations, projects, task_templates, tasks, todos,
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
        .route("/{taskId}/status", put(tasks::update_task_status))
        .route(
            "/{taskId}/todos",
            get(todos::list_todos).post(todos::create_todo),
        )
        .route(
            "/{taskId}/todos/{todoId}",
            get(todos::get_todo)
                .put(todos::update_todo)
                .delete(todos::delete_todo),
        )
        .route(
            "/{taskId}/todos/{todoId}/status",
            put(todos::update_todo_status),
        )
        .route(
            "/{taskId}/todos/{todoId}/assignees",
            post(todos::add_assignee),
        )
        .route(
            "/{taskId}/todos/{todoId}/assignees/{memberId}",
            delete(todos::remove_assignee),
        );

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

struct TaskSetup {
    project_id: String,
    task_id: String,
    token: String,
    member_id: String,
}

async fn setup_task(ctx: &common::TestContext) -> TaskSetup {
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let app = build_app(ctx);

    let resp = app
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
                        "name": "Test Task"
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

    TaskSetup {
        project_id: project_id.to_string(),
        task_id,
        token,
        member_id: member_id.to_string(),
    }
}

#[tokio::test]
async fn todo_crud() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "First Todo",
                        "description": "Do something"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let todo_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["title"], "First Todo");
    assert_eq!(body["description"], "Do something");
    assert_eq!(body["status"], "open");
    assert_eq!(body["todoType"], "ad_hoc");

    // Get todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{base}/{todo_id}"))
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["title"], "First Todo");

    // List todos
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["todos"].as_array().unwrap().len(), 1);

    // Update todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("{base}/{todo_id}"))
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"title": "Updated Todo"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["title"], "Updated Todo");

    // Delete todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("{base}/{todo_id}"))
                .header("Authorization", auth_header(&setup.token))
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
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["todos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn todo_status_and_completion() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"title": "Complete Me"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let todo_id = body["id"].as_str().unwrap().to_string();

    // Complete todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("{base}/{todo_id}/status"))
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "completed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "completed");
    assert!(body["completedAt"].as_str().is_some());
    assert!(body["completedBy"].as_str().is_some());

    // Reopen todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("{base}/{todo_id}/status"))
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "open"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "open");
    assert!(body["completedAt"].is_null());
    assert!(body["completedBy"].is_null());
}

#[tokio::test]
async fn todo_nesting_limit() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create parent todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"title": "Parent"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let parent_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    // Create child todo (should succeed)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"title": "Child", "parentId": parent_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let child_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    // Try to create grandchild (should fail — nesting limit exceeded)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"title": "Grandchild", "parentId": child_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn todo_assignees() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"title": "Assignee Test"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let todo_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    // Add assignee
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{base}/{todo_id}/assignees"))
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"memberId": setup.member_id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["memberId"], setup.member_id);

    // Remove assignee
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("{base}/{todo_id}/assignees/{}", setup.member_id))
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn task_cannot_complete_with_incomplete_todos() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = build_app(&ctx);

    let todo_base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );
    let task_status_url = format!(
        "/api/v1/projects/{}/tasks/{}/status",
        setup.project_id, setup.task_id
    );

    // Create a todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&todo_base)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"title": "Blocker Todo"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let todo_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    // Try to complete task — should fail (incomplete todos)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&task_status_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "completed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Complete the todo
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("{todo_base}/{todo_id}/status"))
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "completed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Now complete task — should succeed
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&task_status_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"status": "completed"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "completed");
}
