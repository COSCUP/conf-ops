-- Phase 5: Conversation system tables

-- Message source type enum
CREATE TYPE message_source_type AS ENUM (
    'member',
    'ai_suggestion',
    'tool_execution',
    'system',
    'email_inbound'
);

-- Messages table (append-only, no updated_at / deleted_at)
CREATE TABLE messages (
    id                   UUID                 PRIMARY KEY,
    task_id              UUID                 NOT NULL REFERENCES tasks(id),
    source_type          message_source_type  NOT NULL,
    source_id            UUID,
    content              JSONB                NOT NULL,
    attachments          JSONB,
    action_result        JSONB,
    last_seen_message_id UUID,
    created_at           TIMESTAMPTZ          NOT NULL DEFAULT NOW()
);

-- Indexes for messages
CREATE INDEX idx_messages_task_id_created_at ON messages (task_id, created_at);
CREATE INDEX idx_messages_source_type ON messages (source_type);
CREATE INDEX idx_messages_content ON messages USING GIN (content jsonb_path_ops);

-- Conversation states (per-user read tracking)
CREATE TABLE conversation_states (
    id                   UUID        PRIMARY KEY,
    task_id              UUID        NOT NULL REFERENCES tasks(id),
    account_id           UUID        NOT NULL REFERENCES accounts(id),
    last_read_message_id UUID,
    unread_count         INTEGER     NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_conversation_states_task_account
        UNIQUE (task_id, account_id)
);

CREATE INDEX idx_conversation_states_task_id ON conversation_states (task_id);
CREATE INDEX idx_conversation_states_account_id ON conversation_states (account_id);

-- Last seen positions (CRDT clock tracking)
CREATE TABLE last_seen_positions (
    id              UUID        PRIMARY KEY,
    task_id         UUID        NOT NULL REFERENCES tasks(id),
    account_id      UUID        NOT NULL REFERENCES accounts(id),
    y_clock         BIGINT      NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_last_seen_positions_task_account
        UNIQUE (task_id, account_id)
);

CREATE INDEX idx_last_seen_positions_task_id ON last_seen_positions (task_id);
CREATE INDEX idx_last_seen_positions_account_id ON last_seen_positions (account_id);
