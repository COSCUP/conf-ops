use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::conversation::models::MessageSourceType;
use crate::modules::conversation::repository::{CreateMessageParams, MessageRepository};

use super::error::EmailError;
use super::mime_parser::{normalize_subject, parse_mime, ParsedEmail};
use super::models::{
    CreateEmailMessageParams, CreateEmailThreadParams, CreateUnassignedEmailParams, EmailDirection,
    SendStatus,
};
use super::repository::{EmailMessageRepository, EmailThreadRepository, UnassignedEmailRepository};
use super::sender_resolver::resolve_sender;
use super::thread_matcher::{match_thread, ThreadMatchResult};

pub struct InboundResult {
    pub status: InboundStatus,
    pub task_id: Option<Uuid>,
    pub thread_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub email_id: Option<Uuid>,
}

#[derive(Debug)]
pub enum InboundStatus {
    Matched,
    Unmatched,
    Duplicate,
}

pub struct InboundEmailService {
    pool: PgPool,
    event_bus: EventBus,
    _file_service: Arc<crate::modules::storage::service::FileService>,
}

impl InboundEmailService {
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        file_service: Arc<crate::modules::storage::service::FileService>,
    ) -> Self {
        Self {
            pool,
            event_bus,
            _file_service: file_service,
        }
    }

    /// Process an inbound email from raw MIME data.
    ///
    /// # Errors
    ///
    /// Returns `EmailError` on parse or database failures.
    pub async fn process_inbound(
        &self,
        raw_mime: &[u8],
        default_project_id: Uuid,
    ) -> Result<InboundResult, EmailError> {
        let parsed = parse_mime(raw_mime)?;

        // Duplicate detection
        if let Some(ref msg_id) = parsed.message_id {
            if EmailMessageRepository::find_by_message_id(&self.pool, msg_id)
                .await?
                .is_some()
            {
                return Ok(InboundResult {
                    status: InboundStatus::Duplicate,
                    task_id: None,
                    thread_id: None,
                    message_id: None,
                    email_id: None,
                });
            }
        }

        // Thread matching
        let match_result = match_thread(&self.pool, &parsed).await?;

        match match_result {
            ThreadMatchResult::ExactMatch(thread) | ThreadMatchResult::HeuristicMatch(thread) => {
                self.process_matched_email(&parsed, &thread, raw_mime).await
            }
            ThreadMatchResult::NoMatch => {
                self.process_unmatched_email(&parsed, raw_mime, default_project_id)
                    .await
            }
        }
    }

    async fn process_matched_email(
        &self,
        parsed: &ParsedEmail,
        thread: &super::models::EmailThread,
        _raw_mime: &[u8],
    ) -> Result<InboundResult, EmailError> {
        let now = Utc::now();

        // Resolve sender identity
        let task = sqlx::query_as!(
            TaskProjectRow,
            r#"SELECT t.id, t.project_id, p.organization_id
             FROM tasks t
             INNER JOIN projects p ON p.id = t.project_id
             WHERE t.id = $1"#,
            thread.task_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(EmailError::Database)?;

        let _sender = resolve_sender(
            &self.pool,
            &parsed.from_address,
            parsed.from_name.as_deref(),
            task.organization_id,
            task.project_id,
        )
        .await?;

        // Create conversation message
        let content = serde_json::json!({
            "text": parsed.text_body.as_deref().unwrap_or(""),
            "from": parsed.from_address,
            "subject": parsed.subject,
        });

        let conv_msg_id = generate_id();
        MessageRepository::create(
            &self.pool,
            &CreateMessageParams {
                id: conv_msg_id,
                task_id: thread.task_id,
                source_type: MessageSourceType::EmailInbound,
                source_id: None,
                content,
                attachments: None,
                action_result: None,
                last_seen_message_id: None,
            },
        )
        .await
        .map_err(|e| {
            EmailError::Database(match e {
                crate::modules::conversation::error::ConversationError::Database(db_err) => db_err,
                _ => sqlx::Error::RowNotFound,
            })
        })?;

        // Create email message record
        let email_msg_id = generate_id();
        let rfc_message_id = parsed
            .message_id
            .clone()
            .unwrap_or_else(|| format!("<{}@inbound>", generate_id()));

        EmailMessageRepository::create(
            &self.pool,
            &CreateEmailMessageParams {
                id: email_msg_id,
                thread_id: thread.id,
                message_id: rfc_message_id.clone(),
                in_reply_to: parsed.in_reply_to.clone(),
                references_header: if parsed.references.is_empty() {
                    None
                } else {
                    Some(parsed.references.join(" "))
                },
                from_address: parsed.from_address.clone(),
                to_addresses: serde_json::json!(parsed.to_addresses),
                cc_addresses: serde_json::json!(parsed.cc_addresses),
                subject: parsed.subject.clone(),
                direction: EmailDirection::Inbound,
                conversation_message_id: Some(conv_msg_id),
                raw_headers: Some(parsed.raw_headers.clone()),
                send_status: SendStatus::Sent,
            },
        )
        .await?;

        self.update_thread_metadata(thread, &rfc_message_id, &parsed.from_address, now)
            .await?;

        self.event_bus.publish(DomainEvent::EmailReceived {
            email_message_id: email_msg_id,
            thread_id: thread.id,
            task_id: thread.task_id,
        });

        Ok(InboundResult {
            status: InboundStatus::Matched,
            task_id: Some(thread.task_id),
            thread_id: Some(thread.id),
            message_id: Some(conv_msg_id),
            email_id: None,
        })
    }

    async fn update_thread_metadata(
        &self,
        thread: &super::models::EmailThread,
        rfc_message_id: &str,
        from_address: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), EmailError> {
        EmailThreadRepository::append_message_id(&self.pool, thread.id, rfc_message_id).await?;
        EmailThreadRepository::update_last_message_at(&self.pool, thread.id, now).await?;

        let mut participants: Vec<String> =
            serde_json::from_value(thread.participants.clone()).unwrap_or_default();
        if !participants.contains(&from_address.to_string()) {
            participants.push(from_address.to_string());
        }
        EmailThreadRepository::update_participants(
            &self.pool,
            thread.id,
            &serde_json::json!(participants),
        )
        .await?;
        Ok(())
    }

    async fn process_unmatched_email(
        &self,
        parsed: &ParsedEmail,
        raw_mime: &[u8],
        project_id: Uuid,
    ) -> Result<InboundResult, EmailError> {
        let snippet = parsed
            .text_body
            .as_deref()
            .map(|t| t.chars().take(200).collect::<String>());

        let unassigned_id = generate_id();
        UnassignedEmailRepository::create(
            &self.pool,
            &CreateUnassignedEmailParams {
                id: unassigned_id,
                project_id,
                from_name: parsed.from_name.clone(),
                from_address: parsed.from_address.clone(),
                subject: parsed.subject.clone(),
                snippet,
                raw_mime: raw_mime.to_vec(),
                has_attachments: !parsed.attachments.is_empty(),
            },
        )
        .await?;

        Ok(InboundResult {
            status: InboundStatus::Unmatched,
            task_id: None,
            thread_id: None,
            message_id: None,
            email_id: Some(unassigned_id),
        })
    }

    /// Assign an unassigned email to a task, creating a thread and conversation message.
    ///
    /// # Errors
    ///
    /// Returns `EmailError` on database failures.
    pub async fn assign_email(
        &self,
        email_id: Uuid,
        task_id: Uuid,
    ) -> Result<InboundResult, EmailError> {
        let unassigned = UnassignedEmailRepository::get_by_id(&self.pool, email_id).await?;

        if unassigned.assigned_at.is_some() {
            return Err(EmailError::AlreadyAssigned);
        }

        // Parse the raw MIME to get full details
        let parsed = parse_mime(&unassigned.raw_mime)?;

        // Create a new thread for this task
        let thread_id = generate_id();
        let normalized = normalize_subject(&parsed.subject);
        let thread = EmailThreadRepository::create(
            &self.pool,
            &CreateEmailThreadParams {
                id: thread_id,
                task_id,
                subject: if normalized.is_empty() {
                    parsed.subject.clone()
                } else {
                    normalized
                },
                participants: serde_json::json!([parsed.from_address]),
            },
        )
        .await?;

        // Process as matched email
        let result = self
            .process_matched_email(&parsed, &thread, &unassigned.raw_mime)
            .await?;

        // Mark as assigned
        UnassignedEmailRepository::mark_assigned(&self.pool, email_id, task_id).await?;

        Ok(InboundResult {
            status: InboundStatus::Matched,
            task_id: Some(task_id),
            thread_id: Some(thread_id),
            message_id: result.message_id,
            email_id: Some(email_id),
        })
    }
}

#[derive(sqlx::FromRow)]
struct TaskProjectRow {
    #[allow(dead_code)]
    id: Uuid,
    project_id: Uuid,
    organization_id: Uuid,
}
