-- Phase 10: Notifications & Reminders - Notification tables
-- notifications table: stores all notification records
-- notification_preferences table: per-account channel preferences
-- web_push_subscriptions table: Web Push subscription endpoints

CREATE TABLE notifications (
    id                 UUID        PRIMARY KEY,
    account_id         UUID        NOT NULL REFERENCES accounts(id),
    type               VARCHAR(50) NOT NULL,
    title              VARCHAR     NOT NULL,
    body               TEXT,
    reference_type     VARCHAR(20),
    reference_id       UUID,
    project_id         UUID        REFERENCES projects(id),
    is_read            BOOLEAN     NOT NULL DEFAULT false,
    read_at            TIMESTAMPTZ,
    delivered_channels JSONB       NOT NULL DEFAULT '[]',
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User's unread notification list (primary query path)
CREATE INDEX idx_notifications_account_read_created
    ON notifications (account_id, is_read, created_at DESC);

-- Query notifications by project
CREATE INDEX idx_notifications_project_id
    ON notifications (project_id);

-- Query notifications by reference resource
CREATE INDEX idx_notifications_reference
    ON notifications (reference_type, reference_id);

-- notification_preferences table: per-account notification channel preferences
CREATE TABLE notification_preferences (
    id          UUID        PRIMARY KEY,
    account_id  UUID        NOT NULL UNIQUE REFERENCES accounts(id),
    preferences JSONB       NOT NULL DEFAULT '{}',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- web_push_subscriptions table: browser push subscription endpoints
CREATE TABLE web_push_subscriptions (
    id          UUID        PRIMARY KEY,
    account_id  UUID        NOT NULL REFERENCES accounts(id),
    endpoint    TEXT        NOT NULL,
    p256dh_key  TEXT        NOT NULL,
    auth_key    TEXT        NOT NULL,
    device_name VARCHAR,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_web_push_subscriptions_account
    ON web_push_subscriptions (account_id);

CREATE UNIQUE INDEX idx_web_push_subscriptions_endpoint
    ON web_push_subscriptions (endpoint);
