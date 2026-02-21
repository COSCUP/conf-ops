use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::core::member::repository::MemberRepository;
use crate::modules::storage::service::{FileService, UploadFileParams};

use super::crdt::CrdtManager;
use super::error::ConversationError;
use super::models::{MemberContent, Message, MessageAttachment, MessageSourceType};
use super::repository::{
    ConversationStateRepository, CreateMessageParams, LastSeenPositionRepository, MessageRepository,
};

/// Raw email attachment before upload to storage.
pub struct RawEmailAttachment<'a> {
    pub filename: &'a str,
    pub mime_type: &'a str,
    pub data: &'a [u8],
    pub scope_type: &'a str,
    pub scope_id: Uuid,
    pub uploaded_by: Uuid,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
}

pub struct ConversationService {
    pool: PgPool,
    event_bus: EventBus,
    crdt_manager: Arc<CrdtManager>,
    file_service: Arc<FileService>,
}

impl ConversationService {
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        crdt_manager: Arc<CrdtManager>,
        file_service: Arc<FileService>,
    ) -> Self {
        Self {
            pool,
            event_bus,
            crdt_manager,
            file_service,
        }
    }

    /// Validate that `source_id` references an existing entity for the given `source_type`.
    ///
    /// - `Member` → checks `members` table
    /// - `System` → no `source_id` expected
    /// - `AiSuggestion`, `ToolExecution`, `EmailInbound` → accepted without DB check
    ///   (the corresponding services will be added in later phases)
    async fn validate_source(
        &self,
        source_type: &MessageSourceType,
        source_id: Option<Uuid>,
    ) -> Result<(), ConversationError> {
        match source_type {
            MessageSourceType::Member => {
                let id = source_id.ok_or(ConversationError::SourceNotFound)?;
                MemberRepository::get_by_id(&self.pool, id)
                    .await
                    .map_err(|_| ConversationError::SourceNotFound)?;
            }
            MessageSourceType::System | MessageSourceType::EmailInbound => {
                // System messages have no source_id.
                // Email sender may not be matched to a known account/contact.
            }
            MessageSourceType::AiSuggestion | MessageSourceType::ToolExecution => {
                // These source types are validated at the application layer
                // when the corresponding modules are implemented in later phases.
                // For now, only require that a source_id is provided.
                if source_id.is_none() {
                    return Err(ConversationError::SourceNotFound);
                }
            }
        }
        Ok(())
    }

    /// Send a message in a task conversation.
    ///
    /// If `last_seen_message_id` is provided, validates it matches the latest message
    /// (stale conversation protection). Returns 409 if stale.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::SourceNotFound` if `source_id` validation fails.
    /// Returns `ConversationError::StaleConversation` if there are unseen messages.
    /// Returns `ConversationError::Database` on database failure.
    pub async fn send_message(
        &self,
        task_id: Uuid,
        source_type: MessageSourceType,
        source_id: Option<Uuid>,
        content: serde_json::Value,
        attachments: Option<Vec<MessageAttachment>>,
        last_seen_message_id: Option<Uuid>,
    ) -> Result<Message, ConversationError> {
        // Validate source_id exists for the given source_type
        self.validate_source(&source_type, source_id).await?;

        // Validate content structure for Member messages
        if source_type == MessageSourceType::Member {
            serde_json::from_value::<MemberContent>(content.clone()).map_err(|e| {
                ConversationError::InvalidContent(format!(
                    "Member message content must match {{ text, mentions }} structure: {e}"
                ))
            })?;
        }

        // `lastSeenMessageId` stale check
        if let Some(last_seen) = last_seen_message_id {
            self.check_stale(task_id, last_seen).await?;
        }

        let attachments_json =
            attachments.map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));

        let id = generate_id();
        let message = MessageRepository::create(
            &self.pool,
            &CreateMessageParams {
                id,
                task_id,
                source_type: source_type.clone(),
                source_id,
                content,
                attachments: attachments_json,
                action_result: None,
                last_seen_message_id,
            },
        )
        .await?;

        // Push message summary to CRDT Y.Array
        let sender_id = source_id.unwrap_or(Uuid::nil());
        let _ = self
            .crdt_manager
            .push_message(task_id, id, &message.created_at.to_rfc3339(), sender_id)
            .await;

        self.event_bus.publish(DomainEvent::MessageSent {
            message_id: id,
            task_id,
            source_type: format!("{source_type:?}"),
        });

        Ok(message)
    }

    /// Add a system message to a task conversation.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn add_system_message(
        &self,
        task_id: Uuid,
        content: serde_json::Value,
    ) -> Result<Message, ConversationError> {
        let id = generate_id();
        let message = MessageRepository::create(
            &self.pool,
            &CreateMessageParams {
                id,
                task_id,
                source_type: MessageSourceType::System,
                source_id: None,
                content,
                attachments: None,
                action_result: None,
                last_seen_message_id: None,
            },
        )
        .await?;

        // Push message summary to CRDT Y.Array
        let _ = self
            .crdt_manager
            .push_message(task_id, id, &message.created_at.to_rfc3339(), Uuid::nil())
            .await;

        self.event_bus.publish(DomainEvent::MessageSent {
            message_id: id,
            task_id,
            source_type: "System".to_string(),
        });

        Ok(message)
    }

    /// Get conversation messages with cursor-based pagination.
    ///
    /// Returns `(messages, has_more)`.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get_conversation(
        &self,
        task_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
        source_type_filter: Option<&MessageSourceType>,
    ) -> Result<(Vec<Message>, bool), ConversationError> {
        let fetch_limit = limit + 1;
        let messages = MessageRepository::list_by_task(
            &self.pool,
            task_id,
            cursor,
            fetch_limit,
            source_type_filter,
        )
        .await?;

        let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_more = messages.len() > limit_usize;
        let messages = if has_more {
            messages.into_iter().take(limit_usize).collect()
        } else {
            messages
        };

        Ok((messages, has_more))
    }

    /// Update last seen position for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn update_last_seen(
        &self,
        task_id: Uuid,
        account_id: Uuid,
        message_id: Uuid,
    ) -> Result<(), ConversationError> {
        let unread =
            MessageRepository::count_after_message(&self.pool, task_id, message_id).await?;

        let id = generate_id();
        ConversationStateRepository::upsert(
            &self.pool,
            id,
            task_id,
            account_id,
            Some(message_id),
            i32::try_from(unread).unwrap_or(i32::MAX),
        )
        .await?;

        Ok(())
    }

    /// Get last seen state for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get_last_seen(
        &self,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<Uuid>, ConversationError> {
        let state = ConversationStateRepository::get(&self.pool, task_id, account_id).await?;
        Ok(state.and_then(|s| s.last_read_message_id))
    }

    /// Update the CRDT last-seen position (`y_clock`) for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn update_last_seen_position(
        &self,
        task_id: Uuid,
        account_id: Uuid,
        y_clock: i64,
    ) -> Result<(), ConversationError> {
        let id = generate_id();
        LastSeenPositionRepository::upsert(&self.pool, id, task_id, account_id, y_clock).await?;
        Ok(())
    }

    /// Get the CRDT last-seen position (`y_clock`) for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get_last_seen_position(
        &self,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<i64>, ConversationError> {
        let pos = LastSeenPositionRepository::get(&self.pool, task_id, account_id).await?;
        Ok(pos.map(|p| p.y_clock))
    }

    /// Process an inbound email: upload attachments and create a message.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::AttachmentUploadFailed` if any attachment upload fails.
    /// Returns `ConversationError::Database` on database failure.
    pub async fn receive_email_inbound(
        &self,
        task_id: Uuid,
        email_sender_id: Option<Uuid>,
        content: serde_json::Value,
        raw_attachments: Vec<RawEmailAttachment<'_>>,
    ) -> Result<Message, ConversationError> {
        let mut message_attachments = Vec::new();

        for raw in raw_attachments {
            let record = self
                .file_service
                .upload_file(&UploadFileParams {
                    filename: raw.filename,
                    mime_type: raw.mime_type,
                    data: raw.data,
                    scope_type: raw.scope_type,
                    scope_id: raw.scope_id,
                    uploaded_by: raw.uploaded_by,
                    organization_id: raw.organization_id,
                    project_id: raw.project_id,
                    task_id: raw.task_id,
                })
                .await
                .map_err(ConversationError::AttachmentUploadFailed)?;

            message_attachments.push(MessageAttachment {
                file_id: record.id,
                filename: record.filename,
                mime_type: record.mime_type,
                file_size: record.file_size,
                storage_path: record.storage_path,
            });
        }

        let attachments = if message_attachments.is_empty() {
            None
        } else {
            Some(message_attachments)
        };

        self.send_message(
            task_id,
            MessageSourceType::EmailInbound,
            email_sender_id,
            content,
            attachments,
            None,
        )
        .await
    }

    /// Get a reference to the CRDT manager.
    pub fn crdt_manager(&self) -> &Arc<CrdtManager> {
        &self.crdt_manager
    }

    /// Check if a conversation is stale (has unseen messages after `last_seen_message_id`).
    async fn check_stale(
        &self,
        task_id: Uuid,
        last_seen_message_id: Uuid,
    ) -> Result<(), ConversationError> {
        let latest = MessageRepository::get_latest_message_id(&self.pool, task_id).await?;

        if let Some(latest_id) = latest {
            if latest_id != last_seen_message_id {
                let unseen_count = MessageRepository::count_after_message(
                    &self.pool,
                    task_id,
                    last_seen_message_id,
                )
                .await?;
                return Err(ConversationError::StaleConversation {
                    latest_message_id: latest_id,
                    unseen_count,
                });
            }
        }

        Ok(())
    }
}
