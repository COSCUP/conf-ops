-- Webhook configuration and event logs
CREATE TABLE webhooks (
    id          UUID        PRIMARY KEY,
    project_id  UUID        NOT NULL REFERENCES projects(id),
    name        VARCHAR     NOT NULL,
    url         VARCHAR     NOT NULL,
    secret      VARCHAR,
    events      JSONB       NOT NULL,
    enabled     BOOLEAN     NOT NULL DEFAULT true,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE INDEX idx_webhooks_project_enabled
    ON webhooks (project_id, enabled)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_webhooks_url
    ON webhooks (url)
    WHERE deleted_at IS NULL;

CREATE TABLE webhook_event_logs (
    id              UUID        PRIMARY KEY,
    webhook_id      UUID        NOT NULL REFERENCES webhooks(id),
    event_type      VARCHAR     NOT NULL,
    payload         JSONB       NOT NULL,
    status          VARCHAR(20) NOT NULL,
    response_status INTEGER,
    response_body   TEXT,
    attempts        INTEGER     NOT NULL DEFAULT 0,
    max_attempts    INTEGER     NOT NULL DEFAULT 3,
    next_retry_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_webhook_event_logs_webhook_created
    ON webhook_event_logs (webhook_id, created_at DESC);

CREATE INDEX idx_webhook_event_logs_retry_queue
    ON webhook_event_logs (status, next_retry_at)
    WHERE status = 'pending' AND next_retry_at IS NOT NULL;
