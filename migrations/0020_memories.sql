-- Memory and Library Document tables for AI memory inheritance chain

-- library_documents must be created before memories (FK dependency)
CREATE TABLE library_documents (
    id          UUID        PRIMARY KEY,
    scope_type  VARCHAR(20) NOT NULL,
    scope_id    UUID        NOT NULL,
    title       VARCHAR(500) NOT NULL,
    content     TEXT        NOT NULL,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE TABLE memories (
    id          UUID        PRIMARY KEY,
    scope_type  VARCHAR(20) NOT NULL,
    scope_id    UUID        NOT NULL,
    content     TEXT        NOT NULL,
    library_ref UUID        REFERENCES library_documents(id),
    source      VARCHAR(20) NOT NULL,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

CREATE TABLE memory_versions (
    id          UUID        PRIMARY KEY,
    memory_id   UUID        NOT NULL REFERENCES memories(id),
    content     TEXT        NOT NULL,
    changed_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE library_document_versions (
    id          UUID        PRIMARY KEY,
    document_id UUID        NOT NULL REFERENCES library_documents(id),
    title       VARCHAR(500),
    content     TEXT        NOT NULL,
    changed_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes

-- memories: scope query (core index for inheritance chain)
CREATE INDEX idx_memories_scope ON memories (scope_type, scope_id)
    WHERE deleted_at IS NULL;

-- memories: by creator
CREATE INDEX idx_memories_created_by ON memories (created_by)
    WHERE deleted_at IS NULL;

-- memories: by library_ref (find all memories referencing a document)
CREATE INDEX idx_memories_library_ref ON memories (library_ref)
    WHERE library_ref IS NOT NULL AND deleted_at IS NULL;

-- library_documents: scope query
CREATE INDEX idx_library_documents_scope ON library_documents (scope_type, scope_id)
    WHERE deleted_at IS NULL;

-- memory_versions: version history by memory_id
CREATE INDEX idx_memory_versions_memory_id ON memory_versions (memory_id, created_at DESC);

-- library_document_versions: version history by document_id
CREATE INDEX idx_library_document_versions_document_id ON library_document_versions (document_id, created_at DESC);
