use std::sync::Arc;

use sqlx::PgPool;
use tokio::io::AsyncRead;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::backend::StorageBackend;
use super::error::StorageError;
use super::models::{FileMetadata, FileRecord};
use super::repository::{CreateFileParams, FileMetadataRepository, FileRepository};

/// Valid scope types for file uploads.
const VALID_SCOPE_TYPES: &[&str] = &["task", "project"];

/// Configurable storage size limits.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub max_image_size: u64,
    pub max_document_size: u64,
    pub max_file_size: u64,
    pub cleanup_grace_period_secs: i64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_image_size: 10 * 1024 * 1024,         // 10 MB
            max_document_size: 50 * 1024 * 1024,      // 50 MB
            max_file_size: 20 * 1024 * 1024,          // 20 MB
            cleanup_grace_period_secs: 7 * 24 * 3600, // 7 days
        }
    }
}

/// Parameters for file upload.
pub struct UploadFileParams<'a> {
    pub filename: &'a str,
    pub mime_type: &'a str,
    pub data: &'a [u8],
    pub scope_type: &'a str,
    pub scope_id: Uuid,
    pub uploaded_by: Uuid,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
}

pub struct FileService {
    pool: PgPool,
    event_bus: EventBus,
    backend: Arc<dyn StorageBackend>,
    config: StorageConfig,
}

impl FileService {
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        backend: Arc<dyn StorageBackend>,
        config: StorageConfig,
    ) -> Self {
        Self {
            pool,
            event_bus,
            backend,
            config,
        }
    }

    /// Upload a file: validate -> write to backend -> create DB record -> publish event.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on validation failure, I/O error, or database error.
    pub async fn upload_file(
        &self,
        params: &UploadFileParams<'_>,
    ) -> Result<FileRecord, StorageError> {
        // Validate scope type
        if !VALID_SCOPE_TYPES.contains(&params.scope_type) {
            return Err(StorageError::InvalidScopeType(
                params.scope_type.to_string(),
            ));
        }

        // Validate MIME type
        validate_mime_type(params.mime_type)?;

        // Validate file size
        let file_size = params.data.len() as u64;
        self.validate_file_size(params.mime_type, file_size)?;

        // Generate file ID and storage path
        let file_id = generate_id();
        let now = chrono::Utc::now();
        let storage_path = format!(
            "{scope_type}/{scope_id}/{year}/{month:02}/{file_id}/{filename}",
            scope_type = params.scope_type,
            scope_id = params.scope_id,
            year = now.format("%Y"),
            month = now.format("%m"),
            filename = params.filename,
        );

        // Write file via backend
        self.backend.store(&storage_path, params.data).await?;

        // Create DB record
        let file_size_i64 = i64::try_from(file_size).map_err(|_| {
            StorageError::FileTooLarge("File size exceeds maximum representable value".to_string())
        })?;
        let record = FileRepository::create(
            &self.pool,
            &CreateFileParams {
                id: file_id,
                filename: params.filename,
                mime_type: params.mime_type,
                file_size: file_size_i64,
                storage_path: &storage_path,
                scope_type: params.scope_type,
                scope_id: params.scope_id,
                uploaded_by: params.uploaded_by,
                organization_id: params.organization_id,
                project_id: params.project_id,
                task_id: params.task_id,
            },
        )
        .await
        .map_err(|e| {
            // Best-effort cleanup of written file on DB failure
            let backend = Arc::clone(&self.backend);
            let path = storage_path.clone();
            tokio::spawn(async move {
                let _ = backend.remove(&path).await;
            });
            e
        })?;

        self.event_bus.publish(DomainEvent::FileUploaded {
            file_id,
            scope_type: params.scope_type.to_string(),
            scope_id: params.scope_id,
            uploaded_by: params.uploaded_by,
        });

        Ok(record)
    }

    /// Get file metadata by ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist.
    pub async fn get_file(&self, file_id: Uuid) -> Result<FileRecord, StorageError> {
        FileRepository::get_by_id(&self.pool, file_id).await
    }

    /// Download file content from the storage backend as a stream.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist.
    pub async fn download_file(
        &self,
        file_id: Uuid,
    ) -> Result<(FileRecord, Box<dyn AsyncRead + Send + Unpin>), StorageError> {
        let record = FileRepository::get_by_id(&self.pool, file_id).await?;
        let stream = self.backend.load(&record.storage_path).await?;
        Ok((record, stream))
    }

    /// Set (upsert) a metadata key-value pair for a file.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist.
    /// Returns `StorageError::Database` on database failure.
    pub async fn set_file_metadata(
        &self,
        file_id: Uuid,
        key: &str,
        value: &str,
    ) -> Result<FileMetadata, StorageError> {
        FileRepository::get_by_id(&self.pool, file_id).await?;
        let id = generate_id();
        FileMetadataRepository::upsert(&self.pool, id, file_id, key, value).await
    }

    /// Get all metadata key-value pairs for a file.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn get_file_metadata(
        &self,
        file_id: Uuid,
    ) -> Result<Vec<FileMetadata>, StorageError> {
        FileMetadataRepository::list_by_file(&self.pool, file_id).await
    }

    /// Delete a specific metadata key for a file.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Database` on database failure.
    pub async fn delete_file_metadata(&self, file_id: Uuid, key: &str) -> Result<(), StorageError> {
        FileMetadataRepository::delete_by_key(&self.pool, file_id, key).await
    }

    /// Soft-delete a file + publish event.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the file does not exist.
    pub async fn delete_file(&self, file_id: Uuid) -> Result<(), StorageError> {
        let record = FileRepository::get_by_id(&self.pool, file_id).await?;
        FileRepository::soft_delete(&self.pool, file_id).await?;

        self.event_bus.publish(DomainEvent::FileDeleted {
            file_id,
            scope_type: record.scope_type,
            scope_id: record.scope_id,
        });

        Ok(())
    }

    /// Clean up orphaned files past the grace period.
    /// Deletes files from storage and hard-deletes DB records.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on I/O or database error.
    pub async fn cleanup_orphaned_files(&self) -> Result<u64, StorageError> {
        let orphans =
            FileRepository::find_orphaned_files(&self.pool, self.config.cleanup_grace_period_secs)
                .await?;

        let mut cleaned = 0u64;
        for record in orphans {
            let _ = self.backend.remove(&record.storage_path).await;
            FileRepository::hard_delete(&self.pool, record.id).await?;
            cleaned += 1;
        }

        Ok(cleaned)
    }

    /// Validate file size based on MIME type category.
    fn validate_file_size(&self, mime_type: &str, size: u64) -> Result<(), StorageError> {
        let max_size = if mime_type.starts_with("image/") {
            self.config.max_image_size
        } else if mime_type.starts_with("application/") {
            self.config.max_document_size
        } else {
            self.config.max_file_size
        };

        if size > max_size {
            return Err(StorageError::FileTooLarge(format!(
                "File size {size} bytes exceeds maximum {max_size} bytes for type {mime_type}"
            )));
        }

        Ok(())
    }
}

