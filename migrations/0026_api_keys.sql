-- API keys for external data access
CREATE TABLE api_keys (
    id           UUID        PRIMARY KEY,
    project_id   UUID        NOT NULL REFERENCES projects(id),
    name         VARCHAR     NOT NULL,
    key_hash     VARCHAR     NOT NULL,
    permissions  JSONB       NOT NULL DEFAULT '{}',
    created_by   UUID        NOT NULL REFERENCES accounts(id),
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_project_id
    ON api_keys (project_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_api_keys_key_hash
    ON api_keys (key_hash)
    WHERE deleted_at IS NULL;
