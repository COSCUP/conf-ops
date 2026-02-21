mod common;

use axum::http::StatusCode;
use serde_json::json;

use conf_ops::modules::core::member::models::MemberRole;

#[tokio::test]
async fn task_template_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates");

    // Create
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"name": "Speaker Invitation", "description": "Invite speakers"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let template_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Speaker Invitation");
    assert_eq!(body["description"], "Invite speakers");
    assert_eq!(body["createdBy"], account_id.to_string());

    // List
    let (status, body) = common::json_request(app.clone(), "GET", &base, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["templates"].as_array().unwrap().len(), 1);

    // Get
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{base}/{template_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Speaker Invitation");

    // Update
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{base}/{template_id}"),
        &token,
        Some(json!({"name": "Updated Template"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Updated Template");

    // Delete
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("{base}/{template_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // List after delete - should be empty
    let (status, body) = common::json_request(app.clone(), "GET", &base, &token, None).await;
    assert_eq!(status, StatusCode::OK);
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
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates/{template_id}/tags");

    // Link tag
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"memberTagId": tag_id.to_string()})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Link again - should conflict
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"memberTagId": tag_id.to_string()})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // List tags
    let (status, body) = common::json_request(app.clone(), "GET", &base, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tags"].as_array().unwrap().len(), 1);

    // Unlink tag
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("{base}/{tag_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
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
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates/{template_id}/todo-templates");

    // Create parent todo template
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"name": "Parent Todo", "sortOrder": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = body["id"].as_str().unwrap().to_string();
    assert!(body["parentId"].is_null());

    // Create child todo template (valid nesting)
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"name": "Child Todo", "parentId": parent_id, "sortOrder": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let child_id = body["id"].as_str().unwrap().to_string();

    // Create grandchild (should fail - nesting limit)
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({"name": "Grandchild", "parentId": child_id, "sortOrder": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // List
    let (status, body) = common::json_request(app.clone(), "GET", &base, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["todoTemplates"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn data_schema_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas");

    // Create with valid fields
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({
            "name": "Sponsor Info",
            "fields": [
                {
                    "key": "company_name", "label": "Company Name",
                    "description": "Sponsor company name",
                    "type": "single_line_text", "required": true,
                    "constraints": {"maxLength": 200}
                },
                {
                    "key": "level", "label": "Level",
                    "description": "Sponsorship level",
                    "type": "select", "required": true,
                    "constraints": {"options": ["gold", "silver", "bronze"]}
                },
                {
                    "key": "amount", "label": "Amount",
                    "description": "Sponsorship amount",
                    "type": "number", "required": false,
                    "constraints": {"min": 0, "decimal": true}
                },
                {
                    "key": "confirmed", "label": "Confirmed",
                    "description": "Is confirmed",
                    "type": "boolean", "required": false
                }
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let schema_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Sponsor Info");
    assert_eq!(body["fields"].as_array().unwrap().len(), 4);

    // Get
    let (status, _) = common::json_request(
        app.clone(),
        "GET",
        &format!("{base}/{schema_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // List
    let (status, body) = common::json_request(app.clone(), "GET", &base, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["dataSchemas"].as_array().unwrap().len(), 1);

    // Delete
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("{base}/{schema_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn data_schema_field_validation() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas");

    // Duplicate keys - should fail
    let (status, _) = common::json_request(
        app.clone(), "POST", &base, &token,
        Some(json!({
            "name": "Bad Schema",
            "fields": [
                {"key": "dup", "label": "A", "description": "A", "type": "email", "required": false},
                {"key": "dup", "label": "B", "description": "B", "type": "email", "required": false}
            ]
        })),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Invalid constraints - select without options
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({
            "name": "Bad Select",
            "fields": [
                {"key": "s", "label": "S", "description": "S", "type": "select", "required": false}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Wrong constraint type - maxLength on number
    let (status, _) = common::json_request(
        app.clone(), "POST", &base, &token,
        Some(json!({
            "name": "Bad Constraint",
            "fields": [
                {"key": "n", "label": "N", "description": "N", "type": "number", "required": false, "constraints": {"maxLength": 100}}
            ]
        })),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Validates that all 10 field types can be created successfully.
#[tokio::test]
async fn data_schema_all_10_field_types() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    ctx.create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(&ctx);
    let base = format!("/api/v1/projects/{project_id}/task-templates/{template_id}/data-schemas");

    // Create a schema with all 10 field types
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &token,
        Some(json!({
            "name": "All Types",
            "fields": [
                {"key": "f1", "label": "Single Line", "description": "text", "type": "single_line_text", "required": true, "constraints": {"maxLength": 200}},
                {"key": "f2", "label": "Multi Line", "description": "text", "type": "multi_line_text", "required": false, "constraints": {"maxLength": 5000}},
                {"key": "f3", "label": "Number", "description": "num", "type": "number", "required": false, "constraints": {"min": 0.0, "max": 999.0, "decimal": true}},
                {"key": "f4", "label": "Date", "description": "date", "type": "date", "required": false},
                {"key": "f5", "label": "Email", "description": "email", "type": "email", "required": true},
                {"key": "f6", "label": "Url", "description": "url", "type": "url", "required": false},
                {"key": "f7", "label": "Select", "description": "sel", "type": "select", "required": false, "constraints": {"options": ["a", "b", "c"]}},
                {"key": "f8", "label": "Boolean", "description": "bool", "type": "boolean", "required": false},
                {"key": "f9", "label": "Image", "description": "img", "type": "image", "required": false},
                {"key": "f10", "label": "File", "description": "file", "type": "file", "required": false}
            ]
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "all 10 field types should be accepted"
    );
    let schema_id = body["id"].as_str().unwrap().to_string();

    // Verify round-trip: get the schema and check fields
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{base}/{schema_id}"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let fields = body["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 10);

    // Verify each field type is preserved
    let types: Vec<&str> = fields.iter().map(|f| f["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"single_line_text"));
    assert!(types.contains(&"multi_line_text"));
    assert!(types.contains(&"number"));
    assert!(types.contains(&"date"));
    assert!(types.contains(&"email"));
    assert!(types.contains(&"url"));
    assert!(types.contains(&"select"));
    assert!(types.contains(&"boolean"));
    assert!(types.contains(&"image"));
    assert!(types.contains(&"file"));
}
