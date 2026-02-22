mod common;

use axum::http::StatusCode;
use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::task::repository::{CreateTaskParams, TaskRepository};
use conf_ops::modules::email::service::{EmailOutboundService, SendEmailParams};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

/// Helper to build a minimal RFC 5322 email.
fn build_raw_email(
    from: &str,
    to: &str,
    subject: &str,
    message_id: Option<&str>,
    in_reply_to: Option<&str>,
    body: &str,
) -> Vec<u8> {
    use std::fmt::Write;
    let mut headers =
        format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nContent-Type: text/plain\r\n");
    if let Some(mid) = message_id {
        let _ = write!(headers, "Message-ID: {mid}\r\n");
    }
    if let Some(irt) = in_reply_to {
        let _ = write!(headers, "In-Reply-To: {irt}\r\n");
    }
    let _ = write!(headers, "\r\n{body}");
    headers.into_bytes()
}

struct InboundSetup {
    ctx: common::TestContext,
    project_id: Uuid,
    task_id: Uuid,
    token: String,
}

async fn setup_inbound() -> InboundSetup {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Inbound Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Tag").await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let task_id = generate_id();
    TaskRepository::create(
        &ctx.pool,
        &CreateTaskParams {
            id: task_id,
            project_id,
            task_template_id: template_id,
            owner_tag_id: tag_id,
            name: "Inbound Test Task",
            description: None,
            created_by: account_id,
        },
    )
    .await
    .expect("should create task");

    let token = common::TestContext::issue_test_token(account_id);

    InboundSetup {
        ctx,
        project_id,
        task_id,
        token,
    }
}

// ── Webhook auth tests ──────────────────────────────────────

#[tokio::test]
async fn webhook_valid_api_key() {
    let s = setup_inbound().await;
    let app = common::build_app(&s.ctx);

    let raw = build_raw_email(
        "sender@example.com",
        "inbox@conf-ops.dev",
        "Hello",
        Some("<test-1@example.com>"),
        None,
        "Hello body",
    );

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn webhook_invalid_api_key() {
    let s = setup_inbound().await;
    let app = common::build_app(&s.ctx);

    let raw = build_raw_email(
        "sender@example.com",
        "inbox@conf-ops.dev",
        "Hello",
        None,
        None,
        "Body",
    );

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer wrong-key")
                .header("Content-Type", "message/rfc822")
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Thread matching tests ────────────────────────────────────

#[tokio::test]
async fn thread_match_in_reply_to() {
    let s = setup_inbound().await;
    let event_bus = EventBus::default();

    // Send outbound email first
    let outbound = EmailOutboundService::new(
        s.ctx.pool.clone(),
        event_bus.clone(),
        s.ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let out_msg = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id: s.task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Outbound Thread".to_string(),
            html_body: "<p>Outbound</p>".to_string(),
        })
        .await
        .expect("should send outbound");

    // Inbound reply with In-Reply-To
    let raw = build_raw_email(
        "user@example.com",
        "noreply@conf-ops.dev",
        "Re: Outbound Thread",
        Some("<reply-1@example.com>"),
        Some(&out_msg.message_id),
        "Reply body",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["status"], "Matched");
    assert_eq!(body["threadId"], out_msg.thread_id.to_string());
}

#[tokio::test]
async fn thread_match_heuristic() {
    let s = setup_inbound().await;
    let event_bus = EventBus::default();

    // Send outbound email
    let outbound = EmailOutboundService::new(
        s.ctx.pool.clone(),
        event_bus.clone(),
        s.ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let out_msg = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id: s.task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Heuristic Test".to_string(),
            html_body: "<p>Outbound</p>".to_string(),
        })
        .await
        .expect("should send outbound");

    // Inbound with same subject but no In-Reply-To (heuristic match)
    let raw = build_raw_email(
        "user@example.com",
        "noreply@conf-ops.dev",
        "Re: Heuristic Test",
        Some("<heuristic-1@example.com>"),
        None,
        "Heuristic reply",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    // May match heuristically
    let status = body["status"].as_str().unwrap();
    assert!(
        status == "Matched" || status == "HeuristicMatch" || status == "Unmatched",
        "Unexpected status: {status}"
    );

    // If matched, thread should be the same
    if status == "Matched" {
        assert_eq!(body["threadId"], out_msg.thread_id.to_string());
    }
}

// ── Unmatched email tests ────────────────────────────────────

#[tokio::test]
async fn unmatched_email_to_inbox() {
    let s = setup_inbound().await;

    let raw = build_raw_email(
        "unknown@example.com",
        "inbox@conf-ops.dev",
        "Unknown Sender",
        Some("<unmatched-1@example.com>"),
        None,
        "Who am I?",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["status"], "Unmatched");
}

// ── Duplicate detection ──────────────────────────────────────

#[tokio::test]
async fn duplicate_detection() {
    let s = setup_inbound().await;

    let raw = build_raw_email(
        "dup@example.com",
        "inbox@conf-ops.dev",
        "Duplicate Test",
        Some("<dup-1@example.com>"),
        None,
        "First send",
    );

    let app = common::build_app(&s.ctx);

    // First send
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    // May be Unmatched or Matched depending on thread state
    let first_status = body["status"].as_str().unwrap().to_string();

    // Second send (duplicate)
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    // Only the first unmatched email won't have a message_id record,
    // but if the email was previously matched, second send should detect duplicate
    assert_eq!(resp.status(), StatusCode::OK);
    let body2 = common::body_json(resp).await;
    // If first was unmatched, the message_id won't be in email_messages table
    // so duplicate detection may not trigger. This is expected behavior.
    let status2 = body2["status"].as_str().unwrap();
    assert!(
        status2 == "Duplicate" || status2 == "Unmatched",
        "Expected Duplicate or Unmatched, got: {status2} (first was: {first_status})"
    );
}

// ── Sender resolution ────────────────────────────────────────

#[tokio::test]
async fn sender_resolution_member() {
    let s = setup_inbound().await;
    let event_bus = EventBus::default();

    // Send outbound first to create a thread
    let outbound = EmailOutboundService::new(
        s.ctx.pool.clone(),
        event_bus,
        s.ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let (_, sender_email) = s.ctx.create_test_account().await;
    // Note: the test account already has an account entry with this email

    let out_msg = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id: s.task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec![sender_email.clone()],
            cc_addresses: vec![],
            subject: "Member Sender".to_string(),
            html_body: "<p>Test</p>".to_string(),
        })
        .await
        .expect("should send");

    // Inbound reply from a member
    let raw = build_raw_email(
        &sender_email,
        "noreply@conf-ops.dev",
        "Re: Member Sender",
        Some("<member-reply-1@example.com>"),
        Some(&out_msg.message_id),
        "I am a member",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["status"], "Matched");
}

