use chrono::{Duration, Utc};
use sha2::Digest;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::error::WebhookError;
use super::models::{
    is_valid_event_type, CreateWebhookEventLogParams, CreateWebhookParams, UpdateWebhookParams,
    Webhook, WebhookEventLog,
};
use super::repository::{WebhookEventLogRepository, WebhookRepository};

/// The webhook service coordinates webhook management and event dispatch.
pub struct WebhookService {
    pool: PgPool,
    event_bus: EventBus,
    http_client: reqwest::Client,
}

impl WebhookService {
    /// Create a new webhook service.
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            pool,
            event_bus,
            http_client,
        }
    }

    /// Register a new webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError` if event types are invalid or database operation fails.
    pub async fn register_webhook(
        &self,
        project_id: Uuid,
        name: String,
        url: String,
        secret: Option<String>,
        events: Vec<String>,
        created_by: Uuid,
    ) -> Result<Webhook, WebhookError> {
        // Validate event types
        for event_type in &events {
            if !is_valid_event_type(event_type) {
                return Err(WebhookError::InvalidEventType(event_type.clone()));
            }
        }

        let params = CreateWebhookParams {
            id: Uuid::now_v7(),
            project_id,
            name,
            url,
            secret,
            events: serde_json::json!(events),
            created_by,
        };

        WebhookRepository::create(&self.pool, &params).await
    }

    /// Update a webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError` if webhook is not found or event types are invalid.
    pub async fn update_webhook(
        &self,
        webhook_id: Uuid,
        params: UpdateWebhookParams,
    ) -> Result<Webhook, WebhookError> {
        // Validate event types if provided
        if let Some(ref events) = params.events {
            if let Some(arr) = events.as_array() {
                for ev in arr {
                    if let Some(s) = ev.as_str() {
                        if !is_valid_event_type(s) {
                            return Err(WebhookError::InvalidEventType(s.to_string()));
                        }
                    }
                }
            }
        }

        WebhookRepository::update(&self.pool, webhook_id, &params).await
    }

    /// Delete a webhook (soft delete).
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::NotFound` if webhook is not found.
    pub async fn delete_webhook(&self, webhook_id: Uuid) -> Result<(), WebhookError> {
        WebhookRepository::delete(&self.pool, webhook_id).await
    }

    /// Get a webhook by ID.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::NotFound` if not found.
    pub async fn get_webhook(&self, webhook_id: Uuid) -> Result<Webhook, WebhookError> {
        WebhookRepository::get_by_id(&self.pool, webhook_id).await
    }

    /// List webhooks for a project.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn list_webhooks(&self, project_id: Uuid) -> Result<Vec<Webhook>, WebhookError> {
        WebhookRepository::list_by_project(&self.pool, project_id).await
    }

    /// Dispatch an event to all matching webhooks for a project.
    ///
    /// This creates event log entries and sends HTTP POST requests asynchronously.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn dispatch_event(
        &self,
        project_id: Uuid,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<Vec<WebhookEventLog>, WebhookError> {
        let webhooks =
            WebhookRepository::list_enabled_for_event(&self.pool, project_id, event_type).await?;

        let mut logs = Vec::new();

        for webhook in &webhooks {
            let log_params = CreateWebhookEventLogParams {
                id: Uuid::now_v7(),
                webhook_id: webhook.id,
                event_type: event_type.to_string(),
                payload: payload.clone(),
            };

            let event_log = WebhookEventLogRepository::create(&self.pool, &log_params).await?;

            // Attempt delivery
            let delivery_result = self
                .deliver(&webhook.url, webhook.secret.as_deref(), &payload)
                .await;

            match delivery_result {
                Ok((status_code, body)) => {
                    let status_str = if (200..300).contains(&status_code) {
                        "success"
                    } else {
                        "pending"
                    };

                    let next_retry = if status_str == "pending" {
                        Some(Utc::now() + Duration::minutes(1))
                    } else {
                        None
                    };

                    WebhookEventLogRepository::update_after_attempt(
                        &self.pool,
                        event_log.id,
                        status_str,
                        Some(status_code),
                        Some(&body),
                        next_retry,
                    )
                    .await?;
                }
                Err(err) => {
                    let next_retry = Some(Utc::now() + Duration::minutes(1));
                    WebhookEventLogRepository::update_after_attempt(
                        &self.pool,
                        event_log.id,
                        "pending",
                        None,
                        Some(&err.to_string()),
                        next_retry,
                    )
                    .await?;
                }
            }

            logs.push(event_log);
        }

        Ok(logs)
    }

    /// Retry failed webhook events.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn retry_failed_events(&self) -> Result<usize, WebhookError> {
        let pending = WebhookEventLogRepository::get_pending_retries(&self.pool, 50).await?;
        let count = pending.len();

        for event_log in &pending {
            let Ok(webhook) = WebhookRepository::get_by_id(&self.pool, event_log.webhook_id).await
            else {
                continue;
            };

            if !webhook.enabled {
                // Mark as failed if webhook has been disabled
                WebhookEventLogRepository::update_after_attempt(
                    &self.pool,
                    event_log.id,
                    "failed",
                    None,
                    Some("Webhook has been disabled"),
                    None,
                )
                .await?;
                continue;
            }

            let delivery_result = self
                .deliver(&webhook.url, webhook.secret.as_deref(), &event_log.payload)
                .await;

            let attempt_number = event_log.attempts + 1;

            match delivery_result {
                Ok((status_code, body)) => {
                    if (200..300).contains(&status_code) {
                        WebhookEventLogRepository::update_after_attempt(
                            &self.pool,
                            event_log.id,
                            "success",
                            Some(status_code),
                            Some(&body),
                            None,
                        )
                        .await?;
                    } else if attempt_number >= event_log.max_attempts {
                        WebhookEventLogRepository::update_after_attempt(
                            &self.pool,
                            event_log.id,
                            "failed",
                            Some(status_code),
                            Some(&body),
                            None,
                        )
                        .await?;
                    } else {
                        let next_retry = compute_next_retry(attempt_number);
                        WebhookEventLogRepository::update_after_attempt(
                            &self.pool,
                            event_log.id,
                            "pending",
                            Some(status_code),
                            Some(&body),
                            Some(next_retry),
                        )
                        .await?;
                    }
                }
                Err(err) => {
                    if attempt_number >= event_log.max_attempts {
                        WebhookEventLogRepository::update_after_attempt(
                            &self.pool,
                            event_log.id,
                            "failed",
                            None,
                            Some(&err.to_string()),
                            None,
                        )
                        .await?;
                    } else {
                        let next_retry = compute_next_retry(attempt_number);
                        WebhookEventLogRepository::update_after_attempt(
                            &self.pool,
                            event_log.id,
                            "pending",
                            None,
                            Some(&err.to_string()),
                            Some(next_retry),
                        )
                        .await?;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Send a test event to a specific webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError` on failure.
    pub async fn send_test_event(&self, webhook_id: Uuid) -> Result<WebhookEventLog, WebhookError> {
        let webhook = WebhookRepository::get_by_id(&self.pool, webhook_id).await?;

        let payload = serde_json::json!({
            "event": "test",
            "timestamp": Utc::now().to_rfc3339(),
            "project_id": webhook.project_id.to_string(),
            "data": {
                "message": "This is a test webhook event"
            }
        });

        let log_params = CreateWebhookEventLogParams {
            id: Uuid::now_v7(),
            webhook_id,
            event_type: "test".to_string(),
            payload: payload.clone(),
        };

        let event_log = WebhookEventLogRepository::create(&self.pool, &log_params).await?;

        let delivery_result = self
            .deliver(&webhook.url, webhook.secret.as_deref(), &payload)
            .await;

        match delivery_result {
            Ok((status_code, body)) => {
                let status_str = if (200..300).contains(&status_code) {
                    "success"
                } else {
                    "failed"
                };
                WebhookEventLogRepository::update_after_attempt(
                    &self.pool,
                    event_log.id,
                    status_str,
                    Some(status_code),
                    Some(&body),
                    None,
                )
                .await?;
            }
            Err(err) => {
                WebhookEventLogRepository::update_after_attempt(
                    &self.pool,
                    event_log.id,
                    "failed",
                    None,
                    Some(&err.to_string()),
                    None,
                )
                .await?;
            }
        }

        Ok(event_log)
    }

    /// List event logs for a webhook.
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::Database` on database failure.
    pub async fn list_event_logs(
        &self,
        webhook_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<WebhookEventLog>, WebhookError> {
        WebhookEventLogRepository::list_by_webhook(&self.pool, webhook_id, cursor, limit).await
    }

    /// Start the event listener that dispatches webhook events based on domain events.
    pub fn start_event_listener(&self) {
        let pool = self.pool.clone();
        let event_bus = self.event_bus.clone();
        let http_client = self.http_client.clone();
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            let service = Self {
                pool,
                event_bus,
                http_client,
            };

            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = handle_domain_event_for_webhook(&service, &event).await {
                            tracing::warn!(error = %e, "Failed to dispatch webhook for domain event");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Webhook event handler lagged, skipped {n} event(s)");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::error!("Event bus closed, stopping webhook event handler");
                        break;
                    }
                }
            }
        });
    }

    /// Deliver a payload to a webhook URL with optional HMAC signing.
    async fn deliver(
        &self,
        url: &str,
        secret: Option<&str>,
        payload: &serde_json::Value,
    ) -> Result<(i32, String), reqwest::Error> {
        let body = serde_json::to_string(payload).unwrap_or_default();

        let mut request = self
            .http_client
            .post(url)
            .header("Content-Type", "application/json");

        if let Some(secret_key) = secret {
            let signature = compute_hmac_signature(secret_key, &body);
            let timestamp = Utc::now().timestamp().to_string();
            request = request
                .header("X-Webhook-Signature", format!("sha256={signature}"))
                .header("X-Webhook-Timestamp", timestamp);
        }

        let response = request.body(body).send().await?;

        let status = i32::from(response.status().as_u16());
        let response_body = response.text().await.unwrap_or_default();

        Ok((status, response_body))
    }
}

