CREATE TYPE project_status AS ENUM ('preparing', 'active', 'completed', 'archived');

CREATE TABLE projects (
    id                  UUID            PRIMARY KEY,
    organization_id     UUID            NOT NULL REFERENCES organizations(id),
    name                VARCHAR         NOT NULL,
    description         TEXT,
    status              project_status  NOT NULL DEFAULT 'preparing',
    source_project_id   UUID            REFERENCES projects(id),
    permission_settings JSONB           NOT NULL DEFAULT '{}',
    created_by          UUID            NOT NULL REFERENCES accounts(id),
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

CREATE INDEX idx_projects_organization_id ON projects (organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_projects_status ON projects (status) WHERE deleted_at IS NULL;
CREATE INDEX idx_projects_created_by ON projects (created_by) WHERE deleted_at IS NULL;
CREATE INDEX idx_projects_source_project_id ON projects (source_project_id) WHERE source_project_id IS NOT NULL AND deleted_at IS NULL;
