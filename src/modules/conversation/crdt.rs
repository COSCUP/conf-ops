use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Array, Doc, ReadTxn, StateVector, Transact, Update, WriteTxn};

use crate::id::generate_id;
use crate::modules::core::crdt::repository::CrdtRepository;

use super::error::ConversationError;

/// Manages CRDT documents for task conversations.
///
/// Uses an in-memory cache of `yrs::Doc` instances with a 5-minute idle TTL.
/// When a document is evicted from cache, it can be reconstructed by replaying
/// operations from `crdt_operations`.
pub struct CrdtManager {
    pool: PgPool,
    doc_cache: Cache<Uuid, Arc<RwLock<Doc>>>,
}

impl CrdtManager {
    pub fn new(pool: PgPool) -> Self {
        let doc_cache = Cache::builder()
            .time_to_idle(Duration::from_secs(300))
            .max_capacity(500)
            .build();

        Self { pool, doc_cache }
    }

    /// Get or create a `Doc` for a task, hydrating from DB if needed.
    async fn get_or_create_doc(
        &self,
        task_id: Uuid,
    ) -> Result<Arc<RwLock<Doc>>, ConversationError> {
        if let Some(doc) = self.doc_cache.get(&task_id).await {
            return Ok(doc);
        }

        let doc = Doc::new();

        // Hydrate from stored operations
        let operations =
            CrdtRepository::get_operations_by_entity(&self.pool, "conversation", task_id)
                .await
                .map_err(ConversationError::Database)?;

        {
            let mut txn = doc.transact_mut();
            for op in &operations {
                if let Ok(update) = Update::decode_v1(&op.operation) {
                    let _ = txn.apply_update(update);
                }
            }
        }

        let doc = Arc::new(RwLock::new(doc));
        self.doc_cache.insert(task_id, Arc::clone(&doc)).await;
        Ok(doc)
    }