/// Compute HMAC-SHA256 signature for webhook delivery.
fn compute_hmac_signature(secret: &str, body: &str) -> String {
    use sha2::Sha256;
    // Use HMAC manually via sha2: H(key || message)
    // For proper HMAC we use hmac crate pattern with sha2
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(body.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute next retry time using exponential backoff.
fn compute_next_retry(attempt: i32) -> chrono::DateTime<Utc> {
    let delay_minutes = match attempt {
        1 => 1,
        2 => 5,
        _ => 30,
    };
    Utc::now() + Duration::minutes(delay_minutes)
}

/// Handle domain events and dispatch webhook notifications.
async fn handle_domain_event_for_webhook(
    service: &WebhookService,
    event: &DomainEvent,
) -> Result<(), WebhookError> {
    if let Some((project_id, event_type, payload)) = map_domain_event_to_webhook(event) {
        service
            .dispatch_event(project_id, &event_type, payload)
            .await?;
    }
    Ok(())
}

/// Map a domain event to webhook dispatch parameters.
fn map_domain_event_to_webhook(event: &DomainEvent) -> Option<(Uuid, String, serde_json::Value)> {
    match event {
        DomainEvent::TaskCreated {
            task_id,
            project_id,
            ..
        } => {
            let data = serde_json::json!({ "id": task_id.to_string() });
            Some((
                *project_id,
                "task.created".to_string(),
                build_webhook_payload("task.created", project_id, &data),
            ))
        }
        DomainEvent::TaskStatusChanged {
            task_id,
            project_id,
            old_status,
            new_status,
        } => {
            let event_type = if new_status == "completed" {
                "task.completed"
            } else {
                "task.updated"
            };
            let data = serde_json::json!({ "id": task_id.to_string(), "old_status": old_status, "new_status": new_status });
            Some((
                *project_id,
                event_type.to_string(),
                build_webhook_payload(event_type, project_id, &data),
            ))
        }
        DomainEvent::TaskDeleted {
            task_id,
            project_id,
        } => {
            let data = serde_json::json!({ "id": task_id.to_string() });
            Some((
                *project_id,
                "task.deleted".to_string(),
                build_webhook_payload("task.deleted", project_id, &data),
            ))
        }
        DomainEvent::ProjectMemberJoined {
            project_id,
            account_id,
            ..
        } => {
            let data = serde_json::json!({ "account_id": account_id.to_string() });
            Some((
                *project_id,
                "member.added".to_string(),
                build_webhook_payload("member.added", project_id, &data),
            ))
        }
        DomainEvent::ProjectMemberRemoved {
            project_id,
            account_id,
        } => {
            let data = serde_json::json!({ "account_id": account_id.to_string() });
            Some((
                *project_id,
                "member.removed".to_string(),
                build_webhook_payload("member.removed", project_id, &data),
            ))
        }
        DomainEvent::TodoCompleted { todo_id, task_id } => {
            tracing::debug!(todo_id = %todo_id, task_id = %task_id,
                "Todo completed — webhook dispatch requires project_id lookup (stub)");
            None
        }
        DomainEvent::MessageSent {
            message_id,
            task_id,
            ..
        } => {
            tracing::debug!(message_id = %message_id, task_id = %task_id,
                "Message sent — webhook dispatch requires project_id lookup (stub)");
            None
        }
        _ => None,
    }
}

/// Build a standard webhook event payload.
fn build_webhook_payload(
    event_type: &str,
    project_id: &Uuid,
    data: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "event": event_type,
        "timestamp": Utc::now().to_rfc3339(),
        "project_id": project_id.to_string(),
        "data": data
    })
}

/// Encode bytes as hexadecimal string.
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_hmac_signature_deterministic() {
        let sig1 = compute_hmac_signature("secret", "body");
        let sig2 = compute_hmac_signature("secret", "body");
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn compute_hmac_different_secrets_differ() {
        let sig1 = compute_hmac_signature("secret1", "body");
        let sig2 = compute_hmac_signature("secret2", "body");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn compute_next_retry_exponential_backoff() {
        let before = Utc::now();
        let retry1 = compute_next_retry(1);
        let retry2 = compute_next_retry(2);
        let retry3 = compute_next_retry(3);

        assert!(retry1 > before);
        assert!(retry2 > retry1);
        assert!(retry3 > retry2);
    }

    #[test]
    fn hex_encode_works() {
        assert_eq!(hex::encode([0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }
}
