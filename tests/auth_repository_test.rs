mod common;

use chrono::{Duration, Utc};
use common::TestContext;
use conf_ops::id::generate_id;
use conf_ops::modules::auth::error::AuthError;
use conf_ops::modules::auth::repository::{
    AccountRepository, MagicLinkTokenRepository, PasskeyCredentialRepository,
    RefreshTokenRepository,
};

#[tokio::test]
async fn create_and_get_account() {
    let ctx = TestContext::new().await;
    let id = generate_id();

    let account = AccountRepository::create(&ctx.pool, id, "test@example.com", "Test User")
        .await
        .expect("should create account");

    assert_eq!(account.id, id);
    assert_eq!(account.email, "test@example.com");
    assert_eq!(account.name, "Test User");
    assert!(account.deleted_at.is_none());

    let fetched = AccountRepository::get_by_id(&ctx.pool, id)
        .await
        .expect("should fetch account");

    assert_eq!(fetched.email, "test@example.com");
}

#[tokio::test]
async fn get_account_by_email() {
    let ctx = TestContext::new().await;
    let id = generate_id();

    AccountRepository::create(&ctx.pool, id, "find@example.com", "Find Me")
        .await
        .expect("should create account");

    let found = AccountRepository::get_by_email(&ctx.pool, "find@example.com")
        .await
        .expect("should query")
        .expect("should find account");

    assert_eq!(found.id, id);

    let not_found = AccountRepository::get_by_email(&ctx.pool, "nobody@example.com")
        .await
        .expect("should query");

    assert!(not_found.is_none());
}

#[tokio::test]
async fn email_uniqueness_constraint() {
    let ctx = TestContext::new().await;

    AccountRepository::create(&ctx.pool, generate_id(), "dup@example.com", "First")
        .await
        .expect("should create first account");

    let result =
        AccountRepository::create(&ctx.pool, generate_id(), "dup@example.com", "Second").await;

    assert!(matches!(result, Err(AuthError::EmailAlreadyExists)));
}

#[tokio::test]
async fn update_account() {
    let ctx = TestContext::new().await;
    let id = generate_id();

    AccountRepository::create(&ctx.pool, id, "update@example.com", "Original")
        .await
        .expect("should create account");

    let updated = AccountRepository::update(
        &ctx.pool,
        id,
        Some("Updated Name"),
        Some(Some("https://example.com/avatar.png")),
        None,
        Some("en-US"),
    )
    .await
    .expect("should update account");

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(
        updated.avatar_url.as_deref(),
        Some("https://example.com/avatar.png")
    );
    assert_eq!(updated.locale, "en-US");
}

#[tokio::test]
async fn soft_delete_account() {
    let ctx = TestContext::new().await;
    let id = generate_id();

    AccountRepository::create(&ctx.pool, id, "delete@example.com", "To Delete")
        .await
        .expect("should create account");

    AccountRepository::soft_delete(&ctx.pool, id)
        .await
        .expect("should soft delete");

    let result = AccountRepository::get_by_id(&ctx.pool, id).await;
    assert!(matches!(result, Err(AuthError::AccountNotFound)));

    let by_email = AccountRepository::get_by_email(&ctx.pool, "delete@example.com")
        .await
        .expect("should query");
    assert!(by_email.is_none());
}

#[tokio::test]
async fn profile_jsonb_read_write() {
    let ctx = TestContext::new().await;
    let id = generate_id();

    AccountRepository::create(&ctx.pool, id, "profile@example.com", "Profile Test")
        .await
        .expect("should create account");

    let profile = serde_json::json!({
        "bio": "Hello world",
        "social": {
            "twitter": "@test"
        }
    });

    let updated = AccountRepository::update_profile(&ctx.pool, id, &profile, None)
        .await
        .expect("should update profile");

    assert_eq!(updated.profile_data["bio"], "Hello world");
    assert_eq!(updated.profile_data["social"]["twitter"], "@test");
}

