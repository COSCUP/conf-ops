CREATE TABLE member_tags (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id),
    name VARCHAR NOT NULL,
    description TEXT,
    external_task_creation JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);
CREATE INDEX idx_member_tags_project_id ON member_tags (project_id) WHERE deleted_at IS NULL;

CREATE TABLE member_tag_assignments (
    id UUID PRIMARY KEY,
    tag_id UUID NOT NULL REFERENCES member_tags(id),
    member_id UUID REFERENCES members(id),
    contact_id UUID REFERENCES contacts(id),
    project_id UUID NOT NULL REFERENCES projects(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_member_or_contact CHECK (
        (member_id IS NOT NULL AND contact_id IS NULL) OR
        (member_id IS NULL AND contact_id IS NOT NULL)
    )
);
CREATE INDEX idx_tag_assignments_tag_id ON member_tag_assignments (tag_id);
CREATE INDEX idx_tag_assignments_member_id ON member_tag_assignments (member_id) WHERE member_id IS NOT NULL;
CREATE INDEX idx_tag_assignments_contact_id ON member_tag_assignments (contact_id) WHERE contact_id IS NOT NULL;
CREATE INDEX idx_tag_assignments_project_id ON member_tag_assignments (project_id);
CREATE UNIQUE INDEX uq_tag_assignment_member ON member_tag_assignments (tag_id, member_id) WHERE member_id IS NOT NULL;
CREATE UNIQUE INDEX uq_tag_assignment_contact ON member_tag_assignments (tag_id, contact_id) WHERE contact_id IS NOT NULL;
