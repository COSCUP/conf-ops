-- Align accounts table with plan spec
-- Rename display_name -> name
ALTER TABLE accounts RENAME COLUMN display_name TO name;

-- Rename profile -> profile_data
ALTER TABLE accounts RENAME COLUMN profile TO profile_data;

-- Add notification_preferences column
ALTER TABLE accounts ADD COLUMN notification_preferences JSONB NOT NULL DEFAULT '{}';

-- Fix email uniqueness: drop table-level UNIQUE constraint, replace with partial UNIQUE index
ALTER TABLE accounts DROP CONSTRAINT accounts_email_key;
DROP INDEX IF EXISTS idx_accounts_email;
CREATE UNIQUE INDEX uq_accounts_email ON accounts (email) WHERE deleted_at IS NULL;

-- Add locale index for batch notification queries
CREATE INDEX idx_accounts_locale ON accounts (locale);

-- Add rotated_at to refresh_tokens for token rotation grace period detection
ALTER TABLE refresh_tokens ADD COLUMN rotated_at TIMESTAMPTZ;
