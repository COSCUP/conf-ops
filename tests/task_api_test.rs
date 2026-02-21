mod common;

use axum::http::StatusCode;
use serde_json::json;

use conf_ops::modules::core::member::models::MemberRole;

#[tokio::test]
async fn task_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = common::TestContext::issue_test_token(account_id);

    // Create a task template and link a tag
    let template_id = ctx
        .create_test_task_template(project_id, "Sponsorship Contact", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Sponsorship Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = common::build_app(&ctx);
    let base_url = format!("/api/v1/projects/{project_id}/tasks");

    // Create task
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base_url,
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Contact Company A",
            "description": "Reach out to Company A for sponsorship"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["name"], "Contact Company A");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["taskTemplateId"], template_id.to_string());
    assert_eq!(body["ownerTagId"], tag_id.to_string());

    // Get task
    let task_url = format!("{base_url}/{task_id}");
    let (status, body) = common::json_request(app.clone(), "GET", &task_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Contact Company A");

    // List tasks
    let (status, body) = common::json_request(app.clone(), "GET", &base_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tasks"].as_array().unwrap().len(), 1);

    // Update task
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &task_url,
        &token,
        Some(json!({"name": "Contact Company A (Updated)"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Contact Company A (Updated)");

    // Delete task
    let (status, _) = common::json_request(app.clone(), "DELETE", &task_url, &token, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify deleted — list should be empty
    let (status, body) = common::json_request(app.clone(), "GET", &base_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tasks"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn task_status_transitions() {
    let ctx = common::TestContext::new().await;
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

    let app = common::build_app(&ctx);
    let tasks_url = format!("/api/v1/projects/{project_id}/tasks");

    // Create task (starts as pending)
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &tasks_url,
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Status Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["status"], "pending");

    let status_url = format!("{tasks_url}/{task_id}/status");

    // Transition: pending -> in_progress
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &status_url,
        &token,
        Some(json!({"status": "in_progress"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "in_progress");

    // Transition: in_progress -> completed
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &status_url,
        &token,
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");

    // Invalid transition: completed -> in_progress (should fail)
    let (status, _) = common::json_request(
        app.clone(),
        "PUT",
        &status_url,
        &token,
        Some(json!({"status": "in_progress"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn task_cancel_from_pending() {
    let ctx = common::TestContext::new().await;
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

    let app = common::build_app(&ctx);
    let tasks_url = format!("/api/v1/projects/{project_id}/tasks");

    // Create task
    let (_, body) = common::json_request(
        app.clone(),
        "POST",
        &tasks_url,
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Cancel Test Task"
        })),
    )
    .await;
    let task_id = body["id"].as_str().unwrap().to_string();

    // Transition: pending -> cancelled
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{tasks_url}/{task_id}/status"),
        &token,
        Some(json!({"status": "cancelled"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
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
    let token = common::TestContext::issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    // Create a tag but DON'T link it to the template
    let unlinked_tag_id = ctx.create_test_tag(project_id, "Unlinked Team").await;

    let app = common::build_app(&ctx);

    // Try to create task with unlinked tag — should fail
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": unlinked_tag_id,
            "name": "Should Fail"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_tasks_with_filters() {
    let ctx = common::TestContext::new().await;
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
    let tag_a = ctx.create_test_tag(project_id, "Team A").await;
    let tag_b = ctx.create_test_tag(project_id, "Team B").await;
    ctx.link_tag_to_template(template_id, tag_a).await;
    ctx.link_tag_to_template(template_id, tag_b).await;
    ctx.assign_tag_to_member(tag_a, member_id, project_id).await;
    ctx.assign_tag_to_member(tag_b, member_id, project_id).await;

    let app = common::build_app(&ctx);
    let tasks_url = format!("/api/v1/projects/{project_id}/tasks");

    // Create tasks with different tags
    for (name, tag_id) in [("Task A", tag_a), ("Task B", tag_b)] {
        let (status, _) = common::json_request(
            app.clone(),
            "POST",
            &tasks_url,
            &token,
            Some(json!({
                "taskTemplateId": template_id,
                "ownerTagId": tag_id,
                "name": name
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // List all — should have 2
    let (_, body) = common::json_request(app.clone(), "GET", &tasks_url, &token, None).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);

    // Filter by tagId — should have 1
    let (_, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{tasks_url}?tagId={tag_a}"),
        &token,
        None,
    )
    .await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(body["tasks"][0]["name"], "Task A");

    // Filter by status=pending — should have 2
    let (_, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{tasks_url}?status=pending"),
        &token,
        None,
    )
    .await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);

    // Filter by status=in_progress — should have 0
    let (_, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{tasks_url}?status=in_progress"),
        &token,
        None,
    )
    .await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn task_participants() {
    let ctx = common::TestContext::new().await;
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

    // Assign owner to the tag so they appear as owner_tag participant
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = common::build_app(&ctx);

    // Create task
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Participant Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().to_string();

    // Get participants — should include at least the creator's account
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/participants"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let participants = body["participants"].as_array().unwrap();
    assert!(!participants.is_empty(), "participants should not be empty");
    // Creator account should be in participants
    assert!(
        participants
            .iter()
            .any(|p| p.as_str() == Some(&account_id.to_string())),
        "creator should be in participants"
    );
}
