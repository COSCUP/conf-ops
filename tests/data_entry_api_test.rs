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
        data_entries, member_tags, members, organizations, projects, task_templates, tasks, todos,
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
        .route("/{taskId}/data-entries", get(data_entries::list_entries))
        .route(
            "/{taskId}/data-entries/{schemaId}",
            get(data_entries::get_entry)
                .put(data_entries::upsert_entry)
                .delete(data_entries::delete_entry),
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
        .nest("/{projectId}/tasks", task_routes)
        .route(
            "/{projectId}/task-templates/{templateId}/data-sheets/{schemaId}",
            get(data_entries::get_aggregated_sheet),
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

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

struct DataEntrySetup {
    project_id: String,
    task_id: String,
    template_id: String,
    schema_id: String,
    token: String,
}

async fn setup_with_schema(ctx: &common::TestContext) -> DataEntrySetup {
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

    // Create a data schema on the template
    let schema_id = ctx
        .create_test_data_schema(
            template_id,
            "Contact Info",
            &json!([
                {
                    "key": "name",
                    "label": "Name",
                    "description": "Full name",
                    "type": "single_line_text",
                    "required": true
                },
                {
                    "key": "age",
                    "label": "Age",
                    "description": "Age",
                    "type": "number",
                    "required": false,
                    "constraints": {"min": 0.0, "max": 200.0}
                },
                {
                    "key": "active",
                    "label": "Active",
                    "description": "Is active",
                    "type": "boolean",
                    "required": false
                }
            ]),
        )
        .await;

    // Create a task
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

    DataEntrySetup {
        project_id: project_id.to_string(),
        task_id,
        template_id: template_id.to_string(),
        schema_id: schema_id.to_string(),
        token,
    }
}

#[tokio::test]
async fn data_entry_upsert_and_get() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = build_app(&ctx);

    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );

    // Upsert data entry
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "values": {"name": "Alice", "age": 30}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["values"]["name"], "Alice");
    assert_eq!(body["values"]["age"], 30);
    assert_eq!(body["dataSchemaId"], setup.schema_id);

    // Get data entry
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["values"]["name"], "Alice");

    // Upsert again — should merge values
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "values": {"active": true}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    // Original values should still be present
    assert_eq!(body["values"]["name"], "Alice");
    assert_eq!(body["values"]["age"], 30);
    // New value should be added
    assert_eq!(body["values"]["active"], true);
}

#[tokio::test]
async fn data_entry_list_and_delete() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = build_app(&ctx);

    let list_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries",
        setup.project_id, setup.task_id
    );
    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );

    // List — should be empty initially
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&list_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 0);

    // Upsert an entry
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"values": {"name": "Bob"}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List — should have 1 entry
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&list_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 1);

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // List — should be empty again
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&list_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn data_entry_validation_rejects_invalid_values() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = build_app(&ctx);

    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );

    // Wrong type — name expects string, sending number
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"values": {"name": 123}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Wrong type — age expects number, sending string
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"values": {"age": "not a number"}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Number constraint — age exceeds max (200)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"values": {"age": 300}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Unknown field key
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"values": {"nonexistent": "value"}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Boolean type — active expects boolean, sending string
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"values": {"active": "yes"}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn data_sheet_aggregation() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = build_app(&ctx);

    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );
    let aggregated_url = format!(
        "/api/v1/projects/{}/task-templates/{}/data-sheets/{}",
        setup.project_id, setup.template_id, setup.schema_id
    );

    // Upsert data
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"values": {"name": "Charlie"}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Get aggregated data sheet
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&aggregated_url)
                .header("Authorization", auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["values"]["name"], "Charlie");
}
