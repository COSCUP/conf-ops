CREATE TABLE crdt_operations (
    id UUID PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    operation BYTEA NOT NULL,
    created_by UUID NOT NULL REFERENCES accounts(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_crdt_operations_entity ON crdt_operations(entity_type, entity_id);
CREATE INDEX idx_crdt_operations_created_at ON crdt_operations(entity_id, created_at);
