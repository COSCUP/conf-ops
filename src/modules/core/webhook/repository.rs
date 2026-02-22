use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::WebhookError;
use super::models::{
    CreateWebhookEventLogParams, CreateWebhookParams, UpdateWebhookParams, Webhook, WebhookEventLog,
};

fn map_webhook(row: &sqlx::postgres::PgRow) -> Result<Webhook, sqlx::Error> {
    Ok(Webhook {
        id: row.try_get("id")?,
        project_id: row.try_get("project_id")?,
        name: row.try_get("name")?,
        url: row.try_get("url")?,
        secret: row.try_get("secret")?,
        events: row.try_get("events")?,
        enabled: row.try_get("enabled")?,
        created_by: row.try_get("created_by")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        deleted_at: row.try_get("deleted_at")?,
    })
}

fn map_event_log(row: &sqlx::postgres::PgRow) -> Result<WebhookEventLog, sqlx::Error> {
    Ok(WebhookEventLog {
        id: row.try_get("id")?,
        webhook_id: row.try_get("webhook_id")?,
        event_type: row.try_get("event_type")?,
        payload: row.try_get("payload")?,
        status: row.try_get("status")?,
        response_status: row.try_get("response_status")?,
        response_body: row.try_get("response_body")?,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        next_retry_at: row.try_get("next_retry_at")?,
        created_at: row.try_get("created_at")?,
        completed_at: row.try_get("completed_at")?,
    })
}

pub struct WebhookRepository;

impl WebhookRepository {
    /// Create a new webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateWebhookParams,
    ) -> Result<Webhook, WebhookError> {
        let row = sqlx::query(
            r"INSERT INTO webhooks (id, project_id, name, url, secret, events, created_by)
              VALUES ($1, $2, $3, $4, $5, $6, $7)
              RETURNING *",
        )
        .bind(params.id)
        .bind(params.project_id)
        .bind(&params.name)
        .bind(&params.url)
        .bind(&params.secret)
        .bind(&params.events)
        .bind(params.created_by)
        .fetch_one(pool)
        .await?;

        map_webhook(&row).map_err(WebhookError::Database)
    }

    /// Get a webhook by ID.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::NotFound` if not found.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Webhook, WebhookError> {
        let row = sqlx::query("SELECT * FROM webhooks WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| WebhookError::NotFound(id.to_string()))?;

        map_webhook(&row).map_err(WebhookError::Database)
    }

    /// List webhooks for a project.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Webhook>, WebhookError> {
        let rows = sqlx::query(
            "SELECT * FROM webhooks WHERE project_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_webhook(r).map_err(WebhookError::Database))
            .collect()
    }

    /// List enabled webhooks for a project that subscribe to a given event type.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn list_enabled_for_event(
        pool: &PgPool,
        project_id: Uuid,
        event_type: &str,
    ) -> Result<Vec<Webhook>, WebhookError> {
        let rows = sqlx::query(
            r"SELECT * FROM webhooks
              WHERE project_id = $1
                AND enabled = true
                AND deleted_at IS NULL
                AND events @> $2::jsonb",
        )
        .bind(project_id)
        .bind(serde_json::json!([event_type]))
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_webhook(r).map_err(WebhookError::Database))
            .collect()
    }

    /// Update a webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::NotFound` if not found.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        params: &UpdateWebhookParams,
    ) -> Result<Webhook, WebhookError> {
        let existing = Self::get_by_id(pool, id).await?;

        let name = params.name.as_deref().unwrap_or(&existing.name);
        let url = params.url.as_deref().unwrap_or(&existing.url);
        let secret = params.secret.as_deref().or(existing.secret.as_deref());
        let events = params.events.as_ref().unwrap_or(&existing.events);
        let enabled = params.enabled.unwrap_or(existing.enabled);

        let row = sqlx::query(
            r"UPDATE webhooks
              SET name = $2, url = $3, secret = $4, events = $5, enabled = $6, updated_at = NOW()
              WHERE id = $1 AND deleted_at IS NULL
              RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(url)
        .bind(secret)
        .bind(events)
        .bind(enabled)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| WebhookError::NotFound(id.to_string()))?;

        map_webhook(&row).map_err(WebhookError::Database)
    }

    /// Soft-delete a webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::NotFound` if not found.
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), WebhookError> {
        let result = sqlx::query(
            "UPDATE webhooks SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(WebhookError::NotFound(id.to_string()));
        }
        Ok(())
    }
}

pub struct WebhookEventLogRepository;

impl WebhookEventLogRepository {
    /// Create a new webhook event log entry.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateWebhookEventLogParams,
    ) -> Result<WebhookEventLog, WebhookError> {
        let row = sqlx::query(
            r"INSERT INTO webhook_event_logs (id, webhook_id, event_type, payload, status)
              VALUES ($1, $2, $3, $4, 'pending')
              RETURNING *",
        )
        .bind(params.id)
        .bind(params.webhook_id)
        .bind(&params.event_type)
        .bind(&params.payload)
        .fetch_one(pool)
        .await?;

        map_event_log(&row).map_err(WebhookError::Database)
    }

    /// Update event log after delivery attempt.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn update_after_attempt(
        pool: &PgPool,
        id: Uuid,
        status: &str,
        response_status: Option<i32>,
        response_body: Option<&str>,
        next_retry_at: Option<DateTime<Utc>>,
    ) -> Result<(), WebhookError> {
        // Truncate response body to 10KB
        let truncated_body = response_body.map(|b| if b.len() > 10_240 { &b[..10_240] } else { b });

        let completed_at: Option<DateTime<Utc>> = if status == "success" || status == "failed" {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r"UPDATE webhook_event_logs
              SET status = $2,
                  response_status = $3,
                  response_body = $4,
                  attempts = attempts + 1,
                  next_retry_at = $5,
                  completed_at = $6
              WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(response_status)
        .bind(truncated_body)
        .bind(next_retry_at)
        .bind(completed_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// List event logs for a webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn list_by_webhook(
        pool: &PgPool,
        webhook_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<WebhookEventLog>, WebhookError> {
        let rows = if let Some(cursor_id) = cursor {
            sqlx::query(
                r"SELECT * FROM webhook_event_logs
                  WHERE webhook_id = $1 AND id < $2
                  ORDER BY created_at DESC
                  LIMIT $3",
            )
            .bind(webhook_id)
            .bind(cursor_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                r"SELECT * FROM webhook_event_logs
                  WHERE webhook_id = $1
                  ORDER BY created_at DESC
                  LIMIT $2",
            )
            .bind(webhook_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };

        rows.iter()
            .map(|r| map_event_log(r).map_err(WebhookError::Database))
            .collect()
    }

    /// Get pending events that are ready to retry.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn get_pending_retries(
        pool: &PgPool,
        limit: i64,
    ) -> Result<Vec<WebhookEventLog>, WebhookError> {
        let rows = sqlx::query(
            r"SELECT * FROM webhook_event_logs
              WHERE status = 'pending'
                AND next_retry_at IS NOT NULL
                AND next_retry_at <= NOW()
                AND attempts < max_attempts
              ORDER BY next_retry_at ASC
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_event_log(r).map_err(WebhookError::Database))
            .collect()
    }
}
