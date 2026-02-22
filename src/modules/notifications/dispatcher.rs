use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::email::EmailService;

use super::error::NotificationError;
use super::models::{
    notification_type_to_category, CreateNotificationParams, DeliveryChannel, Notification,
    NotificationType,
};
use super::repository::{
    NotificationPreferencesRepository, NotificationRepository, WebPushSubscriptionRepository,
};
use super::web_push::WebPushSender;

/// Parameters for dispatching a notification.
pub struct DispatchParams {
    pub account_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: Option<String>,
    pub reference_type: Option<super::models::NotificationReferenceType>,
    pub reference_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

/// Dispatches notifications across multiple channels based on user preferences.
pub struct NotificationDispatcher {
    pool: PgPool,
    web_push_sender: Arc<WebPushSender>,
    email_service: Arc<dyn EmailService>,
}

impl NotificationDispatcher {
    /// Create a new notification dispatcher.
    pub fn new(
        pool: PgPool,
        web_push_sender: Arc<WebPushSender>,
        email_service: Arc<dyn EmailService>,
    ) -> Self {
        Self {
            pool,
            web_push_sender,
            email_service,
        }
    }

    /// Dispatch a notification to all enabled channels for the recipient.
    ///
    /// Always delivers in-app (if enabled). Optionally delivers via web push
    /// and email based on user preferences.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError` if the in-app notification creation fails.
    /// Individual channel delivery failures are logged but do not prevent
    /// other channels from being delivered.
    pub async fn dispatch(
        &self,
        params: &DispatchParams,
    ) -> Result<Notification, NotificationError> {
        let channels = self
            .resolve_channels(params.account_id, &params.notification_type)
            .await;

        let mut delivered = Vec::new();

        // Always attempt in-app
        if channels.contains(&DeliveryChannel::InApp) {
            delivered.push(DeliveryChannel::InApp);
        }

        // Create the in-app notification record
        let notification_id = Uuid::now_v7();
        let notification = NotificationRepository::create(
            &self.pool,
            &CreateNotificationParams {
                id: notification_id,
                account_id: params.account_id,
                notification_type: params.notification_type.clone(),
                title: params.title.clone(),
                body: params.body.clone(),
                reference_type: params.reference_type.clone(),
                reference_id: params.reference_id,
                project_id: params.project_id,
                delivered_channels: delivered.clone(),
            },
        )
        .await?;

        // Attempt web push delivery
        if channels.contains(&DeliveryChannel::WebPush) {
            match self
                .deliver_web_push(params.account_id, &params.title, params.body.as_deref())
                .await
            {
                Ok(()) => {
                    delivered.push(DeliveryChannel::WebPush);
                }
                Err(e) => {
                    tracing::warn!(
                        account_id = %params.account_id,
                        error = %e,
                        "Web Push delivery failed"
                    );
                }
            }
        }

        // Attempt email delivery
        if channels.contains(&DeliveryChannel::Email) {
            match self
                .deliver_email(params.account_id, &params.title, params.body.as_deref())
                .await
            {
                Ok(()) => {
                    delivered.push(DeliveryChannel::Email);
                }
                Err(e) => {
                    tracing::warn!(
                        account_id = %params.account_id,
                        error = %e,
                        "Email notification delivery failed"
                    );
                }
            }
        }

        // Update delivered channels if more were delivered
        if delivered.len() > 1 || (!delivered.is_empty() && delivered[0] != DeliveryChannel::InApp)
        {
            if let Err(e) = NotificationRepository::update_delivered_channels(
                &self.pool,
                notification_id,
                &delivered,
            )
            .await
            {
                tracing::warn!(
                    notification_id = %notification_id,
                    error = %e,
                    "Failed to update delivered channels"
                );
            }
        }

        Ok(notification)
    }

