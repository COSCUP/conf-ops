use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::error::AuditError;
use super::models::{ActorType, AuditLog, AuditLogFilters, CreateAuditLogParams};
use super::repository::AuditLogRepository;

/// The audit service records and queries audit log entries.
pub struct AuditService {
    pool: PgPool,
    event_bus: EventBus,
}

impl AuditService {
    /// Create a new audit service.
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Record an audit log entry.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn record_event(&self, params: CreateAuditLogParams) -> Result<AuditLog, AuditError> {
        AuditLogRepository::create(&self.pool, &params).await
    }

    /// Query audit logs with filters.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn query_logs(&self, filters: &AuditLogFilters) -> Result<Vec<AuditLog>, AuditError> {
        AuditLogRepository::query(&self.pool, filters).await
    }

    /// Query audit logs for an organization context.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn query_org_logs(
        &self,
        org_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let filters = AuditLogFilters {
            context_type: Some("organization".to_string()),
            context_id: Some(org_id),
            cursor,
            limit,
            ..Default::default()
        };
        self.query_logs(&filters).await
    }

    /// Query audit logs for a project context.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn query_project_logs(
        &self,
        project_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<AuditLog>, AuditError> {
        let filters = AuditLogFilters {
            context_type: Some("project".to_string()),
            context_id: Some(project_id),
            cursor,
            limit,
            ..Default::default()
        };
        self.query_logs(&filters).await
    }

    /// Start listening for domain events and recording audit logs automatically.
    pub fn start_event_listener(&self) {
        let pool = self.pool.clone();
        let mut rx = self.event_bus.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = handle_domain_event_for_audit(&pool, &event).await {
                            tracing::warn!(
                                error = %e,
                                "Failed to record audit log for domain event"
                            );
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Audit event handler lagged, skipped {n} event(s)");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::error!("Event bus closed, stopping audit event handler");
                        break;
                    }
                }
            }
        });
    }

    /// Start a background task to create future audit log partitions monthly.
    pub fn start_partition_manager(&self) {
        let pool = self.pool.clone();

        tokio::spawn(async move {
            // Run once on startup, then monthly
            if let Err(e) = ensure_future_partitions(&pool).await {
                tracing::warn!(error = %e, "Failed to create future audit log partitions on startup");
            }

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;

                // Check if it's the first day of the month
                let now = chrono::Utc::now();
                if now.format("%d").to_string() == "01" {
                    if let Err(e) = ensure_future_partitions(&pool).await {
                        tracing::warn!(error = %e, "Failed to create future audit log partitions");
                    }
                }
            }
        });
    }
}

/// Create partitions for the next 3 months from current date.
async fn ensure_future_partitions(pool: &PgPool) -> Result<(), AuditError> {
    let now = chrono::Utc::now();

    for offset in 0..4 {
        let date = now + chrono::Duration::days(i64::from(offset) * 31);
        let year = date.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
        let month = date.format("%m").to_string().parse::<u32>().unwrap_or(1);
        AuditLogRepository::create_partition(pool, year, month).await?;
    }

    Ok(())
}

/// Handle domain events and record audit logs.
async fn handle_domain_event_for_audit(
    pool: &PgPool,
    event: &DomainEvent,
) -> Result<(), AuditError> {
    let params = map_domain_event_to_audit(event);

    if let Some(params) = params {
        AuditLogRepository::create(pool, &params).await?;
    }

    Ok(())
}

/// Map a domain event to audit log creation parameters.
fn map_domain_event_to_audit(event: &DomainEvent) -> Option<CreateAuditLogParams> {
    let ep = map_task_events(event).or_else(|| map_member_and_other_events(event));
    ep.map(build_audit_params)
}

