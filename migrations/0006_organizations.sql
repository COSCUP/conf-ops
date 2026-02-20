CREATE TYPE org_role AS ENUM ('org_owner', 'org_admin', 'org_member');

CREATE TABLE organizations (
    id          UUID        PRIMARY KEY,
    name        VARCHAR     NOT NULL,
    description TEXT,
    logo_url    VARCHAR,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE TABLE organization_members (
    id              UUID        PRIMARY KEY,
    organization_id UUID        NOT NULL REFERENCES organizations(id),
    account_id      UUID        NOT NULL REFERENCES accounts(id),
    role            org_role    NOT NULL DEFAULT 'org_member',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organizations_name ON organizations (name) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_created_by ON organizations (created_by) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uq_organization_members_org_account ON organization_members (organization_id, account_id);
CREATE INDEX idx_organization_members_account_id ON organization_members (account_id);
