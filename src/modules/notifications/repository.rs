use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::NotificationError;
use super::models::{
    CreateNotificationParams, CreateScheduledReminderParams, CreateWebPushSubscriptionParams,
    DeliveryChannel, Notification, NotificationPreference, ScheduledReminder, WebPushSubscription,
};

// ── Row Mapping Helpers ─────────────────────────────────────

fn map_notification(row: &sqlx::postgres::PgRow) -> Result<Notification, sqlx::Error> {
    Ok(Notification {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        notification_type: row.try_get("type")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        reference_type: row.try_get("reference_type")?,
        reference_id: row.try_get("reference_id")?,
        project_id: row.try_get("project_id")?,
        is_read: row.try_get("is_read")?,
        read_at: row.try_get("read_at")?,
        delivered_channels: row.try_get("delivered_channels")?,
        created_at: row.try_get("created_at")?,
    })
}

fn map_notification_preference(
    row: &sqlx::postgres::PgRow,
) -> Result<NotificationPreference, sqlx::Error> {
    Ok(NotificationPreference {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        preferences: row.try_get("preferences")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn map_web_push_subscription(
    row: &sqlx::postgres::PgRow,
) -> Result<WebPushSubscription, sqlx::Error> {
    Ok(WebPushSubscription {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        endpoint: row.try_get("endpoint")?,
        p256dh_key: row.try_get("p256dh_key")?,
        auth_key: row.try_get("auth_key")?,
        device_name: row.try_get("device_name")?,
        created_at: row.try_get("created_at")?,
    })
}

fn map_scheduled_reminder(row: &sqlx::postgres::PgRow) -> Result<ScheduledReminder, sqlx::Error> {
    Ok(ScheduledReminder {
        id: row.try_get("id")?,
        todo_id: row.try_get("todo_id")?,
        reminder_type: row.try_get("type")?,
        trigger_at: row.try_get("trigger_at")?,
        fired: row.try_get("fired")?,
        fired_at: row.try_get("fired_at")?,
        notification_id: row.try_get("notification_id")?,
        config: row.try_get("config")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

// ── Notification Repository ─────────────────────────────────

pub struct NotificationRepository;

impl NotificationRepository {
    /// Create a notification.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateNotificationParams,
    ) -> Result<Notification, NotificationError> {
        let channels_json: Vec<String> = params
            .delivered_channels
            .iter()
            .map(|ch| ch.as_str().to_string())
            .collect();
        let channels_value = serde_json::to_value(&channels_json)
            .unwrap_or_else(|_| serde_json::Value::Array(vec![]));

        let row = sqlx::query(
            "INSERT INTO notifications
                 (id, account_id, type, title, body, reference_type, reference_id,
                  project_id, delivered_channels)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, account_id, type, title, body, reference_type, reference_id,
                       project_id, is_read, read_at, delivered_channels, created_at",
        )
        .bind(params.id)
        .bind(params.account_id)
        .bind(params.notification_type.as_str())
        .bind(&params.title)
        .bind(params.body.as_deref())
        .bind(
            params
                .reference_type
                .as_ref()
                .map(super::models::NotificationReferenceType::as_str),
        )
        .bind(params.reference_id)
        .bind(params.project_id)
        .bind(&channels_value)
        .fetch_one(pool)
        .await?;

        map_notification(&row).map_err(NotificationError::Database)
    }

    /// List notifications for an account with optional cursor and unread-only filter.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn list_by_account(
        pool: &PgPool,
        account_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        let rows = sqlx::query(
            "SELECT id, account_id, type, title, body, reference_type, reference_id,
                    project_id, is_read, read_at, delivered_channels, created_at
             FROM notifications
             WHERE account_id = $1
               AND ($2::UUID IS NULL OR id < $2)
               AND ($3::BOOLEAN = false OR is_read = false)
             ORDER BY created_at DESC, id DESC
             LIMIT $4",
        )
        .bind(account_id)
        .bind(cursor)
        .bind(unread_only)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_notification(r).map_err(NotificationError::Database))
            .collect()
    }

    /// Get unread count for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn get_unread_count(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<i64, NotificationError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM notifications
             WHERE account_id = $1 AND is_read = false",
        )
        .bind(account_id)
        .fetch_one(pool)
        .await?;

        row.try_get::<i64, _>("count")
            .map_err(NotificationError::Database)
    }

    /// Mark a notification as read.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::NotFound` if not found.
    /// Returns `NotificationError::Database` on database failure.
    pub async fn mark_as_read(
        pool: &PgPool,
        account_id: Uuid,
        notification_id: Uuid,
    ) -> Result<(), NotificationError> {
        let result = sqlx::query(
            "UPDATE notifications
             SET is_read = true, read_at = NOW()
             WHERE id = $1 AND account_id = $2 AND is_read = false",
        )
        .bind(notification_id)
        .bind(account_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            // Check if notification exists at all for this user
            let exists =
                sqlx::query("SELECT 1 FROM notifications WHERE id = $1 AND account_id = $2")
                    .bind(notification_id)
                    .bind(account_id)
                    .fetch_optional(pool)
                    .await?;

            if exists.is_none() {
                return Err(NotificationError::NotFound(notification_id.to_string()));
            }
            // Already read — not an error
        }
        Ok(())
    }

    /// Mark all notifications as read for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn mark_all_as_read(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<i64, NotificationError> {
        let result = sqlx::query(
            "UPDATE notifications
             SET is_read = true, read_at = NOW()
             WHERE account_id = $1 AND is_read = false",
        )
        .bind(account_id)
        .execute(pool)
        .await?;

        Ok(i64::try_from(result.rows_affected()).unwrap_or(0))
    }

    /// Update delivered channels for a notification.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn update_delivered_channels(
        pool: &PgPool,
        notification_id: Uuid,
        channels: &[DeliveryChannel],
    ) -> Result<(), NotificationError> {
        let channels_json: Vec<String> =
            channels.iter().map(|ch| ch.as_str().to_string()).collect();
        let channels_value = serde_json::to_value(&channels_json)
            .unwrap_or_else(|_| serde_json::Value::Array(vec![]));

        sqlx::query("UPDATE notifications SET delivered_channels = $2 WHERE id = $1")
            .bind(notification_id)
            .bind(&channels_value)
            .execute(pool)
            .await?;

        Ok(())
    }
}

