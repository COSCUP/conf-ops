-- AI Context: stores each AI call with prompt/response and token usage
CREATE TABLE ai_contexts (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(id),
    message_id UUID NOT NULL REFERENCES messages(id),
    prompt TEXT NOT NULL,
    response TEXT NOT NULL,
    model VARCHAR(100) NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_contexts_task_id ON ai_contexts (task_id);
CREATE INDEX idx_ai_contexts_message_id ON ai_contexts (message_id);

-- AI Pipeline Events: queued pipeline triggers with retry logic
CREATE TABLE ai_pipeline_events (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(id),
    trigger_type VARCHAR(30) NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_pipeline_events_status_scheduled ON ai_pipeline_events (status, scheduled_at);
CREATE INDEX idx_ai_pipeline_events_task_id ON ai_pipeline_events (task_id);

-- Partial GIN index to accelerate pending suggestion lookups in JSONB content
CREATE INDEX idx_messages_pending_suggestions ON messages
    USING GIN ((content->'suggestionGroup'->'suggestions'))
    WHERE source_type = 'ai_suggestion';
