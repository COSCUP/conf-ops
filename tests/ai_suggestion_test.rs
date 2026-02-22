mod common;

use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use conf_ops::modules::ai::models::{
    AiSuggestionContent, Suggestion, SuggestionContextRef, SuggestionDecision, SuggestionGroup,
    TriggerType,
};
use conf_ops::modules::ai::repository::AiSuggestionRepository;
use conf_ops::modules::core::member::models::MemberRole;

/// Helper: create a standard test setup (account, org, project, template, tag, member, task).
/// Returns `(task_id_str, project_id, account_id, token)`.
async fn setup_task(ctx: &common::TestContext) -> (String, Uuid, Uuid, String) {
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = common::TestContext::issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "AI Test Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "AI Team").await;
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
            "name": "AI Suggestion Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().to_string();

    (task_id, project_id, account_id, token)
}

/// Helper: insert a suggestion message into the DB and return `(message_id, group_id, suggestion_id)`.
async fn insert_suggestion(ctx: &common::TestContext, task_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let group_id = Uuid::now_v7();
    let suggestion_id = Uuid::now_v7();

    let content = AiSuggestionContent {
        suggestion_group: SuggestionGroup {
            id: group_id,
            trigger: TriggerType::ManualRequest,
            suggestions: vec![Suggestion {
                id: suggestion_id,
                summary: "Send welcome email".to_string(),
                tool: "email.send".to_string(),
                parameters: json!({"to": "{{profile.email}}", "subject": "Welcome"}),
                reasoning: "New task created".to_string(),
                context_used: vec![SuggestionContextRef {
                    scope_type: "task".to_string(),
                    memory_id: Uuid::now_v7(),
                    content: "Task context".to_string(),
                }],
                decision: SuggestionDecision::Pending,
                decided_by: None,
                decided_at: None,
                modified_parameters: None,
                execution_result: None,
            }],
            created_at: Utc::now(),
        },
    };

    let message_id =
        AiSuggestionRepository::insert_suggestion_message(&ctx.pool, task_id, None, &content)
            .await
            .expect("should insert suggestion message");

    (message_id, group_id, suggestion_id)
}

// ── List Suggestions ─────────────────────────────────────────

#[tokio::test]
async fn list_suggestions_empty() {
    let ctx = common::TestContext::new().await;
    let (task_id, project_id, _, token) = setup_task(&ctx).await;

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions");
    let (status, body) = common::json_request(app, "GET", &url, &token, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert!(body["nextCursor"].is_null());
}

#[tokio::test]
async fn list_suggestions_returns_inserted() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    insert_suggestion(&ctx, task_id).await;
    insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions");
    let (status, body) = common::json_request(app, "GET", &url, &token, None).await;

    assert_eq!(status, StatusCode::OK);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert!(data[0]["messageId"].is_string());
    assert!(data[0]["suggestionGroup"]["id"].is_string());
    assert_eq!(data[0]["suggestionGroup"]["trigger"], "manual_request");
}

#[tokio::test]
async fn list_suggestions_pagination() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    // Insert 3 suggestions
    for _ in 0..3 {
        insert_suggestion(&ctx, task_id).await;
    }

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions?limit=2");
    let (status, body) = common::json_request(app.clone(), "GET", &url, &token, None).await;

    assert_eq!(status, StatusCode::OK);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert!(body["nextCursor"].is_string());

    // Fetch next page using cursor
    let cursor = body["nextCursor"].as_str().unwrap();
    let url2 = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions?limit=2&cursor={cursor}"
    );
    let (status2, body2) =
        common::json_request(common::build_app(&ctx), "GET", &url2, &token, None).await;
    assert_eq!(status2, StatusCode::OK);
    let data2 = body2["data"].as_array().unwrap();
    assert_eq!(data2.len(), 1);
    assert!(body2["nextCursor"].is_null());
}

// ── Get Suggestion Group ─────────────────────────────────────

#[tokio::test]
async fn get_suggestion_group_found() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (_, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}");
    let (status, body) = common::json_request(app, "GET", &url, &token, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["suggestionGroup"]["id"], group_id.to_string());
    assert_eq!(
        body["suggestionGroup"]["suggestions"][0]["id"],
        suggestion_id.to_string()
    );
    assert_eq!(
        body["suggestionGroup"]["suggestions"][0]["decision"],
        "pending"
    );
}

