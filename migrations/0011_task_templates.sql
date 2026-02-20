-- Task templates, todo templates, data schemas
CREATE TABLE task_templates (
    id          UUID        PRIMARY KEY,
    project_id  UUID        NOT NULL REFERENCES projects(id),
    name        VARCHAR     NOT NULL,
    description TEXT,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE INDEX idx_task_templates_project_id ON task_templates (project_id)
    WHERE deleted_at IS NULL;
CREATE INDEX idx_task_templates_created_by ON task_templates (created_by);
CREATE INDEX idx_task_templates_deleted_at ON task_templates (deleted_at);

-- Task template <-> member tag association
CREATE TABLE task_template_tags (
    id               UUID        PRIMARY KEY,
    task_template_id UUID        NOT NULL REFERENCES task_templates(id),
    member_tag_id    UUID        NOT NULL REFERENCES member_tags(id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_template_tags_task_template_id ON task_template_tags (task_template_id);
CREATE INDEX idx_task_template_tags_member_tag_id    ON task_template_tags (member_tag_id);
CREATE UNIQUE INDEX uq_task_template_tags_template_tag
    ON task_template_tags (task_template_id, member_tag_id);

-- Todo templates (child of task template)
CREATE TABLE todo_templates (
    id               UUID        PRIMARY KEY,
    task_template_id UUID        NOT NULL REFERENCES task_templates(id),
    parent_id        UUID        REFERENCES todo_templates(id),
    name             VARCHAR     NOT NULL,
    description      TEXT,
    sort_order       INTEGER     NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at       TIMESTAMPTZ
);

CREATE INDEX idx_todo_templates_task_template_id ON todo_templates (task_template_id);
CREATE INDEX idx_todo_templates_parent_id        ON todo_templates (parent_id);
CREATE INDEX idx_todo_templates_deleted_at       ON todo_templates (deleted_at);

-- Data schemas (child of task template)
CREATE TABLE data_schemas (
    id               UUID        PRIMARY KEY,
    task_template_id UUID        NOT NULL REFERENCES task_templates(id),
    name             VARCHAR     NOT NULL,
    fields           JSONB       NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at       TIMESTAMPTZ
);

CREATE INDEX idx_data_schemas_task_template_id ON data_schemas (task_template_id);
CREATE INDEX idx_data_schemas_deleted_at       ON data_schemas (deleted_at);
