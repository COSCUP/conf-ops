mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use common::{build_app, json_request, TestContext};
use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::conversation::crdt::CrdtManager;
use conf_ops::modules::conversation::models::MessageSourceType;
use conf_ops::modules::conversation::service::ConversationService;
use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::storage::local::LocalStorageBackend;
use conf_ops::modules::storage::service::{FileService, StorageConfig};

async fn setup_task(ctx: &TestContext) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, String) {
    let (account_id, _email) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Test Template", account_id)
        .await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let token = common::TestContext::issue_test_token(account_id);
    let app = build_app(ctx);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(serde_json::json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().parse::<uuid::Uuid>().unwrap();

    (project_id, task_id, account_id, token)
}

#[tokio::test]
async fn send_and_get_conversation() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Send a message
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
        &token,
        Some(serde_json::json!({
            "content": {"text": "Hello, world!", "mentions": []}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["content"]["text"], "Hello, world!");
    let msg_id = body["id"].as_str().unwrap();

    // Get conversation
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    assert_eq!(body["messages"][0]["id"], msg_id);
    assert!(!body["pagination"]["hasMore"].as_bool().unwrap());
}

#[tokio::test]
async fn stale_conversation_returns_409() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Send first message
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
        &token,
        Some(serde_json::json!({
            "content": {"text": "First message", "mentions": []}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let first_msg_id = body["id"].as_str().unwrap().to_string();

    // Send second message
    let app = build_app(&ctx);
    let (status, _body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
        &token,
        Some(serde_json::json!({
            "content": {"text": "Second message", "mentions": []}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Try to send a third message with stale lastSeenMessageId (pointing to first)
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
        &token,
        Some(serde_json::json!({
            "content": {"text": "Third message", "mentions": []},
            "lastSeenMessageId": first_msg_id
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.get("latestMessageId").is_some());
    assert!(body.get("unseenCount").is_some());
}

#[tokio::test]
async fn cursor_pagination() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Send 5 messages
    for i in 0..5 {
        let app = build_app(&ctx);
        let (status, _) = json_request(
            app,
            "POST",
            &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
            &token,
            Some(serde_json::json!({
                "content": {"text": format!("Message {i}"), "mentions": []}
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // Get first page (limit=2)
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation?limit=2"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert!(body["pagination"]["hasMore"].as_bool().unwrap());
    let next_cursor = body["pagination"]["nextCursor"].as_str().unwrap();

    // Get second page
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!(
            "/api/v1/projects/{project_id}/tasks/{task_id}/conversation?limit=2&cursor={next_cursor}"
        ),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert!(body["pagination"]["hasMore"].as_bool().unwrap());
}

#[tokio::test]
async fn update_and_get_last_seen() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Send a message
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/messages"),
        &token,
        Some(serde_json::json!({
            "content": {"text": "Hello", "mentions": []}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let msg_id = body["id"].as_str().unwrap();

    // Update last seen
    let app = build_app(&ctx);
    let (status, _) = json_request(
        app,
        "PUT",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen"),
        &token,
        Some(serde_json::json!({ "messageId": msg_id })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Get last seen
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["lastReadMessageId"], msg_id);
}

#[tokio::test]
async fn unauthorized_user_gets_403() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    // Create a different user with no project membership
    let (other_account_id, _) = ctx.create_test_account().await;
    let org_id =
        conf_ops::modules::core::project::repository::ProjectRepository::get_organization_id(
            &ctx.pool, project_id,
        )
        .await
        .unwrap();
    let _other_org_member_id = {
        let id = generate_id();
        conf_ops::modules::core::organization::repository::OrgMemberRepository::create(
            &ctx.pool,
            id,
            org_id,
            other_account_id,
            conf_ops::modules::core::organization::models::OrgRole::OrgMember,
        )
        .await
        .unwrap();
        id
    };
    let other_token = common::TestContext::issue_test_token(other_account_id);

    // Try to access conversation - should get 403
    let app = build_app(&ctx);
    let (status, _) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation"),
        &other_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn system_message_via_service() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Add a system message directly through the service
    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    let system_msg = conversation_service
        .add_system_message(
            task_id,
            serde_json::json!({
                "event": "task_status_changed",
                "details": {"from": "open", "to": "in_progress"}
            }),
        )
        .await
        .expect("should add system message");

    assert_eq!(system_msg.source_type, MessageSourceType::System);
    assert!(system_msg.source_id.is_none());
    assert_eq!(system_msg.task_id, task_id);

    // Verify the system message appears in the conversation via HTTP
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["id"], system_msg.id.to_string());
    assert_eq!(messages[0]["sourceType"], "system");
    assert!(messages[0]["sourceId"].is_null());
}

#[tokio::test]
async fn last_seen_position_api() {
    let ctx = TestContext::new().await;
    let (project_id, task_id, _account_id, token) = setup_task(&ctx).await;

    // Initially, last-seen-position should return null y_clock
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen-position"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["yClock"].is_null());

    // Update last-seen-position
    let app = build_app(&ctx);
    let (status, _) = json_request(
        app,
        "PUT",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen-position"),
        &token,
        Some(serde_json::json!({ "yClock": 42 })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Get last-seen-position — should reflect the updated value
    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen-position"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["yClock"], 42);

    // Update again with a higher clock value
    let app = build_app(&ctx);
    let (status, _) = json_request(
        app,
        "PUT",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen-position"),
        &token,
        Some(serde_json::json!({ "yClock": 100 })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let app = build_app(&ctx);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/projects/{project_id}/tasks/{task_id}/conversation/last-seen-position"),
        &token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["yClock"], 100);
}

#[tokio::test]
async fn send_message_with_invalid_member_returns_error() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    // Try to send a member message with a non-existent member source_id via service directly
    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    let non_existent_member_id = generate_id();
    let result = conversation_service
        .send_message(
            task_id,
            MessageSourceType::Member,
            Some(non_existent_member_id),
            serde_json::json!({"text": "Hello", "mentions": []}),
            None,
            None,
        )
        .await;

    assert!(
        result.is_err(),
        "Expected error for non-existent member source_id"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            conf_ops::modules::conversation::error::ConversationError::SourceNotFound
        ),
        "Expected SourceNotFound error, got: {err:?}"
    );
}

#[tokio::test]
async fn crdt_cache_eviction_and_db_rebuild() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    // Phase 1: Apply CRDT updates with the first CrdtManager instance
    let crdt_manager1 = Arc::new(CrdtManager::new(ctx.pool.clone()));

    let msg_id1 = generate_id();
    crdt_manager1
        .push_message(task_id, msg_id1, "2025-01-01T00:00:00Z", uuid::Uuid::nil())
        .await
        .expect("push message 1");

    let msg_id2 = generate_id();
    crdt_manager1
        .push_message(task_id, msg_id2, "2025-01-01T00:01:00Z", uuid::Uuid::nil())
        .await
        .expect("push message 2");

    // Record the state vector from the first manager
    let sv_before = crdt_manager1
        .get_state_vector(task_id)
        .await
        .expect("get state vector");

    // Phase 2: Create a NEW CrdtManager (simulates cache eviction / server restart)
    // This forces reconstruction from crdt_operations in the DB
    let crdt_manager2 = Arc::new(CrdtManager::new(ctx.pool.clone()));

    // The rebuilt document should have the same state vector
    let sv_after = crdt_manager2
        .get_state_vector(task_id)
        .await
        .expect("get state vector after rebuild");
    assert_eq!(
        sv_before, sv_after,
        "State vector must match after cache eviction and DB rebuild"
    );

    // Phase 3: Verify we can continue operating on the rebuilt document
    let msg_id3 = generate_id();
    crdt_manager2
        .push_message(task_id, msg_id3, "2025-01-01T00:02:00Z", uuid::Uuid::nil())
        .await
        .expect("push message 3 on rebuilt doc");

    let sv_final = crdt_manager2
        .get_state_vector(task_id)
        .await
        .expect("get final state vector");
    assert_ne!(
        sv_before, sv_final,
        "State vector must change after adding a new message"
    );
}

#[tokio::test]
async fn source_type_ai_suggestion_via_service() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    let source_id = generate_id();
    let result = conversation_service
        .send_message(
            task_id,
            MessageSourceType::AiSuggestion,
            Some(source_id),
            serde_json::json!({"suggestion": "Use async/await here"}),
            None,
            None,
        )
        .await;

    assert!(result.is_ok(), "AiSuggestion message should succeed");
    let msg = result.unwrap();
    assert_eq!(msg.source_type, MessageSourceType::AiSuggestion);
    assert_eq!(msg.source_id, Some(source_id));
}

#[tokio::test]
async fn source_type_tool_execution_via_service() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    let source_id = generate_id();
    let result = conversation_service
        .send_message(
            task_id,
            MessageSourceType::ToolExecution,
            Some(source_id),
            serde_json::json!({
                "toolName": "search",
                "parameters": {"query": "test"},
                "result": {"count": 5},
                "status": "success"
            }),
            None,
            None,
        )
        .await;

    assert!(result.is_ok(), "ToolExecution message should succeed");
    let msg = result.unwrap();
    assert_eq!(msg.source_type, MessageSourceType::ToolExecution);
    assert_eq!(msg.source_id, Some(source_id));
}

#[tokio::test]
async fn source_type_email_inbound_via_service() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    let source_id = generate_id();
    let result = conversation_service
        .send_message(
            task_id,
            MessageSourceType::EmailInbound,
            Some(source_id),
            serde_json::json!({"subject": "Re: Task update", "body": "Looks good!"}),
            None,
            None,
        )
        .await;

    assert!(result.is_ok(), "EmailInbound message should succeed");
    let msg = result.unwrap();
    assert_eq!(msg.source_type, MessageSourceType::EmailInbound);
    assert_eq!(msg.source_id, Some(source_id));
}

#[tokio::test]
async fn source_types_without_source_id_rejected() {
    let ctx = TestContext::new().await;
    let (_project_id, task_id, _account_id, _token) = setup_task(&ctx).await;

    let crdt_manager = Arc::new(CrdtManager::new(ctx.pool.clone()));
    let conversation_service = ConversationService::new(
        ctx.pool.clone(),
        EventBus::default(),
        crdt_manager,
        Arc::new(FileService::new(
            ctx.pool.clone(),
            EventBus::default(),
            Arc::new(LocalStorageBackend::new(&ctx.storage_dir)),
            StorageConfig::default(),
        )),
    );

    // AiSuggestion and ToolExecution require source_id
    for source_type in [
        MessageSourceType::AiSuggestion,
        MessageSourceType::ToolExecution,
    ] {
        let result = conversation_service
            .send_message(
                task_id,
                source_type.clone(),
                None, // no source_id
                serde_json::json!({"text": "test"}),
                None,
                None,
            )
            .await;

        assert!(
            result.is_err(),
            "{source_type:?} without source_id should fail"
        );
        assert!(
            matches!(
                result.unwrap_err(),
                conf_ops::modules::conversation::error::ConversationError::SourceNotFound
            ),
            "{source_type:?} without source_id should return SourceNotFound"
        );
    }

    // EmailInbound allows None source_id (unknown sender)
    let result = conversation_service
        .send_message(
            task_id,
            MessageSourceType::EmailInbound,
            None,
            serde_json::json!({"subject": "Unknown sender", "body": "Hello"}),
            None,
            None,
        )
        .await;
    assert!(
        result.is_ok(),
        "EmailInbound without source_id should succeed (unknown sender)"
    );
}
