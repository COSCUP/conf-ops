-- Todo management for tasks

CREATE TYPE todo_status AS ENUM ('open', 'completed');
CREATE TYPE todo_type AS ENUM ('template', 'ad_hoc');

CREATE TABLE todos (
    id                 UUID        PRIMARY KEY,
    task_id            UUID        NOT NULL REFERENCES tasks(id),
    parent_id          UUID        REFERENCES todos(id),
    title              VARCHAR     NOT NULL,
    description        TEXT,
    status             todo_status NOT NULL DEFAULT 'open',
    type               todo_type   NOT NULL,
    source_template_id UUID        REFERENCES todo_templates(id),
    due_date           TIMESTAMPTZ,
    sort_order         INTEGER     NOT NULL,
    linked_task_id     UUID        REFERENCES tasks(id),
    completed_at       TIMESTAMPTZ,
    completed_by       UUID        REFERENCES accounts(id),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at         TIMESTAMPTZ
);

-- Query todos by task (with sort order)
CREATE INDEX idx_todos_task_id_sort_order ON todos (task_id, sort_order);

-- Query by linked task
CREATE INDEX idx_todos_linked_task_id ON todos (linked_task_id);

-- Filter by status
CREATE INDEX idx_todos_status ON todos (status);

-- Query sub-todos by parent
CREATE INDEX idx_todos_parent_id ON todos (parent_id);

-- Soft-delete filter
CREATE INDEX idx_todos_deleted_at ON todos (deleted_at);

-- Todo assignees
CREATE TABLE todo_assignees (
    id         UUID        PRIMARY KEY,
    todo_id    UUID        NOT NULL REFERENCES todos(id),
    member_id  UUID        NOT NULL REFERENCES members(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_todo_assignees_todo_id   ON todo_assignees (todo_id);
CREATE INDEX idx_todo_assignees_member_id ON todo_assignees (member_id);
CREATE UNIQUE INDEX uq_todo_assignees_todo_member
    ON todo_assignees (todo_id, member_id);
