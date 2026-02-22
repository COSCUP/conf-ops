mod common;

use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::task::repository::{CreateTaskParams, TaskRepository};
use conf_ops::modules::email::repository::EmailMessageRepository;
use conf_ops::modules::email::service::{EmailOutboundService, SendEmailParams};
use std::sync::Arc;
use uuid::Uuid;

async fn setup_outbound_context() -> (common::TestContext, Uuid, Uuid) {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "Email Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Test Tag").await;
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
            name: "Test Email Task",
            description: None,
            created_by: account_id,
        },
    )
    .await
    .expect("should create task");

    (ctx, project_id, task_id)
}

#[tokio::test]
async fn send_email_creates_thread_and_message() {
    let (ctx, _, task_id) = setup_outbound_context().await;
    let event_bus = EventBus::default();

    let outbound = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus,
        ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let result = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Test Subject".to_string(),
            html_body: "<p>Hello</p>".to_string(),
        })
        .await
        .expect("should send email");

    assert_eq!(result.subject, "Test Subject");
    assert_eq!(result.direction, "outbound");
    assert_eq!(result.send_status, "sent");

    // Verify MockEmailService was called via send_with_headers
    let sent = ctx.email_service.sent_with_headers.lock().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "user@example.com");
    assert_eq!(sent[0].1, "Test Subject");
    drop(sent);
}

#[tokio::test]
async fn send_email_reply_extends_thread() {
    let (ctx, _, task_id) = setup_outbound_context().await;
    let event_bus = EventBus::default();

    let outbound = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus,
        ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    // First email creates a thread
    let first = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Thread Test".to_string(),
            html_body: "<p>First</p>".to_string(),
        })
        .await
        .expect("should send first email");

    // Reply to the same thread
    let reply = outbound
        .send_email(SendEmailParams {
            thread_id: Some(first.thread_id),
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Re: Thread Test".to_string(),
            html_body: "<p>Reply</p>".to_string(),
        })
        .await
        .expect("should send reply");

    // Verify In-Reply-To is set
    assert_eq!(reply.thread_id, first.thread_id);
    assert_eq!(reply.in_reply_to.as_deref(), Some(&*first.message_id));

    // Verify send_with_headers was called with correct headers
    let sent = ctx.email_service.sent_with_headers.lock().await;
    assert_eq!(sent.len(), 2);
    let reply_in_reply_to = sent[1].3.in_reply_to.clone();
    let reply_references = sent[1].3.references.clone();
    drop(sent);
    assert_eq!(reply_in_reply_to.as_deref(), Some(&*first.message_id));
    assert!(reply_references.is_some());
}

#[tokio::test]
async fn send_email_smtp_failure_records_failed() {
    let (ctx, _, task_id) = setup_outbound_context().await;
    let event_bus = EventBus::default();

    let failing_service = Arc::new(common::FailingEmailService);

    let outbound = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus,
        failing_service,
        "conf-ops.dev".to_string(),
    );

    let result = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Fail Test".to_string(),
            html_body: "<p>Should fail</p>".to_string(),
        })
        .await
        .expect("should record even on SMTP failure");

    assert_eq!(result.send_status, "failed");
}

#[tokio::test]
async fn retry_failed_emails_processes() {
    let (ctx, _, task_id) = setup_outbound_context().await;
    let event_bus = EventBus::default();

    // First send with a failing service to create a failed message
    let failing_service = Arc::new(common::FailingEmailService);
    let outbound_fail = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus.clone(),
        failing_service,
        "conf-ops.dev".to_string(),
    );

    let failed_msg = outbound_fail
        .send_email(SendEmailParams {
            thread_id: None,
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Retry Test".to_string(),
            html_body: "<p>Retry me</p>".to_string(),
        })
        .await
        .expect("should record failed email");

    assert_eq!(failed_msg.send_status, "failed");

    // Move next_retry_at to the past so retry picks it up immediately
    sqlx::query(
        "UPDATE email_messages SET next_retry_at = NOW() - INTERVAL '1 minute' WHERE id = $1",
    )
    .bind(failed_msg.id)
    .execute(&ctx.pool)
    .await
    .expect("should update next_retry_at");

    // Now retry with a working service
    let outbound_ok = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus,
        ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let retried = outbound_ok
        .retry_failed_emails()
        .await
        .expect("should retry");

    assert_eq!(retried, 1);

    // Verify the message is now sent
    let msg = EmailMessageRepository::find_by_message_id(&ctx.pool, &failed_msg.message_id)
        .await
        .expect("should find message")
        .expect("message should exist");
    assert_eq!(msg.send_status, "sent");
    assert_eq!(msg.retry_count, 1);
}

#[tokio::test]
async fn send_email_headers_passed_to_smtp() {
    let (ctx, _, task_id) = setup_outbound_context().await;
    let event_bus = EventBus::default();

    let outbound = EmailOutboundService::new(
        ctx.pool.clone(),
        event_bus,
        ctx.email_service.clone(),
        "conf-ops.dev".to_string(),
    );

    let result = outbound
        .send_email(SendEmailParams {
            thread_id: None,
            task_id,
            from_address: "noreply@conf-ops.dev".to_string(),
            to_addresses: vec!["user@example.com".to_string()],
            cc_addresses: vec![],
            subject: "Header Test".to_string(),
            html_body: "<p>Check headers</p>".to_string(),
        })
        .await
        .expect("should send");

    let sent = ctx.email_service.sent_with_headers.lock().await;
    assert_eq!(sent.len(), 1);
    let headers_msg_id = sent[0].3.message_id.clone();
    drop(sent);

    // The message_id should match what was generated
    assert!(headers_msg_id.is_some());
    assert_eq!(headers_msg_id.as_deref(), Some(&*result.message_id));
}
