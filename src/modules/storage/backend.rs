use async_trait::async_trait;
use tokio::io::AsyncRead;

use super::error::StorageError;

/// Abstraction over file storage I/O operations.
///
/// Implementations can target local filesystem, S3, or other backends.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store data at the given storage path.
    async fn store(&self, storage_path: &str, data: &[u8]) -> Result<(), StorageError>;

    /// Load data from the given storage path as a stream.
    async fn load(
        &self,
        storage_path: &str,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError>;

    /// Remove the file at the given storage path.
    async fn remove(&self, storage_path: &str) -> Result<(), StorageError>;

    /// Check whether a file exists at the given storage path.
    async fn exists(&self, storage_path: &str) -> Result<bool, StorageError>;
}
