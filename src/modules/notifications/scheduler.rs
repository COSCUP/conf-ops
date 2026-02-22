use std::sync::Arc;

use sqlx::PgPool;

use super::dispatcher::DispatchParams;
use super::error::NotificationError;
use super::models::{NotificationReferenceType, NotificationType};
use super::repository::ScheduledReminderRepository;
use super::service::NotificationService;

/// Background scheduler that processes due reminders and creates notifications.
pub struct ReminderScheduler {
    notification_service: Arc<NotificationService>,
}

impl ReminderScheduler {
    /// Create a new reminder scheduler.
    pub fn new(notification_service: Arc<NotificationService>) -> Self {
        Self {
            notification_service,
        }
    }

    /// Start the reminder scheduler as a background task.
    ///
    /// Checks for due reminders every 60 seconds.
    pub fn start(self: Arc<Self>) {
        let scheduler = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                match scheduler.process_due_reminders().await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!("Reminder scheduler: processed {n} reminder(s)"),
                    Err(e) => tracing::warn!("Reminder scheduler error: {e}"),
                }
            }
        });
    }

    /// Process all due reminders.
    ///
    /// # Errors
    ///
    /// Returns `NotificationError` on database failure.
    pub async fn process_due_reminders(&self) -> Result<usize, NotificationError> {
        let reminders =
            ScheduledReminderRepository::list_due_reminders(self.notification_service.pool(), 100)
                .await?;

        let mut processed = 0;

        for reminder in &reminders {
            // Check if the todo is still valid (not completed or deleted)
            let todo_info = self.get_todo_info(reminder.todo_id).await?;

            let Some(info) = todo_info else {
                // Todo deleted — skip and mark as fired to avoid retrying
                ScheduledReminderRepository::mark_as_fired(
                    self.notification_service.pool(),
                    reminder.id,
                    uuid::Uuid::now_v7(), // Dummy notification ID
                )
                .await?;
                processed += 1;
                continue;
            };

            if info.status == "completed" {
                // Todo completed — skip and mark as fired
                ScheduledReminderRepository::mark_as_fired(
                    self.notification_service.pool(),
                    reminder.id,
                    uuid::Uuid::now_v7(),
                )
                .await?;
                processed += 1;
                continue;
            }

            // Build notification title based on reminder type
            let title = match reminder.reminder_type.as_str() {
                "due_date_approaching" => {
                    format!("Due date approaching: {}", info.title)
                }
                "due_date_overdue" => {
                    format!("Overdue: {}", info.title)
                }
                "todo_stale" => {
                    format!("Stale todo: {}", info.title)
                }
                _ => format!("Reminder: {}", info.title),
            };

            // Dispatch notification to the todo's assignees
            for assignee_account_id in &info.assignee_account_ids {
                match self
                    .notification_service
                    .dispatch_notification(&DispatchParams {
                        account_id: *assignee_account_id,
                        notification_type: NotificationType::Reminder,
                        title: title.clone(),
                        body: None,
                        reference_type: Some(NotificationReferenceType::Todo),
                        reference_id: Some(reminder.todo_id),
                        project_id: info.project_id,
                    })
                    .await
                {
                    Ok(notification) => {
                        ScheduledReminderRepository::mark_as_fired(
                            self.notification_service.pool(),
                            reminder.id,
                            notification.id,
                        )
                        .await?;
                    }
                    Err(e) => {
                        tracing::warn!(
                            reminder_id = %reminder.id,
                            error = %e,
                            "Failed to dispatch reminder notification"
                        );
                    }
                }
            }

            processed += 1;
        }

        Ok(processed)
    }

    /// Get basic info about a todo for generating reminder notifications.
    async fn get_todo_info(
        &self,
        todo_id: uuid::Uuid,
    ) -> Result<Option<TodoInfo>, NotificationError> {
        let pool = self.notification_service.pool();

        let row = sqlx::query(
            "SELECT t.title, t.status, tk.project_id
             FROM todos t
             JOIN tasks tk ON tk.id = t.task_id
             WHERE t.id = $1 AND t.deleted_at IS NULL",
        )
        .bind(todo_id)
        .fetch_optional(pool)
        .await
        .map_err(NotificationError::Database)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let title: String =
            sqlx::Row::try_get(&row, "title").map_err(NotificationError::Database)?;
        let status: String =
            sqlx::Row::try_get(&row, "status").map_err(NotificationError::Database)?;
        let project_id: Option<uuid::Uuid> =
            sqlx::Row::try_get(&row, "project_id").map_err(NotificationError::Database)?;

        // Get assignee account IDs
        let assignee_rows = sqlx::query(
            "SELECT m.account_id
             FROM todo_assignees ta
             JOIN members m ON m.id = ta.member_id
             WHERE ta.todo_id = $1",
        )
        .bind(todo_id)
        .fetch_all(pool)
        .await
        .map_err(NotificationError::Database)?;

        let assignee_account_ids: Vec<uuid::Uuid> = assignee_rows
            .iter()
            .filter_map(|r| sqlx::Row::try_get(r, "account_id").ok())
            .collect();

        Ok(Some(TodoInfo {
            title,
            status,
            project_id,
            assignee_account_ids,
        }))
    }
}

