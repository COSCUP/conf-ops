use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::id::generate_id;
use crate::modules::email::EmailService;

use super::error::AuthError;
use super::jwt::{issue_access_token, issue_refresh_token, JwtConfig};
use super::magic_link::{generate_magic_link_token, hash_magic_link_token};
use super::repository::{AccountRepository, MagicLinkTokenRepository};

pub struct AuthService {
    pool: PgPool,
    jwt_config: JwtConfig,
    email_service: Arc<dyn EmailService>,
    frontend_url: String,
}

impl AuthService {
    pub fn new(
        pool: PgPool,
        jwt_config: JwtConfig,
        email_service: Arc<dyn EmailService>,
        config: &AppConfig,
    ) -> Self {
        Self {
            pool,
            jwt_config,
            email_service,
            frontend_url: config.frontend_url.clone(),
        }
    }

    /// Request a magic link to be sent to the given email.
    /// Always returns Ok (consistent response regardless of email existence).
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on database or email sending failures.
    pub async fn request_magic_link(&self, email: &str) -> Result<(), AuthError> {
        let account = AccountRepository::get_by_email(&self.pool, email).await?;
        let account_id = account.as_ref().map(|a| a.id);

        let (raw_token, token_hash) = generate_magic_link_token();
        let expires_at = Utc::now() + Duration::minutes(15);
        let token_id = generate_id();

        MagicLinkTokenRepository::create(
            &self.pool,
            token_id,
            account_id,
            email,
            &token_hash,
            expires_at,
        )
        .await?;

        let magic_link_url = format!("{}/auth/magic-link?token={}", self.frontend_url, raw_token);

        let html_body = format!(
            "<h2>Login to Conf-Ops</h2>\
             <p>Click the link below to log in:</p>\
             <p><a href=\"{magic_link_url}\">{magic_link_url}</a></p>\
             <p>This link expires in 15 minutes.</p>"
        );

        self.email_service
            .send(email, "Login to Conf-Ops", &html_body)
            .await
            .map_err(AuthError::EmailSend)?;

        Ok(())
    }

    /// Verify a magic link token and return `(access_token, refresh_token, account_id)`.
    /// Creates a new account if the email doesn't exist yet.
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidToken` if token is invalid, expired, or already used.
    pub async fn verify_magic_link(
        &self,
        raw_token: &str,
    ) -> Result<(String, String, Uuid), AuthError> {
        let token_hash = hash_magic_link_token(raw_token);

        let token_record = MagicLinkTokenRepository::get_by_token_hash(&self.pool, &token_hash)
            .await?
            .ok_or(AuthError::InvalidToken)?;

        if token_record.used_at.is_some() {
            return Err(AuthError::TokenAlreadyUsed);
        }

        if token_record.expires_at < Utc::now() {
            return Err(AuthError::InvalidToken);
        }

        MagicLinkTokenRepository::mark_used(&self.pool, token_record.id).await?;

        let account_id = if let Some(id) = token_record.account_id {
            id
        } else {
            let new_id = generate_id();
            let display_name = token_record
                .email
                .split('@')
                .next()
                .unwrap_or("User")
                .to_string();
            AccountRepository::create(&self.pool, new_id, &token_record.email, &display_name)
                .await?;
            new_id
        };

        let access_token = issue_access_token(&self.jwt_config, account_id)?;
        let refresh_token = issue_refresh_token(&self.pool, &self.jwt_config, account_id).await?;

        Ok((access_token, refresh_token, account_id))
    }
}
