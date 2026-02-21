mod common;

use axum::http::StatusCode;
use serde_json::json;

use conf_ops::modules::core::member::models::MemberRole;

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
    let token = common::TestContext::issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = common::build_app(ctx);

    let (status, body) = common::json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
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
    let app = common::build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "First Todo", "description": "Do something"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["title"], "First Todo");
    assert_eq!(body["description"], "Do something");
    assert_eq!(body["status"], "open");
    assert_eq!(body["todoType"], "ad_hoc");

    // Get todo
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{base}/{todo_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "First Todo");

    // List todos
    let (status, body) = common::json_request(app.clone(), "GET", &base, &setup.token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["todos"].as_array().unwrap().len(), 1);

    // Update todo
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{base}/{todo_id}"),
        &setup.token,
        Some(json!({"title": "Updated Todo"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Updated Todo");

    // Delete todo
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("{base}/{todo_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify deleted — list should be empty
    let (status, body) = common::json_request(app.clone(), "GET", &base, &setup.token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["todos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn todo_status_and_completion() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "Complete Me"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();

    // Complete todo
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{base}/{todo_id}/status"),
        &setup.token,
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert!(body["completedAt"].as_str().is_some());
    assert!(body["completedBy"].as_str().is_some());

    // Reopen todo
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{base}/{todo_id}/status"),
        &setup.token,
        Some(json!({"status": "open"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "open");
    assert!(body["completedAt"].is_null());
    assert!(body["completedBy"].is_null());
}

#[tokio::test]
async fn todo_nesting_limit() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create parent todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "Parent"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = body["id"].as_str().unwrap().to_string();

    // Create child todo (should succeed)
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "Child", "parentId": parent_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let child_id = body["id"].as_str().unwrap().to_string();

    // Try to create grandchild (should fail — nesting limit exceeded)
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "Grandchild", "parentId": child_id})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn todo_assignees() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &base,
        &setup.token,
        Some(json!({"title": "Assignee Test"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();

    // Add assignee
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &format!("{base}/{todo_id}/assignees"),
        &setup.token,
        Some(json!({"memberId": setup.member_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["memberId"], setup.member_id);

    // Remove assignee
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("{base}/{todo_id}/assignees/{}", setup.member_id),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn task_cannot_complete_with_incomplete_todos() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let todo_base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );
    let task_status_url = format!(
        "/api/v1/projects/{}/tasks/{}/status",
        setup.project_id, setup.task_id
    );

    // Create a todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &todo_base,
        &setup.token,
        Some(json!({"title": "Blocker Todo"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();

    // Try to complete task — should fail (incomplete todos)
    let (status, _) = common::json_request(
        app.clone(),
        "PUT",
        &task_status_url,
        &setup.token,
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Complete the todo
    let (status, _) = common::json_request(
        app.clone(),
        "PUT",
        &format!("{todo_base}/{todo_id}/status"),
        &setup.token,
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Now complete task — should succeed
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &task_status_url,
        &setup.token,
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
}

#[tokio::test]
async fn my_todos_cross_project() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let todo_base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create a todo and assign it to the member
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &todo_base,
        &setup.token,
        Some(json!({"title": "My assigned todo"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();

    // Assign to member
    let (status, _) = common::json_request(
        app.clone(),
        "POST",
        &format!("{todo_base}/{todo_id}/assignees"),
        &setup.token,
        Some(json!({"memberId": setup.member_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // List my todos — should include the assigned todo
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        "/api/v1/accounts/me/todos",
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "My assigned todo");
    assert!(items[0]["projectName"].as_str().is_some());
    assert!(items[0]["taskName"].as_str().is_some());

    // Filter by status — open should return 1
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        "/api/v1/accounts/me/todos?status=open",
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    // Filter by status — completed should return 0
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        "/api/v1/accounts/me/todos?status=completed",
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn todo_linked_task() {
    let ctx = common::TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let app = common::build_app(&ctx);

    let todo_base = format!(
        "/api/v1/projects/{}/tasks/{}/todos",
        setup.project_id, setup.task_id
    );

    // Create a todo
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &todo_base,
        &setup.token,
        Some(json!({"title": "Link Test Todo"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let todo_id = body["id"].as_str().unwrap().to_string();
    assert!(body["linkedTaskId"].is_null());

    // Create a second task to link to
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
    let app2 = common::build_app(&ctx);
    let (status, body2) = common::json_request(
        app2,
        "POST",
        &format!("/api/v1/projects/{project_id2}/tasks"),
        &token2,
        Some(json!({
            "taskTemplateId": template_id2,
            "ownerTagId": tag_id2,
            "name": "Linked Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let linked_task_id = body2["id"].as_str().unwrap().to_string();

    // Link task to todo
    let linked_url = format!("{todo_base}/{todo_id}/linked-task");
    let (status, body) = common::json_request(
        app.clone(),
        "PUT",
        &linked_url,
        &setup.token,
        Some(json!({"linkedTaskId": linked_task_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["linkedTaskId"], linked_task_id);

    // Verify via GET
    let (status, body) = common::json_request(
        app.clone(),
        "GET",
        &format!("{todo_base}/{todo_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["linkedTaskId"], linked_task_id);

    // Unlink task
    let (status, body) =
        common::json_request(app.clone(), "DELETE", &linked_url, &setup.token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["linkedTaskId"].is_null());
}
