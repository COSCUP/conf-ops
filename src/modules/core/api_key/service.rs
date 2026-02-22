use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::ApiKeyError;
use super::models::{is_valid_scope, ApiKey, ApiKeyCreated, CreateApiKeyParams};
use super::repository::ApiKeyRepository;

const MAX_KEYS_PER_PROJECT: usize = 10;
const RAW_KEY_LENGTH: usize = 32;

/// The API key service manages creation, validation, and revocation of API keys.
pub struct ApiKeyService {
    pool: PgPool,
}

impl ApiKeyService {
    /// Create a new API key service.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new API key for a project.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError` if limit exceeded, scopes are invalid, or database fails.
    pub async fn create_api_key(
        &self,
        project_id: Uuid,
        name: String,
        permissions: serde_json::Value,
        created_by: Uuid,
    ) -> Result<ApiKeyCreated, ApiKeyError> {
        // Check limit
        let count = ApiKeyRepository::count_by_project(&self.pool, project_id).await?;
        if count >= i64::try_from(MAX_KEYS_PER_PROJECT).unwrap_or(10) {
            return Err(ApiKeyError::LimitExceeded(MAX_KEYS_PER_PROJECT));
        }

        // Validate scopes
        if let Some(scopes) = permissions.get("scopes").and_then(|v| v.as_array()) {
            for scope in scopes {
                if let Some(s) = scope.as_str() {
                    if !is_valid_scope(s) {
                        return Err(ApiKeyError::InvalidScope(s.to_string()));
                    }
                }
            }
        }

        // Generate random key
        let raw_key = generate_raw_key();
        let key_hash = hash_key(&raw_key);

        let params = CreateApiKeyParams {
            id: Uuid::now_v7(),
            project_id,
            name,
            key_hash,
            permissions,
            created_by,
        };

        let api_key_record = ApiKeyRepository::create(&self.pool, &params).await?;

        Ok(ApiKeyCreated {
            api_key_record,
            raw_key,
        })
    }

    /// Validate an API key and return the associated record.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::InvalidKey` if the key is not found or revoked.
    pub async fn validate_key(&self, raw_key: &str) -> Result<ApiKey, ApiKeyError> {
        let key_hash = hash_key(raw_key);
        let api_key = ApiKeyRepository::find_by_hash(&self.pool, &key_hash).await?;

        // Touch last used (fire and forget)
        let pool = self.pool.clone();
        let id = api_key.id;
        tokio::spawn(async move {
            let _ = ApiKeyRepository::touch_last_used(&pool, id).await;
        });

        Ok(api_key)
    }

    /// List API keys for a project (without exposing raw keys).
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::Database` on database failure.
    pub async fn list_api_keys(&self, project_id: Uuid) -> Result<Vec<ApiKey>, ApiKeyError> {
        ApiKeyRepository::list_by_project(&self.pool, project_id).await
    }

    /// Revoke (soft-delete) an API key.
    ///
    /// # Errors
    ///
    /// Returns `ApiKeyError::NotFound` if not found.
    pub async fn revoke_api_key(&self, key_id: Uuid) -> Result<(), ApiKeyError> {
        ApiKeyRepository::delete(&self.pool, key_id).await
    }

    /// Returns a reference to the pool for testing.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Hash an API key using SHA-256.
fn hash_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let result = hasher.finalize();
    // Use same hex encoding as webhook
    result.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Generate a cryptographically random API key.
fn generate_raw_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..RAW_KEY_LENGTH).map(|_| rng.gen()).collect();
    // Encode as base64url for URL-safe usage
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_key_deterministic() {
        let h1 = hash_key("test-key");
        let h2 = hash_key("test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_key_different_inputs_differ() {
        let h1 = hash_key("key1");
        let h2 = hash_key("key2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn generate_raw_key_produces_non_empty_string() {
        let key = generate_raw_key();
        assert!(!key.is_empty());
        // Base64url encoded 32 bytes should be ~43 chars
        assert!(key.len() > 20);
    }

    #[test]
    fn generate_raw_key_unique() {
        let k1 = generate_raw_key();
        let k2 = generate_raw_key();
        assert_ne!(k1, k2);
    }
}
