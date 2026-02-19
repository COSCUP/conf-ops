use tokio::sync::broadcast;

/// Domain events emitted by various modules.
///
/// New variants will be added as modules are implemented in later phases.
#[derive(Debug, Clone)]
pub enum DomainEvent {
    SystemStarted,
    SystemHealthCheck,
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
    async fn publish_with_no_subscribers_returns_zero() {
        let bus = EventBus::default();
        let count = bus.publish(DomainEvent::SystemStarted);
        assert_eq!(count, 0);
    }
}
