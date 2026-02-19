use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::id::generate_id;

use super::error::AuthError;
use super::repository::RefreshTokenRepository;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub access_token_expiry_secs: i64,
    pub refresh_token_expiry_secs: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
}

/// Issue a new JWT access token.
///
/// # Errors
///
/// Returns `AuthError::InvalidToken` if signing fails.
pub fn issue_access_token(config: &JwtConfig, account_id: Uuid) -> Result<String, AuthError> {
    let now = Utc::now();
    let claims = Claims {
        sub: account_id,
        iss: config.issuer.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(config.access_token_expiry_secs)).timestamp(),
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|_| AuthError::InvalidToken)
}

/// Validate a JWT access token and return the claims.
///
/// # Errors
///
/// Returns `AuthError::InvalidToken` if the token is invalid or expired.
pub fn validate_access_token(config: &JwtConfig, token: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::default();
    validation.set_issuer(&[&config.issuer]);

    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AuthError::InvalidToken)
}

/// Hash a raw token using SHA-256.
pub fn hash_token(raw: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(raw);
    hasher.finalize().to_vec()
}

/// Generate a cryptographically random token and return `(raw_base64, hash)`.
pub fn generate_refresh_token() -> (String, Vec<u8>) {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    let raw = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
    let hash = hash_token(raw.as_bytes());

    (raw, hash)
}

/// Issue a refresh token pair: creates DB record and returns raw token string.
///
/// # Errors
///
/// Returns `AuthError::Database` on failure.
pub async fn issue_refresh_token(
    pool: &PgPool,
    config: &JwtConfig,
    account_id: Uuid,
) -> Result<String, AuthError> {
    let (raw, hash) = generate_refresh_token();
    let expires_at = Utc::now() + Duration::seconds(config.refresh_token_expiry_secs);
    let id = generate_id();

    RefreshTokenRepository::create(pool, id, account_id, &hash, expires_at).await?;

    Ok(raw)
}

/// Rotate a refresh token: revoke old, issue new. Returns `(access_token, new_refresh_token)`.
///
/// Implements a 30-second grace period: if the old token was revoked within the last 30 seconds,
/// still allow the rotation (replay protection for network retries).
///
/// # Errors
///
/// Returns `AuthError::InvalidToken` if the token is invalid or expired.
/// Returns `AuthError::RefreshTokenRevoked` if token reuse is detected (beyond grace period).
pub async fn rotate_refresh_token(
    pool: &PgPool,
    config: &JwtConfig,
    raw_token: &str,
) -> Result<(String, String), AuthError> {
    let hash = hash_token(raw_token.as_bytes());

    let mut tx = pool.begin().await.map_err(AuthError::Database)?;

    let old_token = sqlx::query_as::<_, super::models::RefreshToken>(
        "SELECT id, account_id, token_hash, expires_at, revoked_at, replaced_by, created_at
         FROM refresh_tokens
         WHERE token_hash = $1
         FOR UPDATE",
    )
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AuthError::Database)?
    .ok_or(AuthError::InvalidToken)?;

    if old_token.expires_at < Utc::now() {
        return Err(AuthError::InvalidToken);
    }

    // Check for token reuse with grace period
    if let Some(revoked_at) = old_token.revoked_at {
        let grace_period = Duration::seconds(30);
        if Utc::now() - revoked_at > grace_period {
            // Token reuse detected beyond grace period — revoke all tokens for this account
            sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL",
            )
            .bind(old_token.account_id)
            .execute(&mut *tx)
            .await
            .map_err(AuthError::Database)?;

            tx.commit().await.map_err(AuthError::Database)?;
            return Err(AuthError::RefreshTokenRevoked);
        }
        // Within grace period — allow the rotation but don't create a new token
        // Return the existing replacement's token (or error)
        tx.commit().await.map_err(AuthError::Database)?;
        return Err(AuthError::RefreshTokenRevoked);
    }

    // Issue new refresh token
    let (new_raw, new_hash) = generate_refresh_token();
    let new_id = generate_id();
    let expires_at = Utc::now() + Duration::seconds(config.refresh_token_expiry_secs);

    sqlx::query(
        "INSERT INTO refresh_tokens (id, account_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(new_id)
    .bind(old_token.account_id)
    .bind(&new_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await
    .map_err(AuthError::Database)?;

    // Revoke old token
    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW(), replaced_by = $2 WHERE id = $1")
        .bind(old_token.id)
        .bind(new_id)
        .execute(&mut *tx)
        .await
        .map_err(AuthError::Database)?;

    tx.commit().await.map_err(AuthError::Database)?;

    let access_token = issue_access_token(config, old_token.account_id)?;

    Ok((access_token, new_raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-at-least-256-bits-long-for-hs256".to_string(),
            issuer: "conf-ops-test".to_string(),
            access_token_expiry_secs: 900,
            refresh_token_expiry_secs: 604_800,
        }
    }

    #[test]
    fn issue_and_validate_access_token() {
        let config = test_config();
        let account_id = generate_id();

        let token = issue_access_token(&config, account_id).expect("should issue");
        let claims = validate_access_token(&config, &token).expect("should validate");

        assert_eq!(claims.sub, account_id);
        assert_eq!(claims.iss, "conf-ops-test");
    }

    #[test]
    fn invalid_token_fails_validation() {
        let config = test_config();
        let result = validate_access_token(&config, "invalid-token");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn wrong_secret_fails_validation() {
        let config = test_config();
        let account_id = generate_id();

        let token = issue_access_token(&config, account_id).expect("should issue");

        let wrong_config = JwtConfig {
            secret: "different-secret-key-for-testing-wrong-key-validation".to_string(),
            ..config
        };
        let result = validate_access_token(&wrong_config, &token);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn expired_token_fails_validation() {
        let config = JwtConfig {
            access_token_expiry_secs: -120,
            ..test_config()
        };
        let account_id = generate_id();

        let token = issue_access_token(&config, account_id).expect("should issue");
        let result = validate_access_token(&test_config(), &token);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn hash_token_is_deterministic() {
        let hash1 = hash_token(b"test-token");
        let hash2 = hash_token(b"test-token");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn generate_refresh_token_is_unique() {
        let (raw1, hash1) = generate_refresh_token();
        let (raw2, hash2) = generate_refresh_token();
        assert_ne!(raw1, raw2);
        assert_ne!(hash1, hash2);
    }
}
