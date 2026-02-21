-- File status enum
CREATE TYPE file_status AS ENUM ('pending', 'confirmed', 'rejected');

-- Files table
CREATE TABLE files (
    id          UUID PRIMARY KEY,
    filename    VARCHAR(255) NOT NULL,
    mime_type   VARCHAR(127) NOT NULL,
    file_size   BIGINT NOT NULL,
    storage_path VARCHAR(1024) NOT NULL,
    scope_type  VARCHAR(50) NOT NULL,
    scope_id    UUID NOT NULL,
    status      file_status NOT NULL DEFAULT 'confirmed',
    uploaded_by UUID NOT NULL REFERENCES accounts(id),
    organization_id UUID REFERENCES organizations(id),
    project_id  UUID REFERENCES projects(id),
    task_id     UUID REFERENCES tasks(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

-- Index: scope lookup (active files only)
CREATE INDEX idx_files_scope ON files (scope_type, scope_id)
    WHERE deleted_at IS NULL;

-- Index: uploaded_by lookup (active files only)
CREATE INDEX idx_files_uploaded_by ON files (uploaded_by)
    WHERE deleted_at IS NULL;

-- Index: cleanup query (soft-deleted files past grace period)
CREATE INDEX idx_files_cleanup ON files (status, created_at)
    WHERE deleted_at IS NOT NULL;

-- Index: deleted_at for orphan detection
CREATE INDEX idx_files_deleted_at ON files (deleted_at)
    WHERE deleted_at IS NOT NULL;

-- File metadata table (key-value pairs)
CREATE TABLE file_metadata (
    id          UUID PRIMARY KEY,
    file_id     UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    key         VARCHAR(255) NOT NULL,
    value       TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(file_id, key)
);

CREATE INDEX idx_file_metadata_file_id ON file_metadata (file_id);
