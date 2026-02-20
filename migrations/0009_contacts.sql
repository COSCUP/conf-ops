CREATE TABLE contacts (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id),
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    merged_into_id UUID REFERENCES contacts(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_contacts_organization_email ON contacts (organization_id, email) WHERE deleted_at IS NULL AND merged_into_id IS NULL;
CREATE INDEX idx_contacts_organization_id ON contacts (organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_contacts_merged_into_id ON contacts (merged_into_id) WHERE merged_into_id IS NOT NULL;
