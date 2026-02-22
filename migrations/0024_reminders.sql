-- Phase 10: Notifications & Reminders - Scheduled reminders table

CREATE TABLE scheduled_reminders (
    id              UUID        PRIMARY KEY,
    todo_id         UUID        NOT NULL REFERENCES todos(id),
    type            VARCHAR(30) NOT NULL,
    trigger_at      TIMESTAMPTZ NOT NULL,
    fired           BOOLEAN     NOT NULL DEFAULT false,
    fired_at        TIMESTAMPTZ,
    notification_id UUID        REFERENCES notifications(id),
    config          JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scheduler query: unfired reminders that are due
CREATE INDEX idx_scheduled_reminders_trigger
    ON scheduled_reminders (trigger_at, fired)
    WHERE fired = false;

-- Query reminders by todo
CREATE INDEX idx_scheduled_reminders_todo_id
    ON scheduled_reminders (todo_id);
