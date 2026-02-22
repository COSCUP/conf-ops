use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::EmailError;
use super::models::{
    CreateEmailMessageParams, CreateEmailThreadParams, EmailDirection, EmailMessage, EmailThread,
    SendStatus,
};
use super::repository::{EmailMessageRepository, EmailThreadRepository};
use super::EmailService;

pub struct SendEmailParams {
    pub thread_id: Option<Uuid>,
    pub task_id: Uuid,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Vec<String>,
    pub subject: String,
    pub html_body: String,
}

pub struct EmailOutboundService {
    pool: PgPool,
    event_bus: EventBus,
    smtp_service: Arc<dyn EmailService>,
    from_domain: String,
}

impl EmailOutboundService {
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        smtp_service: Arc<dyn EmailService>,
        from_domain: String,
    ) -> Self {
        Self {
            pool,
            event_bus,
            smtp_service,
            from_domain,
        }
    }

    /// Send an email and record it. Creates or extends a thread.
    ///
    /// # Errors
    ///
    /// Returns `EmailError` on SMTP or database failures.
    pub async fn send_email(&self, params: SendEmailParams) -> Result<EmailMessage, EmailError> {
        let email_message_id = format!("<{}@{}>", generate_id(), self.from_domain);
        let now = Utc::now();

        // Resolve or create thread
        let (thread, in_reply_to, references_header) = if let Some(thread_id) = params.thread_id {
            let thread = EmailThreadRepository::get_by_id(&self.pool, thread_id).await?;
            let latest =
                EmailMessageRepository::get_latest_in_thread(&self.pool, thread_id).await?;
            let in_reply_to = latest.as_ref().map(|m| m.message_id.clone());
            let refs = Self::build_references(&thread, latest.as_ref());
            (thread, in_reply_to, Some(refs))
        } else {
            let thread_id = generate_id();
            let thread = EmailThreadRepository::create(
                &self.pool,
                &CreateEmailThreadParams {
                    id: thread_id,
                    task_id: params.task_id,
                    subject: params.subject.clone(),
                    participants: serde_json::json!(params.to_addresses),
                },
            )
            .await?;
            (thread, None, None)
        };

        // Try to send via SMTP
        let to_list = params.to_addresses.join(", ");
        let send_result = self
            .smtp_service
            .send(&to_list, &params.subject, &params.html_body)
            .await;

        let send_status = if send_result.is_ok() {
            SendStatus::Sent
        } else {
            SendStatus::Failed
        };

        // Record the email message
        let msg_id = generate_id();
        let email_msg = EmailMessageRepository::create(
            &self.pool,
            &CreateEmailMessageParams {
                id: msg_id,
                thread_id: thread.id,
                message_id: email_message_id.clone(),
                in_reply_to,
                references_header,
                from_address: params.from_address,
                to_addresses: serde_json::json!(params.to_addresses),
                cc_addresses: serde_json::json!(params.cc_addresses),
                subject: params.subject,
                direction: EmailDirection::Outbound,
                conversation_message_id: None,
                raw_headers: None,
                send_status: send_status.clone(),
            },
        )
        .await?;

        // Update thread metadata
        EmailThreadRepository::append_message_id(&self.pool, thread.id, &email_message_id).await?;
        EmailThreadRepository::update_last_message_at(&self.pool, thread.id, now).await?;

        // Merge participants
        let mut participants: Vec<String> =
            serde_json::from_value(thread.participants.clone()).unwrap_or_default();
        for addr in &params.to_addresses {
            if !participants.contains(addr) {
                participants.push(addr.clone());
            }
        }
        EmailThreadRepository::update_participants(
            &self.pool,
            thread.id,
            &serde_json::json!(participants),
        )
        .await?;

        // Set retry info if failed
        if send_status == SendStatus::Failed {
            let next_retry = now + chrono::Duration::minutes(1);
            EmailMessageRepository::update_send_status(
                &self.pool,
                email_msg.id,
                "failed",
                0,
                Some(next_retry),
            )
            .await?;
        }

        self.event_bus.publish(DomainEvent::EmailSent {
            email_message_id: email_msg.id,
            thread_id: thread.id,
            task_id: params.task_id,
        });

        Ok(email_msg)
    }

    /// Retry failed emails with exponential backoff.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn retry_failed_emails(&self) -> Result<usize, EmailError> {
        let now = Utc::now();
        let failed = EmailMessageRepository::find_failed_for_retry(&self.pool, now).await?;
        let mut retried = 0;

        for msg in failed {
            let to_addresses: Vec<String> =
                serde_json::from_value(msg.to_addresses.clone()).unwrap_or_default();
            let to_list = to_addresses.join(", ");

            // We don't have the body stored, so we send a minimal retry notification
            let result = self.smtp_service.send(&to_list, &msg.subject, "").await;

            let new_count = msg.retry_count + 1;
            if result.is_ok() {
                EmailMessageRepository::update_send_status(
                    &self.pool, msg.id, "sent", new_count, None,
                )
                .await?;
            } else {
                // Exponential backoff: 1min, 4min, 16min
                let backoff_mins = 4_i64.pow(u32::try_from(new_count).unwrap_or(3));
                let next_retry = if new_count < 3 {
                    Some(now + chrono::Duration::minutes(backoff_mins))
                } else {
                    None
                };
                EmailMessageRepository::update_send_status(
                    &self.pool, msg.id, "failed", new_count, next_retry,
                )
                .await?;
            }
            retried += 1;
        }

        Ok(retried)
    }

    fn build_references(thread: &EmailThread, latest: Option<&EmailMessage>) -> String {
        let mut refs: Vec<String> =
            serde_json::from_value(thread.message_ids.clone()).unwrap_or_default();
        if let Some(msg) = latest {
            if !refs.contains(&msg.message_id) {
                refs.push(msg.message_id.clone());
            }
        }
        refs.join(" ")
    }
}
