-- Email threads (one task can have multiple email threads)
CREATE TABLE email_threads (
    id              UUID PRIMARY KEY,
    task_id         UUID NOT NULL REFERENCES tasks(id),
    subject         VARCHAR(998) NOT NULL,
    participants    JSONB NOT NULL DEFAULT '[]',
    message_ids     JSONB NOT NULL DEFAULT '[]',
    last_message_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_threads_task_id ON email_threads (task_id);
CREATE INDEX idx_email_threads_message_ids ON email_threads USING GIN (message_ids jsonb_path_ops);

-- Email messages (append-only, immutable records)
CREATE TABLE email_messages (
    id                      UUID PRIMARY KEY,
    thread_id               UUID NOT NULL REFERENCES email_threads(id),
    message_id              VARCHAR(998) UNIQUE NOT NULL,
    in_reply_to             VARCHAR(998),
    references_header       TEXT,
    from_address            VARCHAR(320) NOT NULL,
    to_addresses            JSONB NOT NULL DEFAULT '[]',
    cc_addresses            JSONB NOT NULL DEFAULT '[]',
    subject                 VARCHAR(998) NOT NULL,
    direction               VARCHAR(20) NOT NULL,
    conversation_message_id UUID REFERENCES messages(id),
    raw_headers             JSONB,
    send_status             VARCHAR(20) NOT NULL DEFAULT 'sent',
    retry_count             INTEGER NOT NULL DEFAULT 0,
    next_retry_at           TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_messages_thread_id ON email_messages (thread_id);
CREATE INDEX idx_email_messages_from_address ON email_messages (from_address);
CREATE INDEX idx_email_messages_in_reply_to ON email_messages (in_reply_to);
CREATE INDEX idx_email_messages_retry ON email_messages (next_retry_at)
    WHERE send_status = 'failed' AND retry_count < 3;

-- Unassigned emails (emails that couldn't be matched to a thread)
CREATE TABLE unassigned_emails (
    id              UUID PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id),
    from_name       VARCHAR(255),
    from_address    VARCHAR(320) NOT NULL,
    subject         VARCHAR(998) NOT NULL,
    snippet         TEXT,
    raw_mime        BYTEA NOT NULL,
    has_attachments BOOLEAN NOT NULL DEFAULT FALSE,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_at     TIMESTAMPTZ,
    assigned_task_id UUID REFERENCES tasks(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_unassigned_emails_project_unassigned
    ON unassigned_emails (project_id, created_at DESC)
    WHERE assigned_at IS NULL;
