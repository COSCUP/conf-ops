-- Task instantiation from task templates

CREATE TYPE task_status AS ENUM ('pending', 'in_progress', 'completed', 'cancelled');

CREATE TABLE tasks (
    id               UUID        PRIMARY KEY,
    project_id       UUID        NOT NULL REFERENCES projects(id),
    task_template_id UUID        NOT NULL REFERENCES task_templates(id),
    owner_tag_id     UUID        NOT NULL REFERENCES member_tags(id),
    name             VARCHAR     NOT NULL,
    description      TEXT,
    status           task_status NOT NULL DEFAULT 'pending',
    created_by       UUID        NOT NULL REFERENCES accounts(id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at       TIMESTAMPTZ
);

-- Query tasks by project and status
CREATE INDEX idx_tasks_project_id_status ON tasks (project_id, status);

-- Query tasks by owner tag
CREATE INDEX idx_tasks_owner_tag_id ON tasks (owner_tag_id);

-- Query tasks by source template
CREATE INDEX idx_tasks_task_template_id ON tasks (task_template_id);

-- Query tasks by creator
CREATE INDEX idx_tasks_created_by ON tasks (created_by);

-- Soft-delete filter
CREATE INDEX idx_tasks_deleted_at ON tasks (deleted_at);
