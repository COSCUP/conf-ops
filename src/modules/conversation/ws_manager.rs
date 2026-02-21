use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

/// A message sent from the server to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerWsMessage {
    /// CRDT sync: diff update (initial sync response).
    SyncDiff { update: Vec<u8> },
    /// CRDT update from another client.
    PeerUpdate { update: Vec<u8> },
    /// Awareness state change.
    AwarenessChange { entries: Vec<AwarenessEntry> },
    /// Write operation rejected (stale `lastSeenMessageId`).
    WriteRejected {
        reason: String,
        latest_message_id: Uuid,
    },
}

/// A message sent from a WebSocket client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientWsMessage {
    /// CRDT sync step 1: client sends state vector.
    SyncStep1 { state_vector: Vec<u8> },
    /// CRDT sync step 2: client sends local update.
    SyncStep2 { update: Vec<u8> },
    /// Awareness state update.
    AwarenessUpdate { state: AwarenessStatePayload },
    /// Write operation with stale-check.
    WriteOperation {
        update: Vec<u8>,
        last_seen_message_id: Uuid,
    },
}

/// Awareness payload sent by clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessStatePayload {
    pub cursor_position: Option<u64>,
    pub is_typing: bool,
}

/// Awareness entry broadcast to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessEntry {
    pub member_id: Uuid,
    pub display_name: String,
    pub color: String,
    pub cursor_position: Option<u64>,
    pub is_typing: bool,
}

/// Per-room state.
struct RoomState {
    sender: broadcast::Sender<ServerWsMessage>,
    connection_count: usize,
}

/// Manages WebSocket rooms (one per task).
///
/// Each task gets a `broadcast::Sender` for server-to-client fan-out.
pub struct WsManager {
    rooms: DashMap<Uuid, RoomState>,
    max_connections_per_task: usize,
}

impl WsManager {
    pub fn new(max_connections_per_task: usize) -> Self {
        Self {
            rooms: DashMap::new(),
            max_connections_per_task,
        }
    }

    /// Join a room. Returns a `broadcast::Receiver` for receiving messages
    /// and an `Arc<broadcast::Sender>` for sending.
    ///
    /// Returns `None` if the room is at capacity.
    pub fn join(
        &self,
        task_id: Uuid,
    ) -> Option<(
        Arc<broadcast::Sender<ServerWsMessage>>,
        broadcast::Receiver<ServerWsMessage>,
    )> {
        let mut room = self.rooms.entry(task_id).or_insert_with(|| {
            let (sender, _) = broadcast::channel(256);
            RoomState {
                sender,
                connection_count: 0,
            }
        });

        if room.connection_count >= self.max_connections_per_task {
            return None;
        }

        room.connection_count += 1;
        let receiver = room.sender.subscribe();
        let sender = Arc::new(room.sender.clone());
        drop(room);
        Some((sender, receiver))
    }

    /// Leave a room, decrementing the connection count.
    ///
    /// Cleans up the room if it becomes empty.
    pub fn leave(&self, task_id: Uuid) {
        let should_remove = self.rooms.get_mut(&task_id).is_some_and(|mut room| {
            room.connection_count = room.connection_count.saturating_sub(1);
            room.connection_count == 0
        });

        if should_remove {
            self.rooms.remove(&task_id);
        }
    }

    /// Broadcast a message to all clients in a room.
    pub fn broadcast(&self, task_id: Uuid, message: ServerWsMessage) {
        if let Some(room) = self.rooms.get(&task_id) {
            // Ignore send errors (no receivers).
            let _ = room.sender.send(message);
        }
    }

    /// Get the current connection count for a task.
    pub fn connection_count(&self, task_id: Uuid) -> usize {
        self.rooms
            .get(&task_id)
            .map_or(0, |room| room.connection_count)
    }
}