/// Validate MIME type against the whitelist.
fn validate_mime_type(mime_type: &str) -> Result<(), StorageError> {
    let allowed = mime_type.starts_with("image/")
        || mime_type.starts_with("text/")
        || matches!(
            mime_type,
            "application/pdf"
                | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                | "application/vnd.ms-excel"
                | "application/msword"
        );

    if !allowed {
        return Err(StorageError::UnsupportedMimeType(mime_type.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_mime_types() {
        assert!(validate_mime_type("image/png").is_ok());
        assert!(validate_mime_type("image/jpeg").is_ok());
        assert!(validate_mime_type("text/plain").is_ok());
        assert!(validate_mime_type("text/csv").is_ok());
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(validate_mime_type(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        )
        .is_ok());
        assert!(validate_mime_type(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
        .is_ok());
        assert!(validate_mime_type("application/vnd.ms-excel").is_ok());
        assert!(validate_mime_type("application/msword").is_ok());
    }

    #[test]
    fn rejects_invalid_mime_types() {
        assert!(validate_mime_type("application/zip").is_err());
        assert!(validate_mime_type("application/octet-stream").is_err());
        assert!(validate_mime_type("video/mp4").is_err());
        assert!(validate_mime_type("audio/mpeg").is_err());
    }

    #[test]
    fn validates_image_size_limit() {
        let config = StorageConfig::default();
        let service_config = config.clone();
        assert!(validate_size_helper(
            &service_config,
            "image/png",
            service_config.max_image_size
        ));
        assert!(!validate_size_helper(
            &service_config,
            "image/png",
            service_config.max_image_size + 1
        ));
    }

    #[test]
    fn validates_document_size_limit() {
        let config = StorageConfig::default();
        assert!(validate_size_helper(
            &config,
            "application/pdf",
            config.max_document_size
        ));
        assert!(!validate_size_helper(
            &config,
            "application/pdf",
            config.max_document_size + 1
        ));
    }

    #[test]
    fn validates_other_size_limit() {
        let config = StorageConfig::default();
        assert!(validate_size_helper(
            &config,
            "text/plain",
            config.max_file_size
        ));
        assert!(!validate_size_helper(
            &config,
            "text/plain",
            config.max_file_size + 1
        ));
    }

    /// Helper to check size validation without needing a full `FileService`.
    fn validate_size_helper(config: &StorageConfig, mime_type: &str, size: u64) -> bool {
        let max_size = if mime_type.starts_with("image/") {
            config.max_image_size
        } else if mime_type.starts_with("application/") {
            config.max_document_size
        } else {
            config.max_file_size
        };
        size <= max_size
    }
}
