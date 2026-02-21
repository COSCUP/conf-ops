mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use conf_ops::modules::core::member::models::MemberRole;

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
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = common::TestContext::issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

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
    let app = common::build_app(ctx);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/tasks"))
                .header("Authorization", common::auth_header(&token))
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
    let body = common::body_json(resp).await;
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
    let app = common::build_app(&ctx);

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
                .header("Authorization", common::auth_header(&setup.token))
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
    let body = common::body_json(resp).await;
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
                .header("Authorization", common::auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["values"]["name"], "Alice");

    // Upsert again — should merge values
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", common::auth_header(&setup.token))
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
    let body = common::body_json(resp).await;
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
    let app = common::build_app(&ctx);

    let list_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries",
        setup.project_id, setup.task_id
    );
    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );

    // List — should have 1 auto-created empty entry from template instantiation
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&list_url)
                .header("Authorization", common::auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 1);

    // Upsert an entry
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 1);

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&entry_url)
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn data_entry_validation_rejects_invalid_values() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = common::build_app(&ctx);

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
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
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
    let app = common::build_app(&ctx);

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
                .header("Authorization", common::auth_header(&setup.token))
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
                .header("Authorization", common::auth_header(&setup.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["values"]["name"], "Charlie");
}

#[tokio::test]
async fn data_share_to_task() {
    let ctx = common::TestContext::new().await;
    let setup = setup_with_schema(&ctx).await;
    let app = common::build_app(&ctx);

    let entry_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}",
        setup.project_id, setup.task_id, setup.schema_id
    );

    // Upsert source data
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&entry_url)
                .header("Authorization", common::auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"values": {"name": "Shared Name", "age": 25}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Create a second task from same template
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id2 = ctx.create_test_project(org_id, account_id).await;
    let member_id2 = ctx
        .create_test_member(project_id2, account_id, MemberRole::Owner)
        .await;
    let token2 = common::TestContext::issue_test_token(account_id);
    let template_id2 = ctx
        .create_test_task_template(project_id2, "Template2", account_id)
        .await;
    let tag_id2 = ctx.create_test_tag(project_id2, "Team2").await;
    ctx.link_tag_to_template(template_id2, tag_id2).await;
    ctx.assign_tag_to_member(tag_id2, member_id2, project_id2)
        .await;
    let target_schema_id = ctx
        .create_test_data_schema(
            template_id2,
            "Target Schema",
            &json!([
                {
                    "key": "full_name",
                    "label": "Full Name",
                    "description": "Name",
                    "type": "single_line_text",
                    "required": false
                }
            ]),
        )
        .await;

    let app2 = common::build_app(&ctx);
    let resp = app2
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id2}/tasks"))
                .header("Authorization", common::auth_header(&token2))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "taskTemplateId": template_id2,
                        "ownerTagId": tag_id2,
                        "name": "Target Task"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = common::body_json(resp).await;
    let target_task_id = body["id"].as_str().unwrap().to_string();

    // Share data
    let share_url = format!(
        "/api/v1/projects/{}/tasks/{}/data-entries/{}/share",
        setup.project_id, setup.task_id, setup.schema_id
    );
    let app3 = common::build_app(&ctx);
    let resp = app3
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&share_url)
                .header("Authorization", common::auth_header(&setup.token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "targetTaskId": target_task_id,
                        "targetSchemaId": target_schema_id,
                        "fieldMappings": [
                            {"sourceKey": "name", "targetKey": "full_name"}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["values"]["full_name"], "Shared Name");
    // Source link should be recorded
    assert!(body["sourceLinks"].is_array() || body["sourceLinks"].is_object());
}
