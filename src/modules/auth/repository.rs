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
        name: &str,
    ) -> Result<Account, AuthError> {
        sqlx::query_as!(
            Account,
            "INSERT INTO accounts (id, email, name)
             VALUES ($1, $2, $3)
             RETURNING id, email, name, avatar_url, locale, bio,
                       profile_data, profile_schema, notification_preferences,
                       created_at, updated_at, deleted_at",
            id,
            email,
            name,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.constraint() == Some("uq_accounts_email") => {
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
        sqlx::query_as!(
            Account,
            "SELECT id, email, name, avatar_url, locale, bio,
                    profile_data, profile_schema, notification_preferences,
                    created_at, updated_at, deleted_at
             FROM accounts
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
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
        let account = sqlx::query_as!(
            Account,
            "SELECT id, email, name, avatar_url, locale, bio,
                    profile_data, profile_schema, notification_preferences,
                    created_at, updated_at, deleted_at
             FROM accounts
             WHERE email = $1 AND deleted_at IS NULL",
            email,
        )
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
        name: Option<&str>,
        avatar_url: Option<Option<&str>>,
        bio: Option<Option<&str>>,
        locale: Option<&str>,
    ) -> Result<Account, AuthError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let avatar_url = match avatar_url {
            Some(url) => url,
            None => current.avatar_url.as_deref(),
        };
        let bio = match bio {
            Some(b) => b,
            None => current.bio.as_deref(),
        };
        let locale = locale.unwrap_or(&current.locale);

        sqlx::query_as!(
            Account,
            "UPDATE accounts
             SET name = $2, avatar_url = $3, bio = $4, locale = $5, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, email, name, avatar_url, locale, bio,
                       profile_data, profile_schema, notification_preferences,
                       created_at, updated_at, deleted_at",
            id,
            name,
            avatar_url,
            bio,
            locale,
        )
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
        profile_data: &serde_json::Value,
        profile_schema: Option<&serde_json::Value>,
    ) -> Result<Account, AuthError> {
        if let Some(schema) = profile_schema {
            sqlx::query_as!(
                Account,
                "UPDATE accounts
                 SET profile_data = $2, profile_schema = $3, updated_at = NOW()
                 WHERE id = $1 AND deleted_at IS NULL
                 RETURNING id, email, name, avatar_url, locale, bio,
                           profile_data, profile_schema, notification_preferences,
                           created_at, updated_at, deleted_at",
                id,
                profile_data,
                schema,
            )
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::AccountNotFound)
        } else {
            sqlx::query_as!(
                Account,
                "UPDATE accounts
                 SET profile_data = $2, updated_at = NOW()
                 WHERE id = $1 AND deleted_at IS NULL
                 RETURNING id, email, name, avatar_url, locale, bio,
                           profile_data, profile_schema, notification_preferences,
                           created_at, updated_at, deleted_at",
                id,
                profile_data,
            )
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::AccountNotFound)
        }
    }

    /// Update account notification preferences.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::AccountNotFound` if not found.
    pub async fn update_notification_preferences(
        pool: &PgPool,
        id: Uuid,
        preferences: &serde_json::Value,
    ) -> Result<Account, AuthError> {
        sqlx::query_as!(
            Account,
            "UPDATE accounts
             SET notification_preferences = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, email, name, avatar_url, locale, bio,
                       profile_data, profile_schema, notification_preferences,
                       created_at, updated_at, deleted_at",
            id,
            preferences,
        )
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
        let result = sqlx::query!(
            "UPDATE accounts
             SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
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
        let cred = sqlx::query_as!(
            PasskeyCredential,
            "INSERT INTO passkey_credentials (id, account_id, credential_id, credential, name)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, account_id, credential_id, credential, name, created_at, last_used_at",
            id,
            account_id,
            credential_id,
            credential,
            name,
        )
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
        let creds = sqlx::query_as!(
            PasskeyCredential,
            "SELECT id, account_id, credential_id, credential, name, created_at, last_used_at
             FROM passkey_credentials
             WHERE account_id = $1
             ORDER BY created_at ASC",
            account_id,
        )
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
        let cred = sqlx::query_as!(
            PasskeyCredential,
            "SELECT id, account_id, credential_id, credential, name, created_at, last_used_at
             FROM passkey_credentials
             WHERE credential_id = $1",
            credential_id,
        )
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
        sqlx::query!(
            "UPDATE passkey_credentials SET last_used_at = NOW() WHERE id = $1",
            id,
        )
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
        sqlx::query!(
            "UPDATE passkey_credentials SET credential = $2 WHERE id = $1",
            id,
            credential,
        )
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
        let result = sqlx::query!(
            "DELETE FROM passkey_credentials WHERE id = $1 AND account_id = $2",
            id,
            account_id,
        )
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
        account_id: Uuid,
        email: &str,
        token_hash: &[u8],
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<MagicLinkToken, AuthError> {
        let token = sqlx::query_as!(
            MagicLinkToken,
            "INSERT INTO magic_link_tokens (id, account_id, email, token_hash, expires_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, account_id, email, token_hash, expires_at, used_at, created_at",
            id,
            account_id,
            email,
            token_hash,
            expires_at,
        )
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
        let token = sqlx::query_as!(
            MagicLinkToken,
            "SELECT id, account_id, email, token_hash, expires_at, used_at, created_at
             FROM magic_link_tokens
             WHERE token_hash = $1",
            token_hash,
        )
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
        sqlx::query!(
            "UPDATE magic_link_tokens SET used_at = NOW() WHERE id = $1",
            id,
        )
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
        let token = sqlx::query_as!(
            RefreshToken,
            "INSERT INTO refresh_tokens (id, account_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)
             RETURNING id, account_id, token_hash, expires_at, revoked_at, rotated_at, replaced_by, created_at",
            id,
            account_id,
            token_hash,
            expires_at,
        )
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
        let token = sqlx::query_as!(
            RefreshToken,
            "SELECT id, account_id, token_hash, expires_at, revoked_at, rotated_at, replaced_by, created_at
             FROM refresh_tokens
             WHERE token_hash = $1
             FOR UPDATE",
            token_hash,
        )
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
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = NOW(), replaced_by = $2 WHERE id = $1",
            id,
            replaced_by,
        )
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
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL",
            account_id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
