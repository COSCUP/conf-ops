CREATE TYPE member_role AS ENUM ('owner', 'tag_admin', 'member');

CREATE TABLE members (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    role member_role NOT NULL DEFAULT 'member',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX uq_members_project_account ON members (project_id, account_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_members_project_id ON members (project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_members_account_id ON members (account_id) WHERE deleted_at IS NULL;
