use sqlx::PgPool;
use uuid::Uuid;

use super::error::AiError;
use super::models::{AiContext, AiPipelineEvent, AiSuggestionContent, SuggestionGroup};

// ── Insert Params ────────────────────────────────────────────

/// Parameters for inserting an `ai_context` record.
pub struct InsertAiContextParams {
    pub id: Uuid,
    pub task_id: Uuid,
    pub message_id: Uuid,
    pub prompt: String,
    pub response: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub duration_ms: i32,
}

// ── AI Pipeline Repository ───────────────────────────────────

pub struct AiPipelineRepository;

impl AiPipelineRepository {
    /// Insert a new AI pipeline event with `status = 'pending'`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn insert_event(
        pool: &PgPool,
        task_id: Uuid,
        trigger_type: &str,
        payload: serde_json::Value,
    ) -> Result<AiPipelineEvent, AiError> {
        let event = sqlx::query_as!(
            AiPipelineEvent,
            r#"INSERT INTO ai_pipeline_events (id, task_id, trigger_type, payload, status)
             VALUES ($1, $2, $3, $4, 'pending')
             RETURNING id, task_id, trigger_type, payload, status,
                       attempts, max_attempts, scheduled_at,
                       started_at, completed_at, error_message, created_at"#,
            Uuid::now_v7(),
            task_id,
            trigger_type,
            payload,
        )
        .fetch_one(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(event)
    }

    /// Claim the next pending event using `FOR UPDATE SKIP LOCKED`.
    ///
    /// Sets `status = 'processing'`, increments `attempts`, and sets `started_at = NOW()`.
    /// Only claims events where `status = 'pending'`, `scheduled_at <= NOW()`,
    /// and `attempts < max_attempts`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn claim_next(pool: &PgPool) -> Result<Option<AiPipelineEvent>, AiError> {
        sqlx::query_as!(
            AiPipelineEvent,
            r#"UPDATE ai_pipeline_events
             SET status = 'processing',
                 attempts = attempts + 1,
                 started_at = NOW()
             WHERE id = (
                 SELECT id FROM ai_pipeline_events
                 WHERE status = 'pending'
                   AND scheduled_at <= NOW()
                   AND attempts < max_attempts
                 ORDER BY scheduled_at
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING id, task_id, trigger_type, payload, status,
                       attempts, max_attempts, scheduled_at,
                       started_at, completed_at, error_message, created_at"#,
        )
        .fetch_optional(pool)
        .await
        .map_err(AiError::Database)
    }

    /// Mark a pipeline event as completed.
    ///
    /// Sets `status = 'completed'` and `completed_at = NOW()`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn complete_event(pool: &PgPool, id: Uuid) -> Result<(), AiError> {
        sqlx::query!(
            r#"UPDATE ai_pipeline_events
             SET status = 'completed', completed_at = NOW()
             WHERE id = $1"#,
            id,
        )
        .execute(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(())
    }

    /// Mark a pipeline event as failed or reschedule with exponential backoff.
    ///
    /// If `attempts < max_attempts`, reschedules with exponential backoff:
    /// - attempt 1: +30 seconds
    /// - attempt 2: +2 minutes
    /// - attempt 3+: +10 minutes
    ///
    /// Otherwise sets `status = 'failed'`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn fail_event(pool: &PgPool, id: Uuid, error_message: &str) -> Result<(), AiError> {
        sqlx::query!(
            r#"UPDATE ai_pipeline_events
             SET error_message = $2,
                 status = CASE
                     WHEN attempts < max_attempts THEN 'pending'
                     ELSE 'failed'
                 END,
                 scheduled_at = CASE
                     WHEN attempts < max_attempts THEN NOW() + (
                         CASE attempts
                             WHEN 1 THEN INTERVAL '30 seconds'
                             WHEN 2 THEN INTERVAL '2 minutes'
                             ELSE INTERVAL '10 minutes'
                         END
                     )
                     ELSE scheduled_at
                 END
             WHERE id = $1"#,
            id,
            error_message,
        )
        .execute(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(())
    }

    /// Reset pipeline events stuck in `'processing'` for longer than `timeout_secs`.
    ///
    /// Returns the number of rows reset back to `'pending'`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn reset_stale(pool: &PgPool, timeout_secs: i64) -> Result<u64, AiError> {
        let result = sqlx::query!(
            r#"UPDATE ai_pipeline_events
             SET status = 'pending', started_at = NULL
             WHERE status = 'processing'
               AND started_at < NOW() - ($1::BIGINT * INTERVAL '1 second')"#,
            timeout_secs,
        )
        .execute(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(result.rows_affected())
    }
}

// ── AI Context Repository ────────────────────────────────────

pub struct AiContextRepository;

impl AiContextRepository {
    /// Insert an `ai_context` record.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn insert(
        pool: &PgPool,
        params: &InsertAiContextParams,
    ) -> Result<AiContext, AiError> {
        sqlx::query_as!(
            AiContext,
            r#"INSERT INTO ai_contexts
                 (id, task_id, message_id, prompt, response, model,
                  input_tokens, output_tokens, duration_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, task_id, message_id, prompt, response, model,
                       input_tokens, output_tokens, duration_ms, created_at"#,
            params.id,
            params.task_id,
            params.message_id,
            params.prompt,
            params.response,
            params.model,
            params.input_tokens,
            params.output_tokens,
            params.duration_ms,
        )
        .fetch_one(pool)
        .await
        .map_err(AiError::Database)
    }
}

// ── Update Suggestion Decision Params ────────────────────────

/// Parameters for updating a suggestion's decision within a message.
pub struct UpdateSuggestionDecisionParams {
    pub message_id: Uuid,
    pub group_id: Uuid,
    pub suggestion_id: Uuid,
    pub decision: String,
    pub decided_by: Uuid,
    pub modified_parameters: Option<serde_json::Value>,
    pub execution_result: Option<serde_json::Value>,
}

