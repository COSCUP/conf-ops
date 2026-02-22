-- Audit logs with monthly partitioning
CREATE TABLE audit_logs (
    id            UUID        NOT NULL,
    actor_type    VARCHAR(20) NOT NULL,
    actor_id      UUID,
    action        VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id   UUID        NOT NULL,
    context_type  VARCHAR(20),
    context_id    UUID,
    details       JSONB       NOT NULL DEFAULT '{}',
    ip_address    INET,
    user_agent    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create partitions for current month and future 3 months
-- Note: partition boundaries use first day of each month
CREATE TABLE audit_logs_2026_02 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE audit_logs_2026_03 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE audit_logs_2026_04 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE audit_logs_2026_05 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');

-- Default partition for any rows outside defined ranges
CREATE TABLE audit_logs_default PARTITION OF audit_logs DEFAULT;

CREATE INDEX idx_audit_logs_created_at
    ON audit_logs (created_at);

CREATE INDEX idx_audit_logs_actor
    ON audit_logs (actor_type, actor_id);

CREATE INDEX idx_audit_logs_resource
    ON audit_logs (resource_type, resource_id);

CREATE INDEX idx_audit_logs_context
    ON audit_logs (context_type, context_id);

CREATE INDEX idx_audit_logs_action
    ON audit_logs (action);
