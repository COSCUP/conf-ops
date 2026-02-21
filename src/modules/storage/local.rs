use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::io::AsyncRead;

use super::backend::StorageBackend;
use super::error::StorageError;

/// Local filesystem storage backend.
pub struct LocalStorageBackend {
    base_path: PathBuf,
}

impl LocalStorageBackend {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    fn full_path(&self, storage_path: &str) -> PathBuf {
        self.base_path.join(storage_path)
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store(&self, storage_path: &str, data: &[u8]) -> Result<(), StorageError> {
        let full_path = self.full_path(storage_path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full_path, data).await?;
        Ok(())
    }

    async fn load(
        &self,
        storage_path: &str,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError> {
        let full_path = self.full_path(storage_path);
        let file = tokio::fs::File::open(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;
        Ok(Box::new(file))
    }

    async fn remove(&self, storage_path: &str) -> Result<(), StorageError> {
        let full_path = self.full_path(storage_path);
        let _ = tokio::fs::remove_file(&full_path).await;
        // Clean up empty parent directories up to base_path
        cleanup_empty_parents(&full_path, &self.base_path).await;
        Ok(())
    }

    async fn exists(&self, storage_path: &str) -> Result<bool, StorageError> {
        let full_path = self.full_path(storage_path);
        Ok(tokio::fs::try_exists(&full_path).await.unwrap_or(false))
    }
}

/// Remove empty parent directories up to (but not including) the base path.
async fn cleanup_empty_parents(file_path: &Path, base_path: &Path) {
    let mut current = file_path.parent();
    while let Some(dir) = current {
        if dir == base_path {
            break;
        }
        if tokio::fs::remove_dir(dir).await.is_err() {
            break;
        }
        current = dir.parent();
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;

    #[tokio::test]
    async fn store_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalStorageBackend::new(dir.path());

        backend
            .store("test/file.txt", b"hello")
            .await
            .expect("store should succeed");
        let mut reader = backend
            .load("test/file.txt")
            .await
            .expect("load should succeed");
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn load_missing_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalStorageBackend::new(dir.path());

        let result = backend.load("nonexistent.txt").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }

    #[tokio::test]
    async fn remove_and_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalStorageBackend::new(dir.path());

        backend.store("a/b.txt", b"data").await.unwrap();
        assert!(backend.exists("a/b.txt").await.unwrap());

        backend.remove("a/b.txt").await.unwrap();
        assert!(!backend.exists("a/b.txt").await.unwrap());
    }
}