#[tokio::test]
async fn passkey_credential_crud() {
    let ctx = TestContext::new().await;
    let account_id = generate_id();
    AccountRepository::create(&ctx.pool, account_id, "passkey@example.com", "Passkey User")
        .await
        .expect("should create account");

    let cred_id = generate_id();
    let credential_id_bytes = b"test-credential-id";
    let credential_json = serde_json::json!({"type": "public-key"});

    PasskeyCredentialRepository::create(
        &ctx.pool,
        cred_id,
        account_id,
        credential_id_bytes,
        &credential_json,
        "Test Passkey",
    )
    .await
    .expect("should create credential");

    let creds = PasskeyCredentialRepository::list_by_account(&ctx.pool, account_id)
        .await
        .expect("should list credentials");
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].name, "Test Passkey");

    let found = PasskeyCredentialRepository::get_by_credential_id(&ctx.pool, credential_id_bytes)
        .await
        .expect("should query")
        .expect("should find credential");
    assert_eq!(found.id, cred_id);

    PasskeyCredentialRepository::delete(&ctx.pool, cred_id, account_id)
        .await
        .expect("should delete credential");

    let after_delete = PasskeyCredentialRepository::list_by_account(&ctx.pool, account_id)
        .await
        .expect("should list");
    assert!(after_delete.is_empty());
}

#[tokio::test]
async fn magic_link_token_lifecycle() {
    let ctx = TestContext::new().await;
    let account_id = generate_id();
    AccountRepository::create(&ctx.pool, account_id, "magic@example.com", "Magic User")
        .await
        .expect("should create account");

    let token_id = generate_id();
    let token_hash = b"hashed-token-value";
    let expires_at = Utc::now() + Duration::minutes(15);

    MagicLinkTokenRepository::create(
        &ctx.pool,
        token_id,
        account_id,
        "magic@example.com",
        token_hash,
        expires_at,
    )
    .await
    .expect("should create magic link token");

    let found = MagicLinkTokenRepository::get_by_token_hash(&ctx.pool, token_hash)
        .await
        .expect("should query")
        .expect("should find token");
    assert_eq!(found.email, "magic@example.com");
    assert!(found.used_at.is_none());

    MagicLinkTokenRepository::mark_used(&ctx.pool, token_id)
        .await
        .expect("should mark used");

    let used = MagicLinkTokenRepository::get_by_token_hash(&ctx.pool, token_hash)
        .await
        .expect("should query")
        .expect("should find token");
    assert!(used.used_at.is_some());
}

#[tokio::test]
async fn refresh_token_lifecycle() {
    let ctx = TestContext::new().await;
    let account_id = generate_id();
    AccountRepository::create(&ctx.pool, account_id, "refresh@example.com", "Refresh User")
        .await
        .expect("should create account");

    let token1_id = generate_id();
    let token1_hash = b"hashed-refresh-token-1";
    let expires_at = Utc::now() + Duration::days(7);

    RefreshTokenRepository::create(&ctx.pool, token1_id, account_id, token1_hash, expires_at)
        .await
        .expect("should create refresh token");

    // Create a second token that will replace the first
    let token2_id = generate_id();
    let token2_hash = b"hashed-refresh-token-2";
    RefreshTokenRepository::create(&ctx.pool, token2_id, account_id, token2_hash, expires_at)
        .await
        .expect("should create second refresh token");

    // Revoke the first token, recording the replacement
    RefreshTokenRepository::revoke(&ctx.pool, token1_id, Some(token2_id))
        .await
        .expect("should revoke with replacement");

    // Revoke without replacement
    RefreshTokenRepository::revoke(&ctx.pool, token2_id, None)
        .await
        .expect("should revoke without replacement");

    // Create another token and test revoke_all
    let token3_id = generate_id();
    let token3_hash = b"hashed-refresh-token-3";
    RefreshTokenRepository::create(&ctx.pool, token3_id, account_id, token3_hash, expires_at)
        .await
        .expect("should create third refresh token");

    RefreshTokenRepository::revoke_all_for_account(&ctx.pool, account_id)
        .await
        .expect("should revoke all");
}
