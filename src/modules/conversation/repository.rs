use sqlx::PgPool;
use uuid::Uuid;

use super::error::ConversationError;
use super::models::{ConversationState, LastSeenPosition, Message, MessageSourceType};

pub struct CreateMessageParams {
    pub id: Uuid,
    pub task_id: Uuid,
    pub source_type: MessageSourceType,
    pub source_id: Option<Uuid>,
    pub content: serde_json::Value,
    pub attachments: Option<serde_json::Value>,
    pub action_result: Option<serde_json::Value>,
    pub last_seen_message_id: Option<Uuid>,
}

pub struct MessageRepository;

impl MessageRepository {
    /// Insert a new message.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateMessageParams,
    ) -> Result<Message, ConversationError> {
        sqlx::query_as!(
            Message,
            r#"INSERT INTO messages (id, task_id, source_type, source_id, content, attachments, action_result, last_seen_message_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, task_id,
                       source_type AS "source_type: MessageSourceType",
                       source_id, content, attachments, action_result,
                       last_seen_message_id, created_at"#,
            params.id,
            params.task_id,
            params.source_type.clone() as MessageSourceType,
            params.source_id,
            params.content,
            params.attachments,
            params.action_result,
            params.last_seen_message_id,
        )
        .fetch_one(pool)
        .await
        .map_err(ConversationError::Database)
    }

    /// Fetch a message by ID.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::MessageNotFound` if not found.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Message, ConversationError> {
        sqlx::query_as!(
            Message,
            r#"SELECT id, task_id,
                      source_type AS "source_type: MessageSourceType",
                      source_id, content, attachments, action_result,
                      last_seen_message_id, created_at
             FROM messages
             WHERE id = $1"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ConversationError::MessageNotFound)
    }

    /// List messages by task with cursor-based pagination (newest first).
    ///
    /// Uses UUID v7's natural time ordering: `WHERE id < $cursor ORDER BY id DESC`.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn list_by_task(
        pool: &PgPool,
        task_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
        source_type_filter: Option<&MessageSourceType>,
    ) -> Result<Vec<Message>, ConversationError> {
        sqlx::query_as!(
            Message,
            r#"SELECT id, task_id,
                      source_type AS "source_type: MessageSourceType",
                      source_id, content, attachments, action_result,
                      last_seen_message_id, created_at
             FROM messages
             WHERE task_id = $1
               AND ($2::UUID IS NULL OR id < $2)
               AND ($3::message_source_type IS NULL OR source_type = $3)
             ORDER BY id DESC
             LIMIT $4"#,
            task_id,
            cursor,
            source_type_filter as Option<&MessageSourceType>,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(ConversationError::Database)
    }

    /// Get the latest message ID for a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get_latest_message_id(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Option<Uuid>, ConversationError> {
        let id = sqlx::query_scalar!(
            r#"SELECT id FROM messages WHERE task_id = $1 ORDER BY id DESC LIMIT 1"#,
            task_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(id)
    }

    /// Count messages after a given message ID.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn count_after_message(
        pool: &PgPool,
        task_id: Uuid,
        message_id: Uuid,
    ) -> Result<i64, ConversationError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM messages WHERE task_id = $1 AND id > $2"#,
            task_id,
            message_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }
}

pub struct ConversationStateRepository;

impl ConversationStateRepository {
    /// Upsert a conversation state.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn upsert(
        pool: &PgPool,
        id: Uuid,
        task_id: Uuid,
        account_id: Uuid,
        last_read_message_id: Option<Uuid>,
        unread_count: i32,
    ) -> Result<ConversationState, ConversationError> {
        sqlx::query_as!(
            ConversationState,
            r#"INSERT INTO conversation_states (id, task_id, account_id, last_read_message_id, unread_count)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (task_id, account_id)
             DO UPDATE SET last_read_message_id = $4, unread_count = $5, updated_at = NOW()
             RETURNING id, task_id, account_id, last_read_message_id, unread_count, created_at, updated_at"#,
            id,
            task_id,
            account_id,
            last_read_message_id,
            unread_count,
        )
        .fetch_one(pool)
        .await
        .map_err(ConversationError::Database)
    }

    /// Get conversation state for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get(
        pool: &PgPool,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<ConversationState>, ConversationError> {
        sqlx::query_as!(
            ConversationState,
            r#"SELECT id, task_id, account_id, last_read_message_id, unread_count, created_at, updated_at
             FROM conversation_states
             WHERE task_id = $1 AND account_id = $2"#,
            task_id,
            account_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(ConversationError::Database)
    }
}

pub struct LastSeenPositionRepository;

impl LastSeenPositionRepository {
    /// Upsert a last seen position.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn upsert(
        pool: &PgPool,
        id: Uuid,
        task_id: Uuid,
        account_id: Uuid,
        y_clock: i64,
    ) -> Result<LastSeenPosition, ConversationError> {
        sqlx::query_as!(
            LastSeenPosition,
            r#"INSERT INTO last_seen_positions (id, task_id, account_id, y_clock)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (task_id, account_id)
             DO UPDATE SET y_clock = $4, updated_at = NOW()
             RETURNING id, task_id, account_id, y_clock, updated_at"#,
            id,
            task_id,
            account_id,
            y_clock,
        )
        .fetch_one(pool)
        .await
        .map_err(ConversationError::Database)
    }

    /// Get last seen position for a user in a task.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get(
        pool: &PgPool,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<LastSeenPosition>, ConversationError> {
        sqlx::query_as!(
            LastSeenPosition,
            r#"SELECT id, task_id, account_id, y_clock, updated_at
             FROM last_seen_positions
             WHERE task_id = $1 AND account_id = $2"#,
            task_id,
            account_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(ConversationError::Database)
    }
}
