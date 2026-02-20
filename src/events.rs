use tokio::sync::broadcast;

/// Domain events emitted by various modules.
///
/// New variants will be added as modules are implemented in later phases.
#[derive(Debug, Clone)]
pub enum DomainEvent {
    SystemStarted,
    SystemHealthCheck,
    AccountCreated {
        account_id: uuid::Uuid,
    },
    AccountUpdated {
        account_id: uuid::Uuid,
    },
    OrganizationCreated {
        organization_id: uuid::Uuid,
        created_by: uuid::Uuid,
    },
    MemberInvited {
        organization_id: uuid::Uuid,
        account_id: uuid::Uuid,
    },
    MemberJoined {
        organization_id: uuid::Uuid,
        account_id: uuid::Uuid,
    },
    ProjectCreated {
        project_id: uuid::Uuid,
        organization_id: uuid::Uuid,
    },
    ProjectCopied {
        project_id: uuid::Uuid,
        source_project_id: uuid::Uuid,
        organization_id: uuid::Uuid,
    },
    ProjectStatusChanged {
        project_id: uuid::Uuid,
        old_status: String,
        new_status: String,
    },
    ProjectMemberJoined {
        project_id: uuid::Uuid,
        account_id: uuid::Uuid,
        role: String,
    },
    ProjectMemberRemoved {
        project_id: uuid::Uuid,
        account_id: uuid::Uuid,
    },
    ProjectMemberRoleChanged {
        project_id: uuid::Uuid,
        account_id: uuid::Uuid,
        old_role: String,
        new_role: String,
    },
    ContactCreated {
        contact_id: uuid::Uuid,
        organization_id: uuid::Uuid,
    },
    ContactsMerged {
        target_id: uuid::Uuid,
        source_ids: Vec<uuid::Uuid>,
        organization_id: uuid::Uuid,
    },
    MemberTagCreated {
        tag_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    MemberTagDeleted {
        tag_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TagAssigned {
        tag_id: uuid::Uuid,
        assignment_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TagUnassigned {
        tag_id: uuid::Uuid,
        assignment_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TaskTemplateCreated {
        template_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TaskTemplateDeleted {
        template_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TaskCreated {
        task_id: uuid::Uuid,
        project_id: uuid::Uuid,
        template_id: uuid::Uuid,
    },
    TaskStatusChanged {
        task_id: uuid::Uuid,
        project_id: uuid::Uuid,
        old_status: String,
        new_status: String,
    },
    TaskDeleted {
        task_id: uuid::Uuid,
        project_id: uuid::Uuid,
    },
    TodoCompleted {
        todo_id: uuid::Uuid,
        task_id: uuid::Uuid,
    },
}

/// In-process event bus backed by a Tokio broadcast channel.
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish a domain event to all subscribers.
    ///
    /// Returns the number of receivers that received the event.
    /// Returns 0 if there are no active subscribers.
    pub fn publish(&self, event: DomainEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to domain events.
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe();

        bus.publish(DomainEvent::SystemStarted);

        let event = rx.recv().await.expect("should receive event");
        assert!(matches!(event, DomainEvent::SystemStarted));
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_same_event() {
        let bus = EventBus::default();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let receivers = bus.publish(DomainEvent::SystemHealthCheck);
        assert_eq!(receivers, 2);

        let e1 = rx1.recv().await.expect("subscriber 1 should receive event");
        let e2 = rx2.recv().await.expect("subscriber 2 should receive event");

        assert!(matches!(e1, DomainEvent::SystemHealthCheck));
        assert!(matches!(e2, DomainEvent::SystemHealthCheck));
    }

    #[tokio::test]
    async fn member_joined_event() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe();

        let org_id = uuid::Uuid::new_v4();
        let account_id = uuid::Uuid::new_v4();

        bus.publish(DomainEvent::MemberJoined {
            organization_id: org_id,
            account_id,
        });

        let event = rx.recv().await.expect("should receive event");
        match event {
            DomainEvent::MemberJoined {
                organization_id,
                account_id: received_account_id,
            } => {
                assert_eq!(organization_id, org_id);
                assert_eq!(received_account_id, account_id);
            }
            _ => panic!("expected MemberJoined event"),
        }
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_returns_zero() {
        let bus = EventBus::default();
        let count = bus.publish(DomainEvent::SystemStarted);
        assert_eq!(count, 0);
    }
}
