-- Data entries: structured data collected per task per schema
CREATE TABLE data_entries (
    id              UUID        PRIMARY KEY,
    task_id         UUID        NOT NULL REFERENCES tasks(id),
    data_schema_id  UUID        NOT NULL REFERENCES data_schemas(id),
    values          JSONB       NOT NULL DEFAULT '{}',
    source_links    JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);

-- Index: query entries by task
CREATE INDEX idx_data_entries_task_id ON data_entries (task_id);

-- Index: query entries by schema (for DataSheet aggregation)
CREATE INDEX idx_data_entries_data_schema_id ON data_entries (data_schema_id);

-- Unique: one entry per task per schema
CREATE UNIQUE INDEX uq_data_entries_task_schema
    ON data_entries (task_id, data_schema_id);

-- Index: soft-delete filter
CREATE INDEX idx_data_entries_deleted_at ON data_entries (deleted_at);
