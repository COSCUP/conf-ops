use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::error::MemoryError;
use super::models::{
    CreateLibraryDocumentParams, CreateMemoryParams, InheritedMemories, LibraryDocument,
    LibraryDocumentVersion, Memory, MemoryVersion, ScopeType, UpdateLibraryDocumentParams,
    UpdateMemoryParams,
};
use super::repository::{LibraryDocumentRepository, MemoryRepository};

pub struct MemoryService {
    pool: PgPool,
    event_bus: EventBus,
    inherited_cache: Cache<Uuid, Arc<InheritedMemories>>,
}

impl MemoryService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        let inherited_cache = Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .max_capacity(1_000)
            .build();

        Self {
            pool,
            event_bus,
            inherited_cache,
        }
    }

    /// Start the cache invalidation listener for `MemoryUpserted` events.
    pub fn start_cache_invalidation(&self, event_bus: &EventBus) -> tokio::task::JoinHandle<()> {
        let cache = self.inherited_cache.clone();
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(DomainEvent::MemoryUpserted {
                        scope_type,
                        scope_id,
                        ..
                    }) => {
                        Self::invalidate_by_scope(&cache, &scope_type, scope_id).await;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            "Memory cache invalidation handler lagged, skipped {n} event(s)"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Event bus closed, stopping memory cache invalidation");
                        break;
                    }
                }
            }
        })
    }

    async fn invalidate_by_scope(
        cache: &Cache<Uuid, Arc<InheritedMemories>>,
        scope_type: &str,
        scope_id: Uuid,
    ) {
        if scope_type == "task" {
            cache.invalidate(&scope_id).await;
        } else {
            cache.invalidate_all();
        }
    }

    async fn validate_scope(
        &self,
        scope_type: &ScopeType,
        scope_id: Uuid,
    ) -> Result<(), MemoryError> {
        let exists = match scope_type {
            ScopeType::Account => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
            ScopeType::Organization => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1 AND deleted_at IS NULL) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
            ScopeType::Project => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND deleted_at IS NULL) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
            ScopeType::MemberTag => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM member_tags WHERE id = $1 AND deleted_at IS NULL) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
            ScopeType::TaskTemplate => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM task_templates WHERE id = $1 AND deleted_at IS NULL) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
            ScopeType::Task => {
                sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM tasks WHERE id = $1 AND deleted_at IS NULL) AS "exists!""#,
                    scope_id,
                )
                .fetch_one(&self.pool)
                .await?
            }
        };

        if !exists {
            return Err(MemoryError::InvalidScopeId {
                scope_type: scope_type.to_string(),
                scope_id,
            });
        }
        Ok(())
    }

    /// Create a new memory with scope validation.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` on invalid scope, missing library document, or database failure.
    pub async fn create_memory(&self, params: &CreateMemoryParams) -> Result<Memory, MemoryError> {
        self.validate_scope(&params.scope_type, params.scope_id)
            .await?;

        if let Some(library_ref) = params.library_ref {
            LibraryDocumentRepository::get_by_id(&self.pool, library_ref).await?;
        }

        let memory = MemoryRepository::create(&self.pool, params).await?;

        // Inline cache invalidation before publishing event
        Self::invalidate_by_scope(&self.inherited_cache, &memory.scope_type, memory.scope_id).await;

        self.event_bus.publish(DomainEvent::MemoryUpserted {
            memory_id: memory.id,
            scope_type: memory.scope_type.clone(),
            scope_id: memory.scope_id,
        });

        Ok(memory)
    }

    /// Get a memory by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn get_memory(&self, id: Uuid) -> Result<Memory, MemoryError> {
        MemoryRepository::get_by_id(&self.pool, id).await
    }

    /// List memories by scope with cursor-based pagination.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::InvalidScopeType` or database error.
    pub async fn list_memories(
        &self,
        scope_type: &str,
        scope_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<Memory>, MemoryError> {
        scope_type
            .parse::<ScopeType>()
            .map_err(|_| MemoryError::InvalidScopeType(scope_type.to_string()))?;

        MemoryRepository::list_by_scope(&self.pool, scope_type, scope_id, cursor, limit).await
    }

    /// Update a memory's content and/or library reference.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` on not found, missing library document, or database failure.
    pub async fn update_memory(
        &self,
        id: Uuid,
        params: &UpdateMemoryParams,
        changed_by: Uuid,
    ) -> Result<Memory, MemoryError> {
        if let Some(library_ref) = params.library_ref {
            LibraryDocumentRepository::get_by_id(&self.pool, library_ref).await?;
        }

        let memory = MemoryRepository::update(&self.pool, id, params, changed_by).await?;

        // Inline cache invalidation before publishing event
        Self::invalidate_by_scope(&self.inherited_cache, &memory.scope_type, memory.scope_id).await;

        self.event_bus.publish(DomainEvent::MemoryUpserted {
            memory_id: memory.id,
            scope_type: memory.scope_type.clone(),
            scope_id: memory.scope_id,
        });

        Ok(memory)
    }

    /// Soft-delete a memory.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), MemoryError> {
        let memory = MemoryRepository::get_by_id(&self.pool, id).await?;
        MemoryRepository::soft_delete(&self.pool, id).await?;

        // Inline cache invalidation before publishing event
        Self::invalidate_by_scope(&self.inherited_cache, &memory.scope_type, memory.scope_id).await;

        self.event_bus.publish(DomainEvent::MemoryUpserted {
            memory_id: id,
            scope_type: memory.scope_type,
            scope_id: memory.scope_id,
        });

        Ok(())
    }

    /// List version history for a memory.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::NotFound` or database error.
    pub async fn list_memory_versions(
        &self,
        memory_id: Uuid,
    ) -> Result<Vec<MemoryVersion>, MemoryError> {
        MemoryRepository::get_by_id(&self.pool, memory_id).await?;
        MemoryRepository::list_versions(&self.pool, memory_id).await
    }

    /// Collect inherited memories for a task through the inheritance chain.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` on database failure.
    pub async fn get_inherited_memories(
        &self,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Arc<InheritedMemories>, MemoryError> {
        if let Some(cached) = self.inherited_cache.get(&task_id).await {
            return Ok(cached);
        }

        let inherited =
            MemoryRepository::collect_inherited(&self.pool, task_id, account_id).await?;
        let arc = Arc::new(inherited);
        self.inherited_cache.insert(task_id, Arc::clone(&arc)).await;
        Ok(arc)
    }

    /// Create a new library document with scope validation.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` on invalid scope or database failure.
    pub async fn create_library_document(
        &self,
        params: &CreateLibraryDocumentParams,
    ) -> Result<LibraryDocument, MemoryError> {
        self.validate_scope(&params.scope_type, params.scope_id)
            .await?;
        LibraryDocumentRepository::create(&self.pool, params).await
    }

    /// Get a library document by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn get_library_document(&self, id: Uuid) -> Result<LibraryDocument, MemoryError> {
        LibraryDocumentRepository::get_by_id(&self.pool, id).await
    }

    /// List library documents by scope with cursor-based pagination.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::InvalidScopeType` or database error.
    pub async fn list_library_documents(
        &self,
        scope_type: &str,
        scope_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<LibraryDocument>, MemoryError> {
        scope_type
            .parse::<ScopeType>()
            .map_err(|_| MemoryError::InvalidScopeType(scope_type.to_string()))?;

        LibraryDocumentRepository::list_by_scope(&self.pool, scope_type, scope_id, cursor, limit)
            .await
    }

    /// Update a library document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn update_library_document(
        &self,
        id: Uuid,
        params: &UpdateLibraryDocumentParams,
        changed_by: Uuid,
    ) -> Result<LibraryDocument, MemoryError> {
        LibraryDocumentRepository::update(&self.pool, id, params, changed_by).await
    }

    /// Soft-delete a library document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn delete_library_document(&self, id: Uuid) -> Result<(), MemoryError> {
        LibraryDocumentRepository::soft_delete(&self.pool, id).await
    }

    /// List version history for a library document.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::LibraryDocumentNotFound` or database error.
    pub async fn list_library_document_versions(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<LibraryDocumentVersion>, MemoryError> {
        LibraryDocumentRepository::get_by_id(&self.pool, document_id).await?;
        LibraryDocumentRepository::list_versions(&self.pool, document_id).await
    }
}
