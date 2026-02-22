use serde_json::json;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::models::TriggerType;
use super::repository::AiPipelineRepository;

/// Subscribes to the [`EventBus`] and routes [`DomainEvent`]s to the AI pipeline
/// by inserting pending [`AiPipelineEvent`] rows.
pub struct TriggerRouter;

impl TriggerRouter {
    /// Spawn a background task that listens for domain events and creates pipeline events.
    ///
    /// Returns a [`tokio::task::JoinHandle`] for the spawned task.
    pub fn start(pool: PgPool, event_bus: &EventBus) -> tokio::task::JoinHandle<()> {
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Some((task_id, trigger_type, payload)) = Self::map_event(&event) {
                            if let Err(e) = AiPipelineRepository::insert_event(
                                &pool,
                                task_id,
                                &trigger_type.to_string(),
                                payload,
                            )
                            .await
                            {
                                tracing::warn!("Failed to insert pipeline event: {e}");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Trigger router lagged, skipped {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Event bus closed, stopping trigger router");
                        break;
                    }
                }
            }
        })
    }

    /// Map a [`DomainEvent`] to an optional `(task_id, TriggerType, payload)` triple.
    ///
    /// Returns `None` for events that should not trigger the AI pipeline in Phase 8.
    fn map_event(event: &DomainEvent) -> Option<(Uuid, TriggerType, serde_json::Value)> {
        match event {
            DomainEvent::TaskCreated { task_id, .. } => Some((
                *task_id,
                TriggerType::TaskCreated,
                json!({ "task_id": task_id }),
            )),
            DomainEvent::TodoCompleted { task_id, todo_id } => Some((
                *task_id,
                TriggerType::TodoCompleted,
                json!({ "todo_id": todo_id }),
            )),
            DomainEvent::MessageSent {
                task_id,
                message_id,
                source_type,
            } => {
                // Only trigger for member, email_inbound, and webhook messages
                if source_type == "member"
                    || source_type == "email_inbound"
                    || source_type == "webhook"
                {
                    Some((
                        *task_id,
                        TriggerType::MessageSent,
                        json!({
                            "message_id": message_id,
                            "source_type": source_type,
                        }),
                    ))
                } else {
                    None
                }
            }
            DomainEvent::ToolExecutionFailed {
                task_id,
                message_id,
                tool_name,
                error,
            } => Some((
                *task_id,
                TriggerType::ToolError,
                json!({
                    "message_id": message_id,
                    "tool_name": tool_name,
                    "error": error,
                }),
            )),
            DomainEvent::DataEntryChanged {
                task_id,
                data_entry_id,
            } => Some((
                *task_id,
                TriggerType::SourceDataChanged,
                json!({
                    "data_entry_id": data_entry_id,
                }),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_task_created() {
        let task_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let template_id = Uuid::new_v4();

        let event = DomainEvent::TaskCreated {
            task_id,
            project_id,
            template_id,
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, payload) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::TaskCreated);
        assert_eq!(payload["task_id"], task_id.to_string());
    }

    #[test]
    fn map_todo_completed() {
        let task_id = Uuid::new_v4();
        let todo_id = Uuid::new_v4();

        let event = DomainEvent::TodoCompleted { task_id, todo_id };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, payload) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::TodoCompleted);
        assert_eq!(payload["todo_id"], todo_id.to_string());
    }

    #[test]
    fn map_message_sent_member() {
        let task_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let event = DomainEvent::MessageSent {
            task_id,
            message_id,
            source_type: "member".to_string(),
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, payload) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::MessageSent);
        assert_eq!(payload["message_id"], message_id.to_string());
        assert_eq!(payload["source_type"], "member");
    }

    #[test]
    fn map_message_sent_email_inbound() {
        let task_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let event = DomainEvent::MessageSent {
            task_id,
            message_id,
            source_type: "email_inbound".to_string(),
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, _) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::MessageSent);
    }

    #[test]
    fn map_message_sent_webhook() {
        let task_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let event = DomainEvent::MessageSent {
            task_id,
            message_id,
            source_type: "webhook".to_string(),
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, _) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::MessageSent);
    }

    #[test]
    fn map_message_sent_ai_suggestion_ignored() {
        let task_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let event = DomainEvent::MessageSent {
            task_id,
            message_id,
            source_type: "ai_suggestion".to_string(),
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_none());
    }

    #[test]
    fn map_tool_execution_failed() {
        let task_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let event = DomainEvent::ToolExecutionFailed {
            task_id,
            message_id,
            tool_name: "email.send".to_string(),
            error: "SMTP connection refused".to_string(),
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, payload) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::ToolError);
        assert_eq!(payload["tool_name"], "email.send");
        assert_eq!(payload["error"], "SMTP connection refused");
    }

    #[test]
    fn map_data_entry_changed() {
        let task_id = Uuid::new_v4();
        let data_entry_id = Uuid::new_v4();

        let event = DomainEvent::DataEntryChanged {
            task_id,
            data_entry_id,
        };

        let result = TriggerRouter::map_event(&event);
        assert!(result.is_some());
        let (tid, trigger, payload) = result.unwrap();
        assert_eq!(tid, task_id);
        assert_eq!(trigger, TriggerType::SourceDataChanged);
        assert_eq!(payload["data_entry_id"], data_entry_id.to_string());
    }

    #[test]
    fn map_unrelated_events_return_none() {
        let events = vec![
            DomainEvent::SystemStarted,
            DomainEvent::SystemHealthCheck,
            DomainEvent::AccountCreated {
                account_id: Uuid::new_v4(),
            },
            DomainEvent::TaskStatusChanged {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
                old_status: "open".to_string(),
                new_status: "closed".to_string(),
            },
        ];

        for event in events {
            assert!(
                TriggerRouter::map_event(&event).is_none(),
                "expected None for event: {event:?}"
            );
        }
    }
}