/// Map task-related domain events to audit event params.
fn map_task_events(event: &DomainEvent) -> Option<AuditEventParams> {
    match event {
        DomainEvent::TaskCreated {
            task_id,
            project_id,
            ..
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "task.create".to_string(),
            resource_type: "task".to_string(),
            resource_id: *task_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({}),
        }),
        DomainEvent::TaskStatusChanged {
            task_id,
            project_id,
            old_status,
            new_status,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "task.status_change".to_string(),
            resource_type: "task".to_string(),
            resource_id: *task_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({ "before": { "status": old_status }, "after": { "status": new_status } }),
        }),
        DomainEvent::TaskDeleted {
            task_id,
            project_id,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "task.delete".to_string(),
            resource_type: "task".to_string(),
            resource_id: *task_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({}),
        }),
        DomainEvent::TodoCompleted { todo_id, task_id } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "todo.status_change".to_string(),
            resource_type: "todo".to_string(),
            resource_id: *todo_id,
            context_type: Some("task".to_string()),
            context_id: Some(*task_id),
            details: serde_json::json!({ "after": { "status": "completed" } }),
        }),
        DomainEvent::DataEntryChanged {
            task_id,
            data_entry_id,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "data_entry.update".to_string(),
            resource_type: "data_entry".to_string(),
            resource_id: *data_entry_id,
            context_type: Some("task".to_string()),
            context_id: Some(*task_id),
            details: serde_json::json!({}),
        }),
        _ => None,
    }
}

/// Map member and other domain events to audit event params.
fn map_member_and_other_events(event: &DomainEvent) -> Option<AuditEventParams> {
    match event {
        DomainEvent::ProjectMemberJoined {
            project_id,
            account_id,
            role,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: Some(*account_id),
            action: "member.add".to_string(),
            resource_type: "member".to_string(),
            resource_id: *account_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({ "role": role }),
        }),
        DomainEvent::ProjectMemberRemoved {
            project_id,
            account_id,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: Some(*account_id),
            action: "member.remove".to_string(),
            resource_type: "member".to_string(),
            resource_id: *account_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({}),
        }),
        DomainEvent::ProjectMemberRoleChanged {
            project_id,
            account_id,
            old_role,
            new_role,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: Some(*account_id),
            action: "member.role_change".to_string(),
            resource_type: "member".to_string(),
            resource_id: *account_id,
            context_type: Some("project".to_string()),
            context_id: Some(*project_id),
            details: serde_json::json!({ "before": { "role": old_role }, "after": { "role": new_role } }),
        }),
        DomainEvent::SuggestionDecided {
            message_id,
            task_id,
            suggestion_id,
            decision,
        } => Some(AuditEventParams {
            actor_type: ActorType::Account,
            actor_id: None,
            action: format!("suggestion.{decision}"),
            resource_type: "message".to_string(),
            resource_id: *message_id,
            context_type: Some("task".to_string()),
            context_id: Some(*task_id),
            details: serde_json::json!({ "suggestion_id": suggestion_id.to_string() }),
        }),
        DomainEvent::ToolExecutionFailed {
            task_id,
            message_id,
            tool_name,
            error,
        } => Some(AuditEventParams {
            actor_type: ActorType::System,
            actor_id: None,
            action: "tool.execute".to_string(),
            resource_type: "tool".to_string(),
            resource_id: *message_id,
            context_type: Some("task".to_string()),
            context_id: Some(*task_id),
            details: serde_json::json!({ "tool_name": tool_name, "result": "failed", "error": error }),
        }),
        _ => None,
    }
}

/// Parameters for building an audit log entry from a domain event.
struct AuditEventParams {
    actor_type: ActorType,
    actor_id: Option<Uuid>,
    action: String,
    resource_type: String,
    resource_id: Uuid,
    context_type: Option<String>,
    context_id: Option<Uuid>,
    details: serde_json::Value,
}

/// Build audit log creation parameters from event params.
fn build_audit_params(params: AuditEventParams) -> CreateAuditLogParams {
    CreateAuditLogParams {
        id: Uuid::now_v7(),
        actor_type: params.actor_type,
        actor_id: params.actor_id,
        action: params.action,
        resource_type: params.resource_type,
        resource_id: params.resource_id,
        context_type: params.context_type,
        context_id: params.context_id,
        details: params.details,
        ip_address: None,
        user_agent: None,
    }
}
