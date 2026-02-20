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
    use conf_ops::api::routes::{member_tags, members, organizations, projects, task_templates};

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
        )
        .route(
            "/{templateId}/todo-templates",
            get(task_templates::list_todo_templates).post(task_templates::create_todo_template),
        )
        .route(
            "/{templateId}/todo-templates/reorder",
            put(task_templates::reorder_todo_templates),
        )
        .route(
            "/{templateId}/todo-templates/{todoTemplateId}",
            get(task_templates::get_todo_template)
                .put(task_templates::update_todo_template)
                .delete(task_templates::delete_todo_template),
        )
        .route(
            "/{templateId}/data-schemas",
            get(task_templates::list_data_schemas).post(task_templates::create_data_schema),
        )
        .route(
            "/{templateId}/data-schemas/{schemaId}",
            get(task_templates::get_data_schema)
                .put(task_templates::update_data_schema)
                .delete(task_templates::delete_data_schema),
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
        .nest("/{projectId}/task-templates", task_template_routes);

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
async fn task_template_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);
    let app = build_app(&ctx);

    // Create
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/task-templates"))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"name": "Speaker Invitation", "description": "Invite speakers"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let template_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Speaker Invitation");
    assert_eq!(body["description"], "Invite speakers");
    assert_eq!(body["createdBy"], account_id.to_string());

    // List
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/task-templates"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["templates"].as_array().unwrap().len(), 1);

    // Get
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Speaker Invitation");

    // Update
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"name": "Updated Template"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Updated Template");

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // List after delete - should be empty
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{project_id}/task-templates"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["templates"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn tag_linking() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Speakers").await;
    let template_id = ctx
        .create_test_task_template(project_id, "Speaker Template", account_id)
        .await;
    let token = ctx.issue_test_token(account_id);
    let app = build_app(&ctx);

    // Link tag
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/tags"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"memberTagId": tag_id.to_string()}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Link again - should conflict
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/tags"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"memberTagId": tag_id.to_string()}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // List tags
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/tags"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["tags"].as_array().unwrap().len(), 1);

    // Unlink tag
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/tags/{tag_id}"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn todo_template_crud_and_nesting() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let token = ctx.issue_test_token(account_id);
    let app = build_app(&ctx);

    // Create parent todo template
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/todo-templates"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"name": "Parent Todo", "sortOrder": 1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let parent_id = body["id"].as_str().unwrap().to_string();
    assert!(body["parentId"].is_null());

    // Create child todo template (valid nesting)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/todo-templates"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"name": "Child Todo", "parentId": parent_id, "sortOrder": 1})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let child_body = body_json(resp).await;
    let child_id = child_body["id"].as_str().unwrap().to_string();

    // Create grandchild (should fail - nesting limit)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/todo-templates"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"name": "Grandchild", "parentId": child_id, "sortOrder": 1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // List
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/todo-templates"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["todoTemplates"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn data_schema_crud_and_field_validation() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let token = ctx.issue_test_token(account_id);
    let app = build_app(&ctx);

    // Create with valid fields
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Sponsor Info",
                        "fields": [
                            {
                                "key": "company_name",
                                "label": "Company Name",
                                "description": "Sponsor company name",
                                "type": "single_line_text",
                                "required": true,
                                "constraints": {"maxLength": 200}
                            },
                            {
                                "key": "level",
                                "label": "Level",
                                "description": "Sponsorship level",
                                "type": "select",
                                "required": true,
                                "constraints": {"options": ["gold", "silver", "bronze"]}
                            },
                            {
                                "key": "amount",
                                "label": "Amount",
                                "description": "Sponsorship amount",
                                "type": "number",
                                "required": false,
                                "constraints": {"min": 0, "decimal": true}
                            },
                            {
                                "key": "confirmed",
                                "label": "Confirmed",
                                "description": "Is confirmed",
                                "type": "boolean",
                                "required": false
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let schema_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Sponsor Info");
    assert_eq!(body["fields"].as_array().unwrap().len(), 4);

    // Create with duplicate keys - should fail
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Bad Schema",
                        "fields": [
                            {"key": "dup", "label": "A", "description": "A", "type": "email", "required": false},
                            {"key": "dup", "label": "B", "description": "B", "type": "email", "required": false}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Create with invalid constraints - select without options
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Bad Select",
                        "fields": [
                            {"key": "s", "label": "S", "description": "S", "type": "select", "required": false}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Create with wrong constraint type - maxLength on number
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas"
                ))
                .header("Authorization", auth_header(&token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Bad Constraint",
                        "fields": [
                            {"key": "n", "label": "N", "description": "N", "type": "number", "required": false, "constraints": {"maxLength": 100}}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Get
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas/{schema_id}"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["dataSchemas"].as_array().unwrap().len(), 1);

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas/{schema_id}"
                ))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
