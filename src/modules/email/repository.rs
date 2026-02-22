use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::EmailError;
use super::models::{
    CreateEmailMessageParams, CreateEmailThreadParams, CreateUnassignedEmailParams, EmailMessage,
    EmailThread, UnassignedEmail, UpdateEmailThreadParams,
};

// ── Email Thread Repository ──────────────────────────────────

pub struct EmailThreadRepository;

impl EmailThreadRepository {
    /// Create a new email thread.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateEmailThreadParams,
    ) -> Result<EmailThread, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"INSERT INTO email_threads (id, task_id, subject, participants)
             VALUES ($1, $2, $3, $4)
             RETURNING id, task_id, subject, participants, message_ids,
                       last_message_at, created_at, updated_at"#,
            params.id,
            params.task_id,
            params.subject,
            params.participants,
        )
        .fetch_one(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Get a thread by ID.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::ThreadNotFound` if not found.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<EmailThread, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"SELECT id, task_id, subject, participants, message_ids,
                      last_message_at, created_at, updated_at
             FROM email_threads
             WHERE id = $1"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(EmailError::ThreadNotFound)
    }

    /// List threads by task with cursor-based pagination (newest first).
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn list_by_task(
        pool: &PgPool,
        task_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<EmailThread>, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"SELECT id, task_id, subject, participants, message_ids,
                      last_message_at, created_at, updated_at
             FROM email_threads
             WHERE task_id = $1
               AND ($2::UUID IS NULL OR id < $2)
             ORDER BY id DESC
             LIMIT $3"#,
            task_id,
            cursor,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Update a thread's subject and/or participants.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::ThreadNotFound` if not found.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        params: &UpdateEmailThreadParams,
    ) -> Result<EmailThread, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"UPDATE email_threads
             SET subject = COALESCE($2, subject),
                 participants = COALESCE($3, participants),
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, task_id, subject, participants, message_ids,
                       last_message_at, created_at, updated_at"#,
            id,
            params.subject,
            params.participants,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(EmailError::ThreadNotFound)
    }

    /// Delete a thread by ID.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::ThreadNotFound` if not found.
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), EmailError> {
        let result = sqlx::query!(r#"DELETE FROM email_threads WHERE id = $1"#, id,)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(EmailError::ThreadNotFound);
        }
        Ok(())
    }

    /// Append a `message_id` to a thread's `message_ids` array.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn append_message_id(
        pool: &PgPool,
        thread_id: Uuid,
        message_id: &str,
    ) -> Result<(), EmailError> {
        sqlx::query!(
            r#"UPDATE email_threads
             SET message_ids = message_ids || to_jsonb($2::TEXT),
                 updated_at = NOW()
             WHERE id = $1"#,
            thread_id,
            message_id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update a thread's `last_message_at` timestamp.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn update_last_message_at(
        pool: &PgPool,
        thread_id: Uuid,
        last_message_at: DateTime<Utc>,
    ) -> Result<(), EmailError> {
        sqlx::query!(
            r#"UPDATE email_threads
             SET last_message_at = $2, updated_at = NOW()
             WHERE id = $1"#,
            thread_id,
            last_message_at,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update a thread's participants.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn update_participants(
        pool: &PgPool,
        thread_id: Uuid,
        participants: &serde_json::Value,
    ) -> Result<(), EmailError> {
        sqlx::query!(
            r#"UPDATE email_threads
             SET participants = $2, updated_at = NOW()
             WHERE id = $1"#,
            thread_id,
            participants,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find a thread by checking if any of its `message_ids` match (GIN index).
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn find_by_message_ids_contains(
        pool: &PgPool,
        message_id: &str,
    ) -> Result<Option<EmailThread>, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"SELECT id, task_id, subject, participants, message_ids,
                      last_message_at, created_at, updated_at
             FROM email_threads
             WHERE message_ids @> to_jsonb($1::TEXT)
             LIMIT 1"#,
            message_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Find a thread by normalized subject and sender address.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn find_by_subject_and_sender(
        pool: &PgPool,
        normalized_subject: &str,
        from_address: &str,
    ) -> Result<Option<EmailThread>, EmailError> {
        sqlx::query_as!(
            EmailThread,
            r#"SELECT et.id, et.task_id, et.subject, et.participants, et.message_ids,
                      et.last_message_at, et.created_at, et.updated_at
             FROM email_threads et
             INNER JOIN email_messages em ON em.thread_id = et.id
             WHERE LOWER(et.subject) = LOWER($1)
               AND em.from_address = $2
             ORDER BY et.last_message_at DESC NULLS LAST
             LIMIT 1"#,
            normalized_subject,
            from_address,
        )
        .fetch_optional(pool)
        .await
        .map_err(EmailError::Database)
    }
}

// ── Email Message Repository ─────────────────────────────────

pub struct EmailMessageRepository;

impl EmailMessageRepository {
    /// Create a new email message.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    /// Returns `EmailError::DuplicateEmail` if `message_id` already exists.
    pub async fn create(
        pool: &PgPool,
        params: &CreateEmailMessageParams,
    ) -> Result<EmailMessage, EmailError> {
        sqlx::query_as!(
            EmailMessage,
            r#"INSERT INTO email_messages
                 (id, thread_id, message_id, in_reply_to, references_header,
                  from_address, to_addresses, cc_addresses, subject, direction,
                  conversation_message_id, raw_headers, send_status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING id, thread_id, message_id, in_reply_to, references_header,
                       from_address, to_addresses, cc_addresses, subject, direction,
                       conversation_message_id, raw_headers, send_status,
                       retry_count, next_retry_at, created_at"#,
            params.id,
            params.thread_id,
            params.message_id,
            params.in_reply_to,
            params.references_header,
            params.from_address,
            params.to_addresses,
            params.cc_addresses,
            params.subject,
            params.direction.as_str(),
            params.conversation_message_id,
            params.raw_headers,
            params.send_status.as_str(),
        )
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("email_messages_message_id_key") {
                    return EmailError::DuplicateEmail;
                }
            }
            EmailError::Database(e)
        })
    }

    /// List messages by thread (oldest first).
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn list_by_thread(
        pool: &PgPool,
        thread_id: Uuid,
    ) -> Result<Vec<EmailMessage>, EmailError> {
        sqlx::query_as!(
            EmailMessage,
            r#"SELECT id, thread_id, message_id, in_reply_to, references_header,
                      from_address, to_addresses, cc_addresses, subject, direction,
                      conversation_message_id, raw_headers, send_status,
                      retry_count, next_retry_at, created_at
             FROM email_messages
             WHERE thread_id = $1
             ORDER BY created_at ASC"#,
            thread_id,
        )
        .fetch_all(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Find a message by its RFC 5322 `Message-ID`.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn find_by_message_id(
        pool: &PgPool,
        message_id: &str,
    ) -> Result<Option<EmailMessage>, EmailError> {
        sqlx::query_as!(
            EmailMessage,
            r#"SELECT id, thread_id, message_id, in_reply_to, references_header,
                      from_address, to_addresses, cc_addresses, subject, direction,
                      conversation_message_id, raw_headers, send_status,
                      retry_count, next_retry_at, created_at
             FROM email_messages
             WHERE message_id = $1"#,
            message_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Get the latest message in a thread.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn get_latest_in_thread(
        pool: &PgPool,
        thread_id: Uuid,
    ) -> Result<Option<EmailMessage>, EmailError> {
        sqlx::query_as!(
            EmailMessage,
            r#"SELECT id, thread_id, message_id, in_reply_to, references_header,
                      from_address, to_addresses, cc_addresses, subject, direction,
                      conversation_message_id, raw_headers, send_status,
                      retry_count, next_retry_at, created_at
             FROM email_messages
             WHERE thread_id = $1
             ORDER BY created_at DESC
             LIMIT 1"#,
            thread_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Find failed outbound messages that need retry.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn find_failed_for_retry(
        pool: &PgPool,
        now: DateTime<Utc>,
    ) -> Result<Vec<EmailMessage>, EmailError> {
        sqlx::query_as!(
            EmailMessage,
            r#"SELECT id, thread_id, message_id, in_reply_to, references_header,
                      from_address, to_addresses, cc_addresses, subject, direction,
                      conversation_message_id, raw_headers, send_status,
                      retry_count, next_retry_at, created_at
             FROM email_messages
             WHERE send_status = 'failed'
               AND retry_count < 3
               AND next_retry_at <= $1
             ORDER BY next_retry_at ASC"#,
            now,
        )
        .fetch_all(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Update send status and retry info.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn update_send_status(
        pool: &PgPool,
        id: Uuid,
        send_status: &str,
        retry_count: i32,
        next_retry_at: Option<DateTime<Utc>>,
    ) -> Result<(), EmailError> {
        sqlx::query!(
            r#"UPDATE email_messages
             SET send_status = $2, retry_count = $3, next_retry_at = $4
             WHERE id = $1"#,
            id,
            send_status,
            retry_count,
            next_retry_at,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

// ── Unassigned Email Repository ──────────────────────────────

pub struct UnassignedEmailRepository;

impl UnassignedEmailRepository {
    /// Create a new unassigned email.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateUnassignedEmailParams,
    ) -> Result<UnassignedEmail, EmailError> {
        sqlx::query_as!(
            UnassignedEmail,
            r#"INSERT INTO unassigned_emails
                 (id, project_id, from_name, from_address, subject, snippet, raw_mime, has_attachments)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, project_id, from_name, from_address, subject, snippet,
                       raw_mime, has_attachments, received_at, assigned_at,
                       assigned_task_id, created_at"#,
            params.id,
            params.project_id,
            params.from_name,
            params.from_address,
            params.subject,
            params.snippet,
            params.raw_mime,
            params.has_attachments,
        )
        .fetch_one(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// List unassigned emails by project (newest first, unassigned only).
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<UnassignedEmail>, EmailError> {
        sqlx::query_as!(
            UnassignedEmail,
            r#"SELECT id, project_id, from_name, from_address, subject, snippet,
                      raw_mime, has_attachments, received_at, assigned_at,
                      assigned_task_id, created_at
             FROM unassigned_emails
             WHERE project_id = $1
               AND assigned_at IS NULL
               AND ($2::UUID IS NULL OR id < $2)
             ORDER BY id DESC
             LIMIT $3"#,
            project_id,
            cursor,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(EmailError::Database)
    }

    /// Get an unassigned email by ID.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::UnassignedEmailNotFound` if not found.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<UnassignedEmail, EmailError> {
        sqlx::query_as!(
            UnassignedEmail,
            r#"SELECT id, project_id, from_name, from_address, subject, snippet,
                      raw_mime, has_attachments, received_at, assigned_at,
                      assigned_task_id, created_at
             FROM unassigned_emails
             WHERE id = $1"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(EmailError::UnassignedEmailNotFound)
    }

    /// Mark an unassigned email as assigned to a task.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::AlreadyAssigned` if already assigned.
    pub async fn mark_assigned(
        pool: &PgPool,
        id: Uuid,
        task_id: Uuid,
    ) -> Result<UnassignedEmail, EmailError> {
        let result = sqlx::query_as!(
            UnassignedEmail,
            r#"UPDATE unassigned_emails
             SET assigned_at = NOW(), assigned_task_id = $2
             WHERE id = $1 AND assigned_at IS NULL
             RETURNING id, project_id, from_name, from_address, subject, snippet,
                       raw_mime, has_attachments, received_at, assigned_at,
                       assigned_task_id, created_at"#,
            id,
            task_id,
        )
        .fetch_optional(pool)
        .await?;

        result.ok_or(EmailError::AlreadyAssigned)
    }
}