#[tokio::test]
async fn get_suggestion_group_not_found() {
    let ctx = common::TestContext::new().await;
    let (task_id, project_id, _, token) = setup_task(&ctx).await;
    let fake_group_id = Uuid::now_v7();

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{fake_group_id}");
    let (status, _) = common::json_request(app, "GET", &url, &token, None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Decide: Accept ───────────────────────────────────────────

#[tokio::test]
async fn decide_accept() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        &url,
        &token,
        Some(json!({
            "decision": "accept",
            "lastSeenMessageId": message_id.to_string()
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    // Verify the suggestion is now accepted
    let get_url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}");
    let (status, body) =
        common::json_request(common::build_app(&ctx), "GET", &get_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["suggestionGroup"]["suggestions"][0]["decision"],
        "accept"
    );
    assert!(body["suggestionGroup"]["suggestions"][0]["executionResult"].is_object());
}

// ── Decide: Modify and Accept ────────────────────────────────

#[tokio::test]
async fn decide_modify_and_accept() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let modified = json!({"to": "modified@example.com", "subject": "Modified Subject"});
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({
            "decision": "modify_and_accept",
            "lastSeenMessageId": message_id.to_string(),
            "modifiedParameters": modified
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    // Verify
    let get_url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}");
    let (status, body) =
        common::json_request(common::build_app(&ctx), "GET", &get_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    let sg = &body["suggestionGroup"]["suggestions"][0];
    assert_eq!(sg["decision"], "modify_and_accept");
    assert_eq!(sg["modifiedParameters"]["to"], "modified@example.com");
    assert!(sg["executionResult"].is_object());
}

// ── Decide: Reject ───────────────────────────────────────────

#[tokio::test]
async fn decide_reject() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({
            "decision": "reject",
            "lastSeenMessageId": message_id.to_string()
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    // Verify
    let get_url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}");
    let (status, body) =
        common::json_request(common::build_app(&ctx), "GET", &get_url, &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["suggestionGroup"]["suggestions"][0]["decision"],
        "reject"
    );
}

// ── Decide: Re-Suggest ───────────────────────────────────────

#[tokio::test]
async fn decide_re_suggest() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({
            "decision": "re_suggest",
            "lastSeenMessageId": message_id.to_string(),
            "additionalInstructions": "Please also include a meeting link"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    // Verify a new pipeline event was created
    let event: (String, serde_json::Value) = sqlx::query_as(
        r"SELECT trigger_type, payload FROM ai_pipeline_events
         WHERE task_id = $1 AND trigger_type = 'manual_request'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(task_id)
    .fetch_one(&ctx.pool)
    .await
    .expect("should find re-suggest pipeline event");

    assert_eq!(event.0, "manual_request");
    assert_eq!(event.1["re_suggest_for"], suggestion_id.to_string());
    assert_eq!(
        event.1["additional_instructions"],
        "Please also include a meeting link"
    );
}

// ── Decide: Already Decided ──────────────────────────────────

#[tokio::test]
async fn decide_already_decided_returns_conflict() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let decide_body = json!({
        "decision": "accept",
        "lastSeenMessageId": message_id.to_string()
    });

    // First decision succeeds
    let (status, _) =
        common::json_request(app, "POST", &url, &token, Some(decide_body.clone())).await;
    assert_eq!(status, StatusCode::OK);

    // Second decision on same suggestion should fail with 409
    let (status, body) = common::json_request(
        common::build_app(&ctx),
        "POST",
        &url,
        &token,
        Some(decide_body),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["detail"].as_str().unwrap().contains("already decided"));
}

// ── Decide: Stale Conversation (409) ─────────────────────────

#[tokio::test]
async fn decide_stale_conversation_returns_conflict() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, account_id, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let (message_id, group_id, suggestion_id) = insert_suggestion(&ctx, task_id).await;

    // Insert a newer message after the suggestion to make lastSeenMessageId stale
    let newer_message_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO messages (id, task_id, source_type, source_id, content)
         VALUES ($1, $2, 'member', $3, '{"text": "newer message"}'::jsonb)"#,
    )
    .bind(newer_message_id)
    .bind(task_id)
    .bind(account_id)
    .execute(&ctx.pool)
    .await
    .expect("should insert newer message");

    let app = common::build_app(&ctx);
    let url = format!(
        "/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/{group_id}/suggestions/{suggestion_id}/decide"
    );
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({
            "decision": "accept",
            "lastSeenMessageId": message_id.to_string()
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["detail"]
        .as_str()
        .unwrap()
        .contains("Stale conversation"));
}

// ── Request Suggestion ───────────────────────────────────────

#[tokio::test]
async fn request_suggestion_creates_pipeline_event() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/request");
    let (status, body) = common::json_request(app, "POST", &url, &token, Some(json!({}))).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body["eventId"].is_string());

    // Verify the pipeline event exists
    let event_id: Uuid = body["eventId"].as_str().unwrap().parse().unwrap();
    let event: (String, String) =
        sqlx::query_as("SELECT trigger_type, status FROM ai_pipeline_events WHERE id = $1")
            .bind(event_id)
            .fetch_one(&ctx.pool)
            .await
            .expect("should find pipeline event");

    assert_eq!(event.0, "manual_request");
    assert_eq!(event.1, "pending");
}

#[tokio::test]
async fn request_suggestion_with_last_seen_message() {
    let ctx = common::TestContext::new().await;
    let (task_id_str, project_id, _, token) = setup_task(&ctx).await;
    let task_id: Uuid = task_id_str.parse().unwrap();

    let last_seen = Uuid::now_v7();

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/suggestions/request");
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({ "lastSeenMessageId": last_seen.to_string() })),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);

    let event_id: Uuid = body["eventId"].as_str().unwrap().parse().unwrap();
    let (payload,): (serde_json::Value,) =
        sqlx::query_as("SELECT payload FROM ai_pipeline_events WHERE id = $1")
            .bind(event_id)
            .fetch_one(&ctx.pool)
            .await
            .expect("should find pipeline event");

    assert_eq!(payload["last_seen_message_id"], last_seen.to_string());
}

// ── Resolve Placeholders ─────────────────────────────────────

#[tokio::test]
async fn resolve_placeholders_no_placeholders() {
    let ctx = common::TestContext::new().await;
    let (task_id, project_id, _, token) = setup_task(&ctx).await;

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/ai/resolve-placeholders");
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({ "text": "Hello world, no placeholders here" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["text"], "Hello world, no placeholders here");
    assert!(body["unresolved"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn resolve_placeholders_with_unresolved() {
    let ctx = common::TestContext::new().await;
    let (task_id, project_id, _, token) = setup_task(&ctx).await;

    let app = common::build_app(&ctx);
    let url = format!("/api/v1/projects/{project_id}/tasks/{task_id}/ai/resolve-placeholders");
    let (status, body) = common::json_request(
        app,
        "POST",
        &url,
        &token,
        Some(json!({ "text": "Dear {{profile.name}}, code: {{data.code}}" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let unresolved = body["unresolved"].as_array().unwrap();
    // Profile and data keys don't exist, so both should be unresolved
    assert!(!unresolved.is_empty());
}
