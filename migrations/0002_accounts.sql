-- Accounts table: core user identity
CREATE TABLE accounts (
    id          UUID PRIMARY KEY,
    email       TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL DEFAULT '',
    avatar_url  TEXT,
    locale      TEXT NOT NULL DEFAULT 'zh-TW',
    profile     JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE INDEX idx_accounts_email ON accounts (email) WHERE deleted_at IS NULL;
CREATE INDEX idx_accounts_deleted_at ON accounts (deleted_at) WHERE deleted_at IS NOT NULL;

-- Passkey credentials: WebAuthn public key credentials
CREATE TABLE passkey_credentials (
    id              UUID PRIMARY KEY,
    account_id      UUID NOT NULL REFERENCES accounts(id),
    credential_id   BYTEA NOT NULL UNIQUE,
    credential      JSONB NOT NULL,
    name            TEXT NOT NULL DEFAULT 'My Passkey',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_passkey_credentials_account_id ON passkey_credentials (account_id);

-- Magic link tokens: email-based passwordless authentication
CREATE TABLE magic_link_tokens (
    id              UUID PRIMARY KEY,
    account_id      UUID REFERENCES accounts(id),
    email           TEXT NOT NULL,
    token_hash      BYTEA NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    used_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_magic_link_tokens_token_hash ON magic_link_tokens (token_hash) WHERE used_at IS NULL;
CREATE INDEX idx_magic_link_tokens_email ON magic_link_tokens (email);

-- Refresh tokens: long-lived session tokens
CREATE TABLE refresh_tokens (
    id              UUID PRIMARY KEY,
    account_id      UUID NOT NULL REFERENCES accounts(id),
    token_hash      BYTEA NOT NULL UNIQUE,
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    replaced_by     UUID REFERENCES refresh_tokens(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_account_id ON refresh_tokens (account_id) WHERE revoked_at IS NULL;
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens (token_hash) WHERE revoked_at IS NULL;
