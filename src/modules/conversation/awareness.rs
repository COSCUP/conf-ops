use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-connection awareness state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessState {
    pub member_id: Uuid,
    pub display_name: String,
    pub color: String,
    pub cursor_position: Option<u64>,
    pub is_typing: bool,
    pub last_active: DateTime<Utc>,
}

/// Manages awareness states for all tasks.
///
/// `DashMap<task_id, DashMap<member_id, AwarenessState>>`.
pub struct AwarenessManager {
    states: DashMap<Uuid, DashMap<Uuid, AwarenessState>>,
}

impl AwarenessManager {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    /// Update awareness state for a member in a task.
    pub fn update(&self, task_id: Uuid, state: AwarenessState) {
        let room = self.states.entry(task_id).or_default();
        room.insert(state.member_id, state);
    }

    /// Remove a member's awareness from a task.
    pub fn remove(&self, task_id: Uuid, member_id: Uuid) {
        if let Some(room) = self.states.get(&task_id) {
            room.remove(&member_id);
            if room.is_empty() {
                drop(room);
                self.states.remove(&task_id);
            }
        }
    }

    /// Get all awareness states for a task.
    pub fn get_all(&self, task_id: Uuid) -> Vec<AwarenessState> {
        self.states
            .get(&task_id)
            .map(|room| room.iter().map(|entry| entry.value().clone()).collect())
            .unwrap_or_default()
    }

    /// Remove stale entries older than the given threshold.
    pub fn cleanup_stale(&self, threshold: DateTime<Utc>) {
        let empty_rooms: Vec<Uuid> = self
            .states
            .iter()
            .filter_map(|entry| {
                entry.retain(|_, state| state.last_active > threshold);
                if entry.is_empty() {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();

        for room_id in empty_rooms {
            self.states.remove(&room_id);
        }
    }
}

impl Default for AwarenessManager {
    fn default() -> Self {
        Self::new()
    }
}
