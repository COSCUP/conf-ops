use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::*;
use webauthn_rs::Webauthn;

use crate::config::AppConfig;
use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::email::EmailService;

use super::error::AuthError;
use super::jwt::{issue_access_token, issue_refresh_token, JwtConfig};
use super::magic_link::{generate_magic_link_token, hash_magic_link_token};
use super::passkey::ChallengeStore;
use super::repository::{AccountRepository, MagicLinkTokenRepository, PasskeyCredentialRepository};

pub struct AuthService {
    pool: PgPool,
    jwt_config: JwtConfig,
    email_service: Arc<dyn EmailService>,
    event_bus: EventBus,
    frontend_url: String,
    webauthn: Arc<Webauthn>,
    reg_challenges: ChallengeStore<PasskeyRegistration>,
    auth_challenges: ChallengeStore<DiscoverableAuthentication>,
}

impl AuthService {
    pub fn new(
        pool: PgPool,
        jwt_config: JwtConfig,
        email_service: Arc<dyn EmailService>,
        webauthn: Arc<Webauthn>,
        event_bus: EventBus,
        config: &AppConfig,
    ) -> Self {
        Self {
            pool,
            jwt_config,
            email_service,
            event_bus,
            frontend_url: config.frontend_url.clone(),
            webauthn,
            reg_challenges: ChallengeStore::new(std::time::Duration::from_secs(300)),
            auth_challenges: ChallengeStore::new(std::time::Duration::from_secs(300)),
        }
    }

    // ── Magic Link ─────────────────────────────────────────────────

    /// Request a magic link to be sent to the given email.
    /// Always returns Ok (consistent response regardless of email existence).
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on database or email sending failures.
    pub async fn request_magic_link(&self, email: &str) -> Result<(), AuthError> {
        let account = AccountRepository::get_by_email(&self.pool, email).await?;
        let account_id = if let Some(a) = account {
            a.id
        } else {
            let new_id = generate_id();
            let name = email.split('@').next().unwrap_or("User").to_string();
            AccountRepository::create(&self.pool, new_id, email, &name).await?;
            self.event_bus
                .publish(DomainEvent::AccountCreated { account_id: new_id });
            new_id
        };

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

        let account_id = token_record.account_id;

        let access_token = issue_access_token(&self.jwt_config, account_id)?;
        let refresh_token = issue_refresh_token(&self.pool, &self.jwt_config, account_id).await?;

        Ok((access_token, refresh_token, account_id))
    }

    // ── Passkey Registration (requires authenticated user) ─────────

    /// Begin passkey registration. Returns `CreationChallengeResponse` for the client.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on `WebAuthn` or database failure.
    pub async fn passkey_register_begin(
        &self,
        account_id: Uuid,
    ) -> Result<CreationChallengeResponse, AuthError> {
        let account = AccountRepository::get_by_id(&self.pool, account_id).await?;
        let existing_creds =
            PasskeyCredentialRepository::list_by_account(&self.pool, account_id).await?;

        let exclude_credentials: Vec<CredentialID> = existing_creds
            .iter()
            .map(|c| CredentialID::from(c.credential_id.clone()))
            .collect();

        let (ccr, reg_state) = self
            .webauthn
            .start_passkey_registration(
                account.id,
                &account.email,
                &account.name,
                Some(exclude_credentials),
            )
            .map_err(|e| AuthError::WebAuthn(e.to_string()))?;

        self.reg_challenges.insert(account_id, reg_state);

        Ok(ccr)
    }

    /// Complete passkey registration.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on `WebAuthn` verification or database failure.
    pub async fn passkey_register_complete(
        &self,
        account_id: Uuid,
        reg: &RegisterPublicKeyCredential,
        name: &str,
    ) -> Result<(), AuthError> {
        let reg_state = self
            .reg_challenges
            .remove(&account_id)
            .ok_or_else(|| AuthError::WebAuthn("No pending registration challenge".to_string()))?;

        let passkey = self
            .webauthn
            .finish_passkey_registration(reg, &reg_state)
            .map_err(|e| AuthError::WebAuthn(e.to_string()))?;

        let credential_json =
            serde_json::to_value(&passkey).map_err(|e| AuthError::WebAuthn(e.to_string()))?;

        PasskeyCredentialRepository::create(
            &self.pool,
            generate_id(),
            account_id,
            passkey.cred_id().as_ref(),
            &credential_json,
            name,
        )
        .await?;

        Ok(())
    }

    // ── Passkey Login (public, discoverable) ───────────────────────

    /// Begin passkey authentication (discoverable / conditional UI).
    /// Returns `(RequestChallengeResponse, challenge_id)`.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on `WebAuthn` failure.
    pub fn passkey_login_begin(&self) -> Result<(RequestChallengeResponse, Uuid), AuthError> {
        let (rcr, auth_state) = self
            .webauthn
            .start_discoverable_authentication()
            .map_err(|e| AuthError::WebAuthn(e.to_string()))?;

        let challenge_id = generate_id();
        self.auth_challenges.insert(challenge_id, auth_state);

        Ok((rcr, challenge_id))
    }

    /// Complete passkey authentication. Returns `(access_token, refresh_token, account_id)`.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` on `WebAuthn` verification or credential lookup failure.
    pub async fn passkey_login_complete(
        &self,
        challenge_id: Uuid,
        auth: &PublicKeyCredential,
    ) -> Result<(String, String, Uuid), AuthError> {
        let auth_state = self.auth_challenges.remove(&challenge_id).ok_or_else(|| {
            AuthError::WebAuthn("No pending authentication challenge".to_string())
        })?;

        let cred_id_bytes = auth.id.as_ref();

        let stored_cred =
            PasskeyCredentialRepository::get_by_credential_id(&self.pool, cred_id_bytes)
                .await?
                .ok_or(AuthError::CredentialNotFound)?;

        let mut passkey: Passkey = serde_json::from_value(stored_cred.credential.clone())
            .map_err(|e| AuthError::WebAuthn(format!("Invalid stored credential: {e}")))?;

        let auth_result = self
            .webauthn
            .finish_discoverable_authentication(
                auth,
                auth_state,
                &[DiscoverableKey::from(passkey.clone())],
            )
            .map_err(|e| AuthError::WebAuthn(e.to_string()))?;

        if auth_result.needs_update() {
            passkey.update_credential(&auth_result);
            if let Ok(updated_json) = serde_json::to_value(&passkey) {
                let _ = PasskeyCredentialRepository::update_credential(
                    &self.pool,
                    stored_cred.id,
                    &updated_json,
                )
                .await;
            }
        }

        PasskeyCredentialRepository::update_last_used(&self.pool, stored_cred.id).await?;

        let access_token = issue_access_token(&self.jwt_config, stored_cred.account_id)?;
        let refresh_token =
            issue_refresh_token(&self.pool, &self.jwt_config, stored_cred.account_id).await?;

        Ok((access_token, refresh_token, stored_cred.account_id))
    }
}