// ── Notification Preferences Repository ─────────────────────

pub struct NotificationPreferencesRepository;

impl NotificationPreferencesRepository {
    /// Get notification preferences for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::PreferencesNotFound` if not found.
    /// Returns `NotificationError::Database` on database failure.
    pub async fn get_by_account(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<NotificationPreference, NotificationError> {
        let row = sqlx::query(
            "SELECT id, account_id, preferences, updated_at
             FROM notification_preferences
             WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_optional(pool)
        .await?
        .ok_or(NotificationError::PreferencesNotFound)?;

        map_notification_preference(&row).map_err(NotificationError::Database)
    }

    /// Upsert notification preferences for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn upsert(
        pool: &PgPool,
        id: Uuid,
        account_id: Uuid,
        preferences: &serde_json::Value,
    ) -> Result<NotificationPreference, NotificationError> {
        let row = sqlx::query(
            "INSERT INTO notification_preferences (id, account_id, preferences)
             VALUES ($1, $2, $3)
             ON CONFLICT (account_id)
             DO UPDATE SET preferences = $3, updated_at = NOW()
             RETURNING id, account_id, preferences, updated_at",
        )
        .bind(id)
        .bind(account_id)
        .bind(preferences)
        .fetch_one(pool)
        .await?;

        map_notification_preference(&row).map_err(NotificationError::Database)
    }
}

// ── Web Push Subscription Repository ────────────────────────

pub struct WebPushSubscriptionRepository;

impl WebPushSubscriptionRepository {
    /// Create a web push subscription.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::DuplicateSubscription` if endpoint already exists.
    /// Returns `NotificationError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateWebPushSubscriptionParams,
    ) -> Result<WebPushSubscription, NotificationError> {
        let row = sqlx::query(
            "INSERT INTO web_push_subscriptions
                 (id, account_id, endpoint, p256dh_key, auth_key, device_name)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, account_id, endpoint, p256dh_key, auth_key, device_name, created_at",
        )
        .bind(params.id)
        .bind(params.account_id)
        .bind(&params.endpoint)
        .bind(&params.p256dh_key)
        .bind(&params.auth_key)
        .bind(params.device_name.as_deref())
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("idx_web_push_subscriptions_endpoint") {
                    return NotificationError::DuplicateSubscription;
                }
            }
            NotificationError::Database(e)
        })?;

        map_web_push_subscription(&row).map_err(NotificationError::Database)
    }

    /// List web push subscriptions for an account.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn list_by_account(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<Vec<WebPushSubscription>, NotificationError> {
        let rows = sqlx::query(
            "SELECT id, account_id, endpoint, p256dh_key, auth_key, device_name, created_at
             FROM web_push_subscriptions
             WHERE account_id = $1
             ORDER BY created_at DESC",
        )
        .bind(account_id)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_web_push_subscription(r).map_err(NotificationError::Database))
            .collect()
    }

    /// Delete a web push subscription by endpoint.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::SubscriptionNotFound` if not found.
    /// Returns `NotificationError::Database` on database failure.
    pub async fn delete_by_endpoint(
        pool: &PgPool,
        account_id: Uuid,
        endpoint: &str,
    ) -> Result<(), NotificationError> {
        let result = sqlx::query(
            "DELETE FROM web_push_subscriptions
             WHERE account_id = $1 AND endpoint = $2",
        )
        .bind(account_id)
        .bind(endpoint)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(NotificationError::SubscriptionNotFound);
        }
        Ok(())
    }
}

