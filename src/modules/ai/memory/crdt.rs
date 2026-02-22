use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, Text, Transact, Update};

use crate::modules::core::crdt::repository::CrdtRepository;

use super::error::MemoryError;

const ENTITY_TYPE: &str = "memory";

/// Manages CRDT documents for memory content, enabling concurrent multi-user editing.
///
/// Uses the existing `crdt_operations` table with `entity_type = "memory"`
/// and `entity_id = memory_id`. Each memory's content is backed by a `yrs::Doc`
/// with a single `Text` type named `"content"`.
pub struct MemoryCrdtManager {
    pool: PgPool,
    doc_cache: Cache<Uuid, Arc<RwLock<Doc>>>,
}

impl MemoryCrdtManager {
    /// Create a new `MemoryCrdtManager` with a 5-minute idle TTL (max 500 entries).
    pub fn new(pool: PgPool) -> Self {
        let doc_cache = Cache::builder()
            .time_to_idle(Duration::from_secs(300))
            .max_capacity(500)
            .build();

        Self { pool, doc_cache }
    }

    /// Get or create a `Doc` for a memory, hydrating from stored operations if needed.
    async fn get_or_create_doc(&self, memory_id: Uuid) -> Result<Arc<RwLock<Doc>>, MemoryError> {
        if let Some(doc) = self.doc_cache.get(&memory_id).await {
            return Ok(doc);
        }

        let doc = Doc::new();

        // Hydrate from stored operations
        let operations =
            CrdtRepository::get_operations_by_entity(&self.pool, ENTITY_TYPE, memory_id)
                .await
                .map_err(MemoryError::Database)?;

        {
            let mut txn = doc.transact_mut();
            for op in &operations {
                if let Ok(update) = Update::decode_v1(&op.operation) {
                    let _ = txn.apply_update(update);
                }
            }
        }

        let arc = Arc::new(RwLock::new(doc));
        self.doc_cache.insert(memory_id, Arc::clone(&arc)).await;
        Ok(arc)
    }

    /// Initialize a memory's CRDT document with the given content.
    ///
    /// Creates a new `yrs::Doc`, inserts the initial text, and stores
    /// the operation in the database.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn initialize(
        &self,
        memory_id: Uuid,
        content: &str,
        created_by: Uuid,
    ) -> Result<(), MemoryError> {
        let doc = Doc::new();
        let update = {
            let text = doc.get_or_insert_text("content");
            let mut txn = doc.transact_mut();
            text.push(&mut txn, content);
            txn.encode_update_v1()
        };

        CrdtRepository::insert_operation(
            &self.pool,
            Uuid::now_v7(),
            ENTITY_TYPE,
            memory_id,
            &update,
            created_by,
        )
        .await
        .map_err(MemoryError::Database)?;

        let arc = Arc::new(RwLock::new(doc));
        self.doc_cache.insert(memory_id, arc).await;

        Ok(())
    }

    /// Apply an incremental CRDT update to a memory's document.
    ///
    /// Stores the operation and applies it to the in-memory doc.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn apply_update(
        &self,
        memory_id: Uuid,
        update_data: &[u8],
        applied_by: Uuid,
    ) -> Result<String, MemoryError> {
        let doc = self.get_or_create_doc(memory_id).await?;

        // Store the operation first
        CrdtRepository::insert_operation(
            &self.pool,
            Uuid::now_v7(),
            ENTITY_TYPE,
            memory_id,
            update_data,
            applied_by,
        )
        .await
        .map_err(MemoryError::Database)?;

        // Apply update
        let doc_guard = doc.write().await;
        {
            let parsed = Update::decode_v1(update_data)
                .map_err(|_| MemoryError::InvalidScopeType("Invalid CRDT update".to_string()))?;
            let mut txn = doc_guard.transact_mut();
            let _ = txn.apply_update(parsed);
        }

        // Read current text content
        let text = doc_guard.get_or_insert_text("content");
        let content = text.get_string(&doc_guard.transact());
        drop(doc_guard);

        Ok(content)
    }

    /// Get the current state vector for a memory's CRDT document.
    ///
    /// Used for computing incremental updates from a client's state.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn get_state_vector(&self, memory_id: Uuid) -> Result<Vec<u8>, MemoryError> {
        let doc = self.get_or_create_doc(memory_id).await?;
        let doc_guard = doc.read().await;
        let result = doc_guard.transact().state_vector().encode_v1();
        drop(doc_guard);
        Ok(result)
    }

    /// Get the current content text of a memory's CRDT document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::Database` on database failure.
    pub async fn get_content(&self, memory_id: Uuid) -> Result<String, MemoryError> {
        let doc = self.get_or_create_doc(memory_id).await?;
        let doc_guard = doc.read().await;
        let text = doc_guard.get_or_insert_text("content");
        let content = text.get_string(&doc_guard.transact());
        drop(doc_guard);
        Ok(content)
    }

    /// Evict a memory's document from the cache.
    pub async fn evict(&self, memory_id: Uuid) {
        self.doc_cache.invalidate(&memory_id).await;
    }
}
