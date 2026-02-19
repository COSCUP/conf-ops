use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::error::AuthError;
use super::models::{Account, MagicLinkToken, PasskeyCredential, RefreshToken};

pub struct AccountRepository;

impl AccountRepository {
    /// Create a new account.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::EmailAlreadyExists` if the email is taken,
    /// or `AuthError::Database` on other failures.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        email: &str,
        display_name: &str,
    ) -> Result<Account, AuthError> {
        sqlx::query_as::<_, Account>(
            "INSERT INTO accounts (id, email, display_name)
             VALUES ($1, $2, $3)
             RETURNING id, email, display_name, avatar_url, locale,
                       profile, created_at, updated_at, deleted_at",
        )
        .bind(id)
        .bind(email)
        .bind(display_name)
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.constraint() == Some("accounts_email_key") => {
                AuthError::EmailAlreadyExists
            }
            _ => AuthError::Database(e),
        })
    }

    /// Get an account by ID (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `AuthError::AccountNotFound` if not found.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Account, AuthError> {
        sqlx::query_as::<_, Account>(
            "SELECT id, email, display_name, avatar_url, locale,
                    profile, created_at, updated_at, deleted_at
             FROM accounts
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthError::AccountNotFound)
    }

    /// Get an account by email (excludes soft-deleted).
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<Account>, AuthError> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, email, display_name, avatar_url, locale,
                    profile, created_at, updated_at, deleted_at
             FROM accounts
             WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(account)
    }

    /// Update account fields. `None` means keep current value.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::AccountNotFound` if not found.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        display_name: Option<&str>,
        avatar_url: Option<Option<&str>>,
        locale: Option<&str>,
    ) -> Result<Account, AuthError> {
        let current = Self::get_by_id(pool, id).await?;

        let display_name = display_name.unwrap_or(&current.display_name);
        let avatar_url = match avatar_url {
            Some(url) => url,
            None => current.avatar_url.as_deref(),
        };
        let locale = locale.unwrap_or(&current.locale);

        sqlx::query_as::<_, Account>(
            "UPDATE accounts
             SET display_name = $2, avatar_url = $3, locale = $4, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, email, display_name, avatar_url, locale,
                       profile, created_at, updated_at, deleted_at",
        )
        .bind(id)
        .bind(display_name)
        .bind(avatar_url)
        .bind(locale)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthError::AccountNotFound)
    }

    /// Update account profile JSONB.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::AccountNotFound` if not found.
    pub async fn update_profile(
        pool: &PgPool,
        id: Uuid,
        profile: &serde_json::Value,
    ) -> Result<Account, AuthError> {
        sqlx::query_as::<_, Account>(
            "UPDATE accounts
             SET profile = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, email, display_name, avatar_url, locale,
                       profile, created_at, updated_at, deleted_at",
        )
        .bind(id)
        .bind(profile)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthError::AccountNotFound)
    }

    /// Soft-delete an account.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::AccountNotFound` if not found.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
        let result = sqlx::query(
            "UPDATE accounts
             SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AuthError::AccountNotFound);
        }

        Ok(())
    }
}

pub struct PasskeyCredentialRepository;