// ── Scheduled Reminder Repository ───────────────────────────

pub struct ScheduledReminderRepository;

impl ScheduledReminderRepository {
    /// Create a scheduled reminder.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateScheduledReminderParams,
    ) -> Result<ScheduledReminder, NotificationError> {
        let row = sqlx::query(
            "INSERT INTO scheduled_reminders
                 (id, todo_id, type, trigger_at, config)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, todo_id, type, trigger_at, fired, fired_at,
                       notification_id, config, created_at, updated_at",
        )
        .bind(params.id)
        .bind(params.todo_id)
        .bind(params.reminder_type.as_str())
        .bind(params.trigger_at)
        .bind(params.config.as_ref())
        .fetch_one(pool)
        .await?;

        map_scheduled_reminder(&row).map_err(NotificationError::Database)
    }

    /// List unfired reminders that are due.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn list_due_reminders(
        pool: &PgPool,
        limit: i64,
    ) -> Result<Vec<ScheduledReminder>, NotificationError> {
        let rows = sqlx::query(
            "SELECT id, todo_id, type, trigger_at, fired, fired_at,
                    notification_id, config, created_at, updated_at
             FROM scheduled_reminders
             WHERE fired = false AND trigger_at <= NOW()
             ORDER BY trigger_at ASC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|r| map_scheduled_reminder(r).map_err(NotificationError::Database))
            .collect()
    }

    /// Mark a reminder as fired.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn mark_as_fired(
        pool: &PgPool,
        reminder_id: Uuid,
        notification_id: Uuid,
    ) -> Result<(), NotificationError> {
        sqlx::query(
            "UPDATE scheduled_reminders
             SET fired = true, fired_at = NOW(), notification_id = $2, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(reminder_id)
        .bind(notification_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete all unfired reminders for a todo.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError::Database` on database failure.
    pub async fn delete_unfired_by_todo(
        pool: &PgPool,
        todo_id: Uuid,
    ) -> Result<i64, NotificationError> {
        let result = sqlx::query(
            "DELETE FROM scheduled_reminders
             WHERE todo_id = $1 AND fired = false",
        )
        .bind(todo_id)
        .execute(pool)
        .await?;

        Ok(i64::try_from(result.rows_affected()).unwrap_or(0))
    }
}