// ── AI Suggestion Repository ─────────────────────────────────

pub struct AiSuggestionRepository;

impl AiSuggestionRepository {
    /// Insert a message with `source_type = 'ai_suggestion'` into the messages table.
    ///
    /// The `content` is serialized as JSONB. Returns the new message `id`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn insert_suggestion_message(
        pool: &PgPool,
        task_id: Uuid,
        sender_id: Option<Uuid>,
        content: &AiSuggestionContent,
    ) -> Result<Uuid, AiError> {
        let content_value = serde_json::to_value(content).map_err(|e| {
            AiError::Database(sqlx::Error::Decode(
                format!("Failed to serialize AiSuggestionContent: {e}").into(),
            ))
        })?;
        let message_id = Uuid::now_v7();

        sqlx::query!(
            r#"INSERT INTO messages (id, task_id, source_type, source_id, content)
             VALUES ($1, $2, 'ai_suggestion', $3, $4)"#,
            message_id,
            task_id,
            sender_id,
            content_value,
        )
        .execute(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(message_id)
    }

    /// Get a specific suggestion group from a message's JSONB content.
    ///
    /// # Errors
    ///
    /// Returns `AiError::GroupNotFound` if the message does not exist or does not contain
    /// a suggestion group with the given `group_id`. Returns `AiError::Database` on
    /// database failure.
    pub async fn get_suggestion_group(
        pool: &PgPool,
        message_id: Uuid,
        group_id: Uuid,
    ) -> Result<SuggestionGroup, AiError> {
        let row = sqlx::query!(
            r#"SELECT content FROM messages
             WHERE id = $1 AND source_type = 'ai_suggestion'"#,
            message_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(AiError::Database)?
        .ok_or(AiError::GroupNotFound)?;

        let ai_content: AiSuggestionContent = serde_json::from_value(row.content).map_err(|e| {
            AiError::Database(sqlx::Error::Decode(
                format!("Failed to deserialize AiSuggestionContent: {e}").into(),
            ))
        })?;

        if ai_content.suggestion_group.id == group_id {
            Ok(ai_content.suggestion_group)
        } else {
            Err(AiError::GroupNotFound)
        }
    }

    /// List suggestion groups from messages with `source_type = 'ai_suggestion'`,
    /// ordered by `created_at` DESC (using UUID v7 ordering via `id DESC`).
    ///
    /// Returns tuples of `(message_id, SuggestionGroup)`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::Database` on database failure.
    pub async fn list_suggestion_groups(
        pool: &PgPool,
        task_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<(Uuid, SuggestionGroup)>, AiError> {
        let rows = sqlx::query!(
            r#"SELECT id, content FROM messages
             WHERE task_id = $1
               AND source_type = 'ai_suggestion'
               AND ($2::UUID IS NULL OR id < $2)
             ORDER BY id DESC
             LIMIT $3"#,
            task_id,
            cursor,
            limit,
        )
        .fetch_all(pool)
        .await
        .map_err(AiError::Database)?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let ai_content: AiSuggestionContent =
                serde_json::from_value(row.content).map_err(|e| {
                    AiError::Database(sqlx::Error::Decode(
                        format!("Failed to deserialize AiSuggestionContent: {e}").into(),
                    ))
                })?;
            results.push((row.id, ai_content.suggestion_group));
        }

        Ok(results)
    }

    /// Update a specific suggestion's decision within a message's JSONB content.
    ///
    /// Uses a fetch-modify-write pattern to update the target suggestion in the
    /// `suggestionGroup.suggestions` array identified by `suggestion_id`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::GroupNotFound` if the message or group does not exist.
    /// Returns `AiError::SuggestionNotFound` if no suggestion with `suggestion_id` exists.
    /// Returns `AiError::Database` on database failure.
    pub async fn update_suggestion_decision(
        pool: &PgPool,
        params: UpdateSuggestionDecisionParams,
    ) -> Result<(), AiError> {
        let row = sqlx::query!(
            r#"SELECT content FROM messages
             WHERE id = $1 AND source_type = 'ai_suggestion'"#,
            params.message_id,
        )
        .fetch_optional(pool)
        .await
        .map_err(AiError::Database)?
        .ok_or(AiError::GroupNotFound)?;

        let mut ai_content: AiSuggestionContent =
            serde_json::from_value(row.content).map_err(|e| {
                AiError::Database(sqlx::Error::Decode(
                    format!("Failed to deserialize AiSuggestionContent: {e}").into(),
                ))
            })?;

        if ai_content.suggestion_group.id != params.group_id {
            return Err(AiError::GroupNotFound);
        }

        let suggestion = ai_content
            .suggestion_group
            .suggestions
            .iter_mut()
            .find(|s| s.id == params.suggestion_id)
            .ok_or(AiError::SuggestionNotFound)?;

        suggestion.decision = params.decision.parse().map_err(AiError::InvalidTrigger)?;
        suggestion.decided_by = Some(params.decided_by);
        suggestion.decided_at = Some(chrono::Utc::now());
        suggestion.modified_parameters = params.modified_parameters;
        suggestion.execution_result = params.execution_result;

        let updated_content = serde_json::to_value(&ai_content).map_err(|e| {
            AiError::Database(sqlx::Error::Decode(
                format!("Failed to serialize updated AiSuggestionContent: {e}").into(),
            ))
        })?;

        sqlx::query!(
            r#"UPDATE messages SET content = $2 WHERE id = $1"#,
            params.message_id,
            updated_content,
        )
        .execute(pool)
        .await
        .map_err(AiError::Database)?;

        Ok(())
    }
}