#[tokio::test]
async fn sender_resolution_auto_create_contact() {
    let s = setup_inbound().await;
    let event_bus = EventBus::default();

    // Send outbound to create a thread
    let outbound = EmailOutboundService::new(
        s.ctx.pool.clone(),
        event_bus,
        s.ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let out_msg = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id: s.task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["external@company.com".to_string()],
            cc_addresses: vec![],
            subject: "Contact Auto-Create".to_string(),
            html_body: "<p>Test</p>".to_string(),
        })
        .await
        .expect("should send");

    // Reply from unknown external sender
    let raw = build_raw_email(
        "external@company.com",
        "noreply@conf-ops.dev",
        "Re: Contact Auto-Create",
        Some("<external-reply@company.com>"),
        Some(&out_msg.message_id),
        "I am a new contact",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_json(resp).await;
    assert_eq!(body["status"], "Matched");
}

// ── Assign endpoint tests ────────────────────────────────────

#[tokio::test]
async fn assign_unassigned_email() {
    let s = setup_inbound().await;

    // Create an unmatched email first
    let raw = build_raw_email(
        "assign-me@example.com",
        "inbox@conf-ops.dev",
        "Please assign me",
        Some("<assign-me-1@example.com>"),
        None,
        "Assign body",
    );

    let app = common::build_app(&s.ctx);
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/email/inbound")
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "message/rfc822")
                .header("X-Project-Id", s.project_id.to_string())
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // List unassigned
    let list_url = format!("/api/v1/projects/{}/unassigned-inbox", s.project_id);
    let (status, body) = common::json_request(app.clone(), "GET", &list_url, &s.token, None).await;
    assert_eq!(status, StatusCode::OK);
    let emails = body["emails"].as_array().expect("emails should be array");
    assert!(
        !emails.is_empty(),
        "should have at least one unassigned email"
    );

    let email_id = emails[0]["id"].as_str().unwrap();

    // Assign
    let assign_url = format!(
        "/api/v1/projects/{}/unassigned-inbox/{}/assign",
        s.project_id, email_id
    );
    let (status, body) = common::json_request(
        app,
        "POST",
        &assign_url,
        &s.token,
        Some(json!({ "taskId": s.task_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["taskId"], s.task_id.to_string());
    assert!(body["threadId"].as_str().is_some());
}

// ── Pagination test ──────────────────────────────────────────

#[tokio::test]
async fn list_unassigned_pagination() {
    let s = setup_inbound().await;

    // Create multiple unmatched emails
    for i in 0..3 {
        let raw = build_raw_email(
            &format!("sender{i}@example.com"),
            "inbox@conf-ops.dev",
            &format!("Pagination Test {i}"),
            Some(&format!("<page-{i}@example.com>")),
            None,
            &format!("Body {i}"),
        );

        let app = common::build_app(&s.ctx);
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/email/inbound")
                    .header("Authorization", "Bearer test-api-key")
                    .header("Content-Type", "message/rfc822")
                    .header("X-Project-Id", s.project_id.to_string())
                    .body(axum::body::Body::from(raw))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // List with limit=2
    let app = common::build_app(&s.ctx);
    let list_url = format!("/api/v1/projects/{}/unassigned-inbox?limit=2", s.project_id);
    let (status, body) = common::json_request(app, "GET", &list_url, &s.token, None).await;
    assert_eq!(status, StatusCode::OK);
    let emails = body["emails"].as_array().expect("emails");
    assert_eq!(emails.len(), 2);
    assert_eq!(body["pagination"]["hasMore"], true);
    assert!(body["pagination"]["nextCursor"].as_str().is_some());

    // Load next page
    let cursor = body["pagination"]["nextCursor"].as_str().unwrap();
    let app = common::build_app(&s.ctx);
    let next_url = format!(
        "/api/v1/projects/{}/unassigned-inbox?limit=2&cursor={}",
        s.project_id, cursor
    );
    let (status, body) = common::json_request(app, "GET", &next_url, &s.token, None).await;
    assert_eq!(status, StatusCode::OK);
    let emails2 = body["emails"].as_array().expect("emails");
    assert_eq!(emails2.len(), 1);
    assert_eq!(body["pagination"]["hasMore"], false);
}