    /// Resolve which delivery channels are enabled for a notification type and account.
    async fn resolve_channels(
        &self,
        account_id: Uuid,
        notification_type: &NotificationType,
    ) -> Vec<DeliveryChannel> {
        let category = notification_type_to_category(notification_type);

        // Try to get user preferences
        let prefs = NotificationPreferencesRepository::get_by_account(&self.pool, account_id).await;

        match prefs {
            Ok(pref) => {
                let mut channels = Vec::new();
                let prefs_value = &pref.preferences;

                // Check each channel
                for (channel_key, channel_enum) in &[
                    ("inApp", DeliveryChannel::InApp),
                    ("webPush", DeliveryChannel::WebPush),
                    ("email", DeliveryChannel::Email),
                ] {
                    if is_channel_enabled_for_category(prefs_value, channel_key, category) {
                        channels.push(channel_enum.clone());
                    }
                }

                // In-app is always included as a minimum
                if !channels.contains(&DeliveryChannel::InApp) {
                    channels.push(DeliveryChannel::InApp);
                }

                channels
            }
            Err(_) => {
                // No preferences stored — use defaults (all channels enabled)
                vec![
                    DeliveryChannel::InApp,
                    DeliveryChannel::WebPush,
                    DeliveryChannel::Email,
                ]
            }
        }
    }

    /// Deliver a web push notification to all subscriptions for the account.
    async fn deliver_web_push(
        &self,
        account_id: Uuid,
        title: &str,
        body: Option<&str>,
    ) -> Result<(), NotificationError> {
        let subscriptions =
            WebPushSubscriptionRepository::list_by_account(&self.pool, account_id).await?;

        for sub in &subscriptions {
            if let Err(e) = self.web_push_sender.send(sub, title, body).await {
                tracing::warn!(
                    endpoint = %sub.endpoint,
                    error = %e,
                    "Failed to send web push to subscription"
                );
            }
        }

        Ok(())
    }

    /// Deliver an email notification.
    async fn deliver_email(
        &self,
        account_id: Uuid,
        title: &str,
        body: Option<&str>,
    ) -> Result<(), NotificationError> {
        // Look up the account's email address
        let row = sqlx::query("SELECT email FROM accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(NotificationError::Database)?;

        let Some(row) = row else {
            tracing::warn!(
                account_id = %account_id,
                "Account not found for email notification"
            );
            return Ok(());
        };

        let email: String =
            sqlx::Row::try_get(&row, "email").map_err(NotificationError::Database)?;

        let html_body = format!("<h2>{}</h2><p>{}</p>", title, body.unwrap_or(""));

        self.email_service
            .send(&email, &format!("[Conf-Ops] {title}"), &html_body)
            .await
            .map_err(|e| NotificationError::EmailFailed(e.to_string()))
    }
}

/// Check if a specific channel is enabled for a given category in the preferences JSON.
fn is_channel_enabled_for_category(
    preferences: &serde_json::Value,
    channel_key: &str,
    category: &str,
) -> bool {
    // preferences structure: { "channels": { "email": { "enabled": true, "categories": { "taskUpdates": true } } } }
    let channels = preferences.get("channels").unwrap_or(preferences);

    let Some(channel) = channels.get(channel_key) else {
        return true; // Default: enabled if not specified
    };

    // Check channel-level enabled
    let channel_enabled = channel
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);

    if !channel_enabled {
        return false;
    }

    // Check category-level enabled
    channel
        .get("categories")
        .and_then(|cats| cats.get(category))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_enabled_default_when_no_preferences() {
        let prefs = serde_json::json!({});
        assert!(is_channel_enabled_for_category(
            &prefs,
            "inApp",
            "taskUpdates"
        ));
        assert!(is_channel_enabled_for_category(&prefs, "email", "mentions"));
    }

    #[test]
    fn channel_disabled_globally() {
        let prefs = serde_json::json!({
            "channels": {
                "email": {
                    "enabled": false,
                    "categories": {
                        "taskUpdates": true
                    }
                }
            }
        });
        assert!(!is_channel_enabled_for_category(
            &prefs,
            "email",
            "taskUpdates"
        ));
    }

    #[test]
    fn channel_enabled_but_category_disabled() {
        let prefs = serde_json::json!({
            "channels": {
                "webPush": {
                    "enabled": true,
                    "categories": {
                        "mentions": false,
                        "taskUpdates": true
                    }
                }
            }
        });
        assert!(!is_channel_enabled_for_category(
            &prefs, "webPush", "mentions"
        ));
        assert!(is_channel_enabled_for_category(
            &prefs,
            "webPush",
            "taskUpdates"
        ));
    }

    #[test]
    fn category_default_true_if_not_listed() {
        let prefs = serde_json::json!({
            "channels": {
                "inApp": {
                    "enabled": true,
                    "categories": {
                        "taskUpdates": true
                    }
                }
            }
        });
        // "mentions" not listed in categories — defaults to true
        assert!(is_channel_enabled_for_category(&prefs, "inApp", "mentions"));
    }
}