/// Basic info about a todo needed for reminder notifications.
struct TodoInfo {
    title: String,
    status: String,
    project_id: Option<uuid::Uuid>,
    assignee_account_ids: Vec<uuid::Uuid>,
}

/// Scan for todos approaching due dates and create reminders.
///
/// This should be called periodically (e.g., every hour).
///
/// # Errors
///
/// Returns `NotificationError` on database failure.
pub async fn scan_due_date_reminders(pool: &PgPool) -> Result<usize, NotificationError> {
    // Find todos with due dates approaching (within 24h) that don't have a pending reminder
    let rows = sqlx::query(
        "SELECT t.id as todo_id, t.due_date
         FROM todos t
         WHERE t.due_date IS NOT NULL
           AND t.status != 'completed'
           AND t.deleted_at IS NULL
           AND t.due_date > NOW()
           AND t.due_date <= NOW() + INTERVAL '24 hours'
           AND NOT EXISTS (
               SELECT 1 FROM scheduled_reminders sr
               WHERE sr.todo_id = t.id
                 AND sr.type = 'due_date_approaching'
                 AND sr.fired = false
           )",
    )
    .fetch_all(pool)
    .await
    .map_err(NotificationError::Database)?;

    let mut created = 0;
    for row in &rows {
        let todo_id: uuid::Uuid =
            sqlx::Row::try_get(row, "todo_id").map_err(NotificationError::Database)?;
        let due_date: chrono::DateTime<chrono::Utc> =
            sqlx::Row::try_get(row, "due_date").map_err(NotificationError::Database)?;

        ScheduledReminderRepository::create(
            pool,
            &super::models::CreateScheduledReminderParams {
                id: uuid::Uuid::now_v7(),
                todo_id,
                reminder_type: super::models::ReminderType::DueDateApproaching,
                trigger_at: due_date - chrono::Duration::hours(24),
                config: Some(serde_json::json!({ "advance_days": 1 })),
            },
        )
        .await?;
        created += 1;
    }

    // Find overdue todos that don't have a pending overdue reminder
    let overdue_rows = sqlx::query(
        "SELECT t.id as todo_id
         FROM todos t
         WHERE t.due_date IS NOT NULL
           AND t.status != 'completed'
           AND t.deleted_at IS NULL
           AND t.due_date < NOW()
           AND NOT EXISTS (
               SELECT 1 FROM scheduled_reminders sr
               WHERE sr.todo_id = t.id
                 AND sr.type = 'due_date_overdue'
                 AND sr.fired = false
           )",
    )
    .fetch_all(pool)
    .await
    .map_err(NotificationError::Database)?;

    for row in &overdue_rows {
        let todo_id: uuid::Uuid =
            sqlx::Row::try_get(row, "todo_id").map_err(NotificationError::Database)?;

        ScheduledReminderRepository::create(
            pool,
            &super::models::CreateScheduledReminderParams {
                id: uuid::Uuid::now_v7(),
                todo_id,
                reminder_type: super::models::ReminderType::DueDateOverdue,
                trigger_at: chrono::Utc::now(),
                config: None,
            },
        )
        .await?;
        created += 1;
    }

    Ok(created)
}

/// Scan for stale todos and create reminders.
///
/// This should be called periodically (e.g., daily).
///
/// # Errors
///
/// Returns `NotificationError` on database failure.
pub async fn scan_stale_todos(pool: &PgPool, stale_days: i64) -> Result<usize, NotificationError> {
    let rows = sqlx::query(
        "SELECT t.id as todo_id
         FROM todos t
         WHERE t.status = 'in_progress'
           AND t.deleted_at IS NULL
           AND t.updated_at < NOW() - ($1::BIGINT || ' days')::INTERVAL
           AND NOT EXISTS (
               SELECT 1 FROM scheduled_reminders sr
               WHERE sr.todo_id = t.id
                 AND sr.type = 'todo_stale'
                 AND sr.fired = false
           )",
    )
    .bind(stale_days)
    .fetch_all(pool)
    .await
    .map_err(NotificationError::Database)?;

    let mut created = 0;
    for row in &rows {
        let todo_id: uuid::Uuid =
            sqlx::Row::try_get(row, "todo_id").map_err(NotificationError::Database)?;

        ScheduledReminderRepository::create(
            pool,
            &super::models::CreateScheduledReminderParams {
                id: uuid::Uuid::now_v7(),
                todo_id,
                reminder_type: super::models::ReminderType::TodoStale,
                trigger_at: chrono::Utc::now(),
                config: Some(serde_json::json!({ "stale_days": stale_days })),
            },
        )
        .await?;
        created += 1;
    }

    Ok(created)
}