    /// Apply a CRDT update to a task's document.
    ///
    /// Persists the operation to `crdt_operations` and returns the merged update
    /// to broadcast to other clients.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn apply_update(
        &self,
        task_id: Uuid,
        update: &[u8],
        sender_id: Uuid,
    ) -> Result<Vec<u8>, ConversationError> {
        let doc = self.get_or_create_doc(task_id).await?;
        let doc_guard = doc.write().await;

        // Get state vector before applying
        let sv_before = doc_guard.transact().state_vector();

        // Apply update
        {
            let mut txn = doc_guard.transact_mut();
            let parsed_update =
                Update::decode_v1(update).map_err(|_| ConversationError::InvalidSourceType)?;
            let _ = txn.apply_update(parsed_update);
        }

        // Persist to database
        let op_id = generate_id();
        CrdtRepository::insert_operation(
            &self.pool,
            op_id,
            "conversation",
            task_id,
            update,
            sender_id,
        )
        .await
        .map_err(ConversationError::Database)?;

        // Encode the diff as an update for broadcasting
        let diff = doc_guard.transact().encode_state_as_update_v1(&sv_before);
        drop(doc_guard);

        Ok(diff)
    }

    /// Get the state vector for a task's CRDT document.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn get_state_vector(&self, task_id: Uuid) -> Result<Vec<u8>, ConversationError> {
        let doc = self.get_or_create_doc(task_id).await?;
        let doc_guard = doc.read().await;
        let result = doc_guard.transact().state_vector().encode_v1();
        drop(doc_guard);
        Ok(result)
    }

    /// Encode the document state as an update relative to a given state vector.
    ///
    /// Used for initial sync: client sends its state vector, server responds
    /// with the diff.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn encode_state_as_update(
        &self,
        task_id: Uuid,
        state_vector: &[u8],
    ) -> Result<Vec<u8>, ConversationError> {
        let doc = self.get_or_create_doc(task_id).await?;
        let doc_guard = doc.read().await;
        let sv = StateVector::decode_v1(state_vector)
            .map_err(|_| ConversationError::InvalidSourceType)?;
        let update = doc_guard.transact().encode_state_as_update_v1(&sv);
        drop(doc_guard);
        Ok(update)
    }

    /// Compact CRDT operations for a task by merging all operations into a single snapshot.
    ///
    /// This reduces the number of rows in `crdt_operations` and speeds up document
    /// reconstruction on cache miss.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn compact(&self, task_id: Uuid) -> Result<(), ConversationError> {
        let doc = self.get_or_create_doc(task_id).await?;
        let doc_guard = doc.read().await;

        // Encode the full document state as a single update
        let snapshot = doc_guard
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        drop(doc_guard);

        // Replace all operations with a single snapshot in a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(ConversationError::Database)?;

        CrdtRepository::delete_operations_by_entity(&mut *tx, "conversation", task_id)
            .await
            .map_err(ConversationError::Database)?;

        let op_id = generate_id();
        CrdtRepository::insert_operation_with_executor(
            &mut *tx,
            op_id,
            "conversation",
            task_id,
            &snapshot,
            Uuid::nil(),
        )
        .await
        .map_err(ConversationError::Database)?;

        tx.commit().await.map_err(ConversationError::Database)?;

        Ok(())
    }

    /// Compact all conversation CRDT documents that exceed the operation threshold.
    ///
    /// Queries the database for entity IDs with more than `threshold` operations,
    /// then compacts each one.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn compact_if_needed(&self, threshold: i64) -> Result<usize, ConversationError> {
        let entity_ids = CrdtRepository::list_entities_exceeding_threshold(
            &self.pool,
            "conversation",
            threshold,
        )
        .await
        .map_err(ConversationError::Database)?;

        let count = entity_ids.len();
        for task_id in entity_ids {
            if let Err(e) = self.compact(task_id).await {
                tracing::warn!("Failed to compact CRDT for task {task_id}: {e}");
            }
        }

        Ok(count)
    }

    /// Append a message summary to the `messages` Y.Array inside a write-locked `Doc`.
    fn append_to_messages_array(doc: &Doc, message_id: Uuid, created_at: &str) {
        let summary = serde_json::json!({
            "id": message_id.to_string(),
            "created_at": created_at,
        })
        .to_string();
        let messages = doc.transact_mut().get_or_insert_array("messages");
        messages.push_back(&mut doc.transact_mut(), summary);
    }

    /// Push a message summary into the Y.Array of the CRDT document.
    ///
    /// The Y.Array stores message summaries (`{ "id": "<uuid>", "created_at": "<iso>" }`)
    /// for ordering and real-time notification. Full content stays in the `messages` table.
    ///
    /// # Errors
    ///
    /// Returns `ConversationError::Database` on database failure.
    pub async fn push_message(
        &self,
        task_id: Uuid,
        message_id: Uuid,
        created_at: &str,
        sender_id: Uuid,
    ) -> Result<Vec<u8>, ConversationError> {
        let doc = self.get_or_create_doc(task_id).await?;
        let doc_guard = doc.write().await;

        let sv_before = doc_guard.transact().state_vector();

        Self::append_to_messages_array(&doc_guard, message_id, created_at);

        // Encode the update for persistence and broadcasting
        let update = doc_guard.transact().encode_state_as_update_v1(&sv_before);
        drop(doc_guard);

        // Persist
        let op_id = generate_id();
        CrdtRepository::insert_operation(
            &self.pool,
            op_id,
            "conversation",
            task_id,
            &update,
            sender_id,
        )
        .await
        .map_err(ConversationError::Database)?;

        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use yrs::updates::decoder::Decode;
    use yrs::updates::encoder::Encode;
    use yrs::{Array, Doc, ReadTxn, StateVector, Transact, Update};

    /// Verify that a Doc's state vector is non-empty after an update is applied.
    ///
    /// This exercises the core encode/apply round-trip that CrdtManager uses
    /// internally, without requiring a database connection.
    #[test]
    fn test_apply_update_and_get_state_vector() {
        // Client doc: insert a value into "items" array
        let client_doc = Doc::new();
        let array = client_doc.get_or_insert_array("items");
        let mut txn = client_doc.transact_mut();
        array.push_back(&mut txn, "task-value");
        drop(txn);

        // Encode the client update from an empty state vector
        let update_bytes = client_doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        // Server doc: apply the update and check state vector is non-empty
        let server_doc = Doc::new();
        {
            let mut txn = server_doc.transact_mut();
            let update = Update::decode_v1(&update_bytes).expect("valid update");
            let _ = txn.apply_update(update);
        }

        let sv_bytes = server_doc.transact().state_vector().encode_v1();
        assert!(
            !sv_bytes.is_empty(),
            "state vector must be non-empty after applying an update"
        );
    }

    /// Simulate two concurrent clients whose updates both get merged into a
    /// single document, verifying CRDT convergence.
    #[test]
    fn test_concurrent_updates_merge() {
        // Client A inserts "hello"
        let doc_a = Doc::new();
        let array_a = doc_a.get_or_insert_array("items");
        let mut txn_a = doc_a.transact_mut();
        array_a.push_back(&mut txn_a, "hello");
        drop(txn_a);
        let update_a = doc_a
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        // Client B inserts "world"
        let doc_b = Doc::new();
        let array_b = doc_b.get_or_insert_array("items");
        let mut txn_b = doc_b.transact_mut();
        array_b.push_back(&mut txn_b, "world");
        drop(txn_b);
        let update_b = doc_b
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        // Server doc: apply both concurrent updates
        let server_doc = Doc::new();
        {
            let mut txn = server_doc.transact_mut();
            let _ = txn.apply_update(Update::decode_v1(&update_a).expect("valid update_a"));
            let _ = txn.apply_update(Update::decode_v1(&update_b).expect("valid update_b"));
        }

        // Both values must be present in the merged array
        let server_array = server_doc.get_or_insert_array("items");
        let txn = server_doc.transact();
        let values: Vec<String> = server_array.iter(&txn).map(|v| v.to_string(&txn)).collect();

        assert_eq!(values.len(), 2, "merged doc must contain both values");
        assert!(
            values.contains(&"hello".to_string()),
            "must contain 'hello'"
        );
        assert!(
            values.contains(&"world".to_string()),
            "must contain 'world'"
        );
    }

    /// Verify that encode_state_as_update_v1 with an old state vector returns
    /// only the diff (i.e., operations after that state vector).
    ///
    /// The test strategy: record the state vector after phase-1 writes, make
    /// phase-2 writes, encode the diff relative to the phase-1 SV, then apply
    /// *both* the full update and the (base + diff) path to separate peer docs
    /// and confirm both end up with the same number of array elements.
    #[test]
    fn test_encode_state_as_update_diff() {
        let doc = Doc::new();
        let array = doc.get_or_insert_array("items");

        // Phase 1: insert initial content and snapshot the state vector
        {
            let mut txn = doc.transact_mut();
            array.push_back(&mut txn, "initial");
        }
        let sv_after_phase1 = doc.transact().state_vector();
        // Encode base state so a peer can be brought up to phase-1
        let base_update = doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        // Phase 2: insert additional content
        {
            let mut txn = doc.transact_mut();
            array.push_back(&mut txn, "added-later");
        }

        // diff must encode only what was added in phase 2
        let diff = doc.transact().encode_state_as_update_v1(&sv_after_phase1);
        assert!(!diff.is_empty(), "diff must be non-empty");

        // Peer A: receives the complete update from scratch
        let full_update = doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        let peer_full = Doc::new();
        {
            let mut txn = peer_full.transact_mut();
            let _ = txn.apply_update(Update::decode_v1(&full_update).expect("valid full_update"));
        }

        // Peer B: receives base state then the incremental diff
        let peer_incremental = Doc::new();
        {
            let mut txn = peer_incremental.transact_mut();
            let _ = txn.apply_update(Update::decode_v1(&base_update).expect("valid base_update"));
            let _ = txn.apply_update(Update::decode_v1(&diff).expect("valid diff"));
        }

        // Both peers must contain the same number of items
        let array_full = peer_full.get_or_insert_array("items");
        let array_incr = peer_incremental.get_or_insert_array("items");
        let len_full = array_full.len(&peer_full.transact());
        let len_incr = array_incr.len(&peer_incremental.transact());

        assert_eq!(len_full, 2, "full-update peer must have 2 items");
        assert_eq!(
            len_incr, 2,
            "incremental-update peer must also have 2 items after applying diff"
        );
    }

    /// Verify that state vectors round-trip through encode/decode without loss.
    #[test]
    fn test_state_vector_encoding() {
        let doc = Doc::new();
        let array = doc.get_or_insert_array("items");
        {
            let mut txn = doc.transact_mut();
            array.push_back(&mut txn, "entry-a");
            array.push_back(&mut txn, "entry-b");
        }

        let original_sv = doc.transact().state_vector();
        let encoded = original_sv.encode_v1();

        let decoded_sv =
            StateVector::decode_v1(&encoded).expect("state vector must decode without error");

        // Re-encode the decoded state vector and compare bytes
        let re_encoded = decoded_sv.encode_v1();
        assert_eq!(
            encoded, re_encoded,
            "state vector must be identical after encode → decode → encode round-trip"
        );

        // A doc whose state equals the state vector produces a no-op diff
        let diff = doc.transact().encode_state_as_update_v1(&decoded_sv);

        // Sync a peer from scratch, then verify applying the no-op diff is harmless
        let peer_doc = Doc::new();
        {
            let full = doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            let mut txn = peer_doc.transact_mut();
            let _ = txn.apply_update(Update::decode_v1(&full).expect("valid full"));
        }

        // Read transaction must be released before acquiring a write transaction
        let peer_array = peer_doc.get_or_insert_array("items");
        let item_count = {
            let txn = peer_doc.transact();
            peer_array.len(&txn)
        };
        assert_eq!(
            item_count, 2,
            "peer doc must contain both entries after sync"
        );

        // Applying the no-op diff must not error and must not change item count
        {
            let mut txn = peer_doc.transact_mut();
            let _ = txn.apply_update(Update::decode_v1(&diff).expect("valid empty diff"));
        }
        let item_count_after = {
            let txn = peer_doc.transact();
            peer_array.len(&txn)
        };
        assert_eq!(
            item_count_after, 2,
            "item count must be unchanged after applying a no-op diff"
        );
    }
}
