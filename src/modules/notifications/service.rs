use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::modules::email::EmailService;

use super::dispatcher::{DispatchParams, NotificationDispatcher};
use super::error::NotificationError;
use super::models::{
    CreateScheduledReminderParams, CreateWebPushSubscriptionParams, DeliveryChannel, Notification,
    ReminderType, ScheduledReminder, WebPushSubscription,
};
use super::repository::{
    NotificationPreferencesRepository, NotificationRepository, ScheduledReminderRepository,
    WebPushSubscriptionRepository,
};
use super::web_push::WebPushSender;

/// The notification service coordinates notification dispatch, preference
/// management, web push subscriptions, and reminder scheduling.
pub struct NotificationService {
    pool: PgPool,
    event_bus: EventBus,
    dispatcher: NotificationDispatcher,
}

impl NotificationService {
    /// Create a new notification service.
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        web_push_sender: Arc<WebPushSender>,
        email_service: Arc<dyn EmailService>,
    ) -> Self {
        let dispatcher = NotificationDispatcher::new(pool.clone(), web_push_sender, email_service);

        Self {
            pool,
            event_bus,
            dispatcher,
        }
    }

    /// Dispatch a notification to a recipient.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError` if the notification creation fails.
    pub async fn dispatch_notification(
        &self,
        params: &DispatchParams,
    ) -> Result<Notification, NotificationError> {
        self.dispatcher.dispatch(params).await
    }

    /// Mark a notification as read.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::NotFound` if the notification does not exist.
    pub async fn mark_as_read(
        &self,
        account_id: Uuid,
        notification_id: Uuid,
    ) -> Result<(), NotificationError> {
        NotificationRepository::mark_as_read(&self.pool, account_id, notification_id).await
    }

    /// Mark all notifications as read for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn mark_all_as_read(&self, account_id: Uuid) -> Result<i64, NotificationError> {
        NotificationRepository::mark_all_as_read(&self.pool, account_id).await
    }

    /// List notifications for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn list_notifications(
        &self,
        account_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        NotificationRepository::list_by_account(&self.pool, account_id, cursor, limit, unread_only)
            .await
    }

    /// Get unread notification count for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn get_unread_count(&self, account_id: Uuid) -> Result<i64, NotificationError> {
        NotificationRepository::get_unread_count(&self.pool, account_id).await
    }

    /// Get notification preferences for an account.
    /// Returns the stored preferences JSON or an empty object if none set.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn get_preferences(
        &self,
        account_id: Uuid,
    ) -> Result<serde_json::Value, NotificationError> {
        match NotificationPreferencesRepository::get_by_account(&self.pool, account_id).await {
            Ok(pref) => Ok(pref.preferences),
            Err(NotificationError::PreferencesNotFound) => Ok(serde_json::json!({})),
            Err(e) => Err(e),
        }
    }

    /// Update notification preferences for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn update_preferences(
        &self,
        account_id: Uuid,
        preferences: &serde_json::Value,
    ) -> Result<serde_json::Value, NotificationError> {
        let pref = NotificationPreferencesRepository::upsert(
            &self.pool,
            Uuid::now_v7(),
            account_id,
            preferences,
        )
        .await?;
        Ok(pref.preferences)
    }

    /// Register a Web Push subscription.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::DuplicateSubscription` if endpoint already exists.
    pub async fn subscribe_web_push(
        &self,
        account_id: Uuid,
        endpoint: String,
        p256dh_key: String,
        auth_key: String,
        device_name: Option<String>,
    ) -> Result<WebPushSubscription, NotificationError> {
        WebPushSubscriptionRepository::create(
            &self.pool,
            &CreateWebPushSubscriptionParams {
                id: Uuid::now_v7(),
                account_id,
                endpoint,
                p256dh_key,
                auth_key,
                device_name,
            },
        )
        .await
    }

    /// Unsubscribe a Web Push endpoint.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::SubscriptionNotFound` if not found.
    pub async fn unsubscribe_web_push(
        &self,
        account_id: Uuid,
        endpoint: &str,
    ) -> Result<(), NotificationError> {
        WebPushSubscriptionRepository::delete_by_endpoint(&self.pool, account_id, endpoint).await
    }

    /// Schedule a reminder for a todo.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn schedule_reminder(
        &self,
        todo_id: Uuid,
        reminder_type: ReminderType,
        trigger_at: chrono::DateTime<chrono::Utc>,
        config: Option<serde_json::Value>,
    ) -> Result<ScheduledReminder, NotificationError> {
        ScheduledReminderRepository::create(
            &self.pool,
            &CreateScheduledReminderParams {
                id: Uuid::now_v7(),
                todo_id,
                reminder_type,
                trigger_at,
                config,
            },
        )
        .await
    }

    /// Start listening for domain events that should trigger notifications.
    pub fn start_event_listener(&self) {
        let pool = self.pool.clone();
        let event_bus = self.event_bus.clone();
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = handle_domain_event(&pool, &event).await {
                            tracing::warn!(
                                error = %e,
                                "Failed to handle domain event for notification"
                            );
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Notification event handler lagged, skipped {n} event(s)");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::error!("Event bus closed, stopping notification event handler");
                        break;
                    }
                }
            }
        });
    }

    /// Returns a reference to the pool for use by the scheduler.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Returns a reference to the dispatcher for use by the scheduler.
    pub fn dispatcher(&self) -> &NotificationDispatcher {
        &self.dispatcher
    }

    /// Update the delivered channels for a notification.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn update_delivered_channels(
        &self,
        notification_id: Uuid,
        channels: &[DeliveryChannel],
    ) -> Result<(), NotificationError> {
        super::repository::NotificationRepository::update_delivered_channels(
            &self.pool,
            notification_id,
            channels,
        )
        .await
    }
}

/// Handle a domain event and create notifications as needed.
///
/// This is a stub that logs the event. In a full implementation, it would
/// query for the relevant recipients and dispatch notifications.
async fn handle_domain_event(_pool: &PgPool, event: &DomainEvent) -> Result<(), NotificationError> {
    match event {
        DomainEvent::TaskStatusChanged {
            task_id,
            new_status,
            ..
        } => {
            if new_status == "completed" {
                tracing::info!(
                    task_id = %task_id,
                    "Task completed — would notify participants (stub)"
                );
            }
        }
        DomainEvent::TodoCompleted { todo_id, task_id } => {
            tracing::info!(
                todo_id = %todo_id,
                task_id = %task_id,
                "Todo completed — would check linked tasks (stub)"
            );
        }
        DomainEvent::MessageSent {
            message_id,
            task_id,
            ..
        } => {
            tracing::info!(
                message_id = %message_id,
                task_id = %task_id,
                "Message sent — would check for mentions (stub)"
            );
        }
        DomainEvent::DataEntryChanged { task_id, .. } => {
            tracing::info!(
                task_id = %task_id,
                "Data entry changed — would notify source data watchers (stub)"
            );
        }
        DomainEvent::UnmatchedEmailReceived {
            email_id,
            project_id,
            ..
        } => {
            tracing::info!(
                email_id = %email_id,
                project_id = %project_id,
                "Unmatched email — would notify project owners (stub)"
            );
        }
        _ => {}
    }

    Ok(())
}