impl PasskeyCredentialRepository {
    /// Create a passkey credential.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        account_id: Uuid,
        credential_id: &[u8],
        credential: &serde_json::Value,
        name: &str,
    ) -> Result<PasskeyCredential, AuthError> {
        let cred = sqlx::query_as::<_, PasskeyCredential>(
            "INSERT INTO passkey_credentials (id, account_id, credential_id, credential, name)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, account_id, credential_id, credential, name, created_at, last_used_at",
        )
        .bind(id)
        .bind(account_id)
        .bind(credential_id)
        .bind(credential)
        .bind(name)
        .fetch_one(pool)
        .await?;

        Ok(cred)
    }

    /// List all passkey credentials for an account.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn list_by_account(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<Vec<PasskeyCredential>, AuthError> {
        let creds = sqlx::query_as::<_, PasskeyCredential>(
            "SELECT id, account_id, credential_id, credential, name, created_at, last_used_at
             FROM passkey_credentials
             WHERE account_id = $1
             ORDER BY created_at ASC",
        )
        .bind(account_id)
        .fetch_all(pool)
        .await?;

        Ok(creds)
    }

    /// Find a passkey credential by its credential ID bytes.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn get_by_credential_id(
        pool: &PgPool,
        credential_id: &[u8],
    ) -> Result<Option<PasskeyCredential>, AuthError> {
        let cred = sqlx::query_as::<_, PasskeyCredential>(
            "SELECT id, account_id, credential_id, credential, name, created_at, last_used_at
             FROM passkey_credentials
             WHERE credential_id = $1",
        )
        .bind(credential_id)
        .fetch_optional(pool)
        .await?;

        Ok(cred)
    }

    /// Update `last_used_at` timestamp.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn update_last_used(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
        sqlx::query("UPDATE passkey_credentials SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update credential JSON data.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn update_credential(
        pool: &PgPool,
        id: Uuid,
        credential: &serde_json::Value,
    ) -> Result<(), AuthError> {
        sqlx::query("UPDATE passkey_credentials SET credential = $2 WHERE id = $1")
            .bind(id)
            .bind(credential)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete a passkey credential.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::CredentialNotFound` if not found.
    pub async fn delete(pool: &PgPool, id: Uuid, account_id: Uuid) -> Result<(), AuthError> {
        let result =
            sqlx::query("DELETE FROM passkey_credentials WHERE id = $1 AND account_id = $2")
                .bind(id)
                .bind(account_id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AuthError::CredentialNotFound);
        }

        Ok(())
    }
}

pub struct MagicLinkTokenRepository;

impl MagicLinkTokenRepository {
    /// Create a magic link token record.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        account_id: Option<Uuid>,
        email: &str,
        token_hash: &[u8],
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<MagicLinkToken, AuthError> {
        let token = sqlx::query_as::<_, MagicLinkToken>(
            "INSERT INTO magic_link_tokens (id, account_id, email, token_hash, expires_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, account_id, email, token_hash, expires_at, used_at, created_at",
        )
        .bind(id)
        .bind(account_id)
        .bind(email)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(token)
    }

    /// Find a magic link token by its hash.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn get_by_token_hash(
        pool: &PgPool,
        token_hash: &[u8],
    ) -> Result<Option<MagicLinkToken>, AuthError> {
        let token = sqlx::query_as::<_, MagicLinkToken>(
            "SELECT id, account_id, email, token_hash, expires_at, used_at, created_at
             FROM magic_link_tokens
             WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;

        Ok(token)
    }

    /// Mark a token as used.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn mark_used(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
        sqlx::query("UPDATE magic_link_tokens SET used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct RefreshTokenRepository;

impl RefreshTokenRepository {
    /// Create a refresh token record.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        account_id: Uuid,
        token_hash: &[u8],
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<RefreshToken, AuthError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            "INSERT INTO refresh_tokens (id, account_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)
             RETURNING id, account_id, token_hash, expires_at, revoked_at, replaced_by, created_at",
        )
        .bind(id)
        .bind(account_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(token)
    }

    /// Get a refresh token by hash with `FOR UPDATE` lock.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn get_by_token_hash_for_update(
        pool: &PgPool,
        token_hash: &[u8],
    ) -> Result<Option<RefreshToken>, AuthError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            "SELECT id, account_id, token_hash, expires_at, revoked_at, replaced_by, created_at
             FROM refresh_tokens
             WHERE token_hash = $1
             FOR UPDATE",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;

        Ok(token)
    }

    /// Revoke a refresh token, optionally recording a replacement.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn revoke(
        pool: &PgPool,
        id: Uuid,
        replaced_by: Option<Uuid>,
    ) -> Result<(), AuthError> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW(), replaced_by = $2 WHERE id = $1")
            .bind(id)
            .bind(replaced_by)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Revoke all active refresh tokens for an account.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::Database` on failure.
    pub async fn revoke_all_for_account(pool: &PgPool, account_id: Uuid) -> Result<(), AuthError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(account_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
