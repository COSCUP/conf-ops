use super::error::NotificationError;
use super::models::WebPushSubscription;

/// Web Push sender that delivers push notifications via VAPID.
///
/// In production, this uses the `web-push` crate. For now, this is a
/// stub that logs the push attempt and can be swapped with a real
/// implementation when VAPID keys are configured.
pub struct WebPushSender {
    vapid_private_key: Option<String>,
    vapid_public_key: Option<String>,
}

impl WebPushSender {
    /// Create a new Web Push sender from environment variables.
    pub fn from_env() -> Self {
        Self {
            vapid_private_key: std::env::var("WEB_PUSH_VAPID_PRIVATE_KEY").ok(),
            vapid_public_key: std::env::var("WEB_PUSH_VAPID_PUBLIC_KEY").ok(),
        }
    }

    /// Create a new Web Push sender for testing (no-op).
    pub fn noop() -> Self {
        Self {
            vapid_private_key: None,
            vapid_public_key: None,
        }
    }

    /// Returns the VAPID public key for client-side subscription.
    pub fn vapid_public_key(&self) -> Option<&str> {
        self.vapid_public_key.as_deref()
    }

    /// Returns whether Web Push is configured and available.
    pub fn is_configured(&self) -> bool {
        self.vapid_private_key.is_some() && self.vapid_public_key.is_some()
    }

    /// Send a push notification to a subscription endpoint.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::WebPushFailed` if delivery fails.
    pub async fn send(
        &self,
        subscription: &WebPushSubscription,
        title: &str,
        body: Option<&str>,
    ) -> Result<(), NotificationError> {
        if !self.is_configured() {
            tracing::debug!(
                endpoint = %subscription.endpoint,
                "Web Push not configured, skipping push notification"
            );
            return Ok(());
        }

        // Build the push notification payload
        let payload = serde_json::json!({
            "title": title,
            "body": body.unwrap_or(""),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        tracing::info!(
            endpoint = %subscription.endpoint,
            account_id = %subscription.account_id,
            payload = %payload,
            "Sending Web Push notification (stub — real VAPID delivery pending web-push crate)"
        );

        // TODO: Replace with actual web-push crate delivery when VAPID keys are configured.
        // For now this is a successful no-op to avoid adding the web-push dependency
        // before it is needed in production.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_sender_is_not_configured() {
        let sender = WebPushSender::noop();
        assert!(!sender.is_configured());
        assert!(sender.vapid_public_key().is_none());
    }

    #[tokio::test]
    async fn noop_sender_send_succeeds() {
        let sender = WebPushSender::noop();
        let sub = WebPushSubscription {
            id: uuid::Uuid::now_v7(),
            account_id: uuid::Uuid::now_v7(),
            endpoint: "https://push.example.com/sub1".to_string(),
            p256dh_key: "test-key".to_string(),
            auth_key: "test-auth".to_string(),
            device_name: None,
            created_at: chrono::Utc::now(),
        };

        let result = sender.send(&sub, "Test Title", Some("Test body")).await;
        assert!(result.is_ok());
    }
}
