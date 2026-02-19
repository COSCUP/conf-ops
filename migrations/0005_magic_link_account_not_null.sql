-- magic_link_tokens.account_id: nullable → NOT NULL
-- Per spec: account is created before magic link token is issued (at request time)

-- Clean up any orphan tokens without account association
DELETE FROM magic_link_tokens WHERE account_id IS NULL;

-- Enforce NOT NULL constraint
ALTER TABLE magic_link_tokens ALTER COLUMN account_id SET NOT NULL;

-- Add missing index (required by plan spec)
CREATE INDEX idx_magic_link_tokens_account_id ON magic_link_tokens (account_id);
