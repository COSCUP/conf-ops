-- MCP Tool Execution Engine: tool_configs and tool_executions tables

-- Tool execution status enum
CREATE TYPE tool_execution_status AS ENUM ('success', 'error');

-- Tool configuration table
CREATE TABLE tool_configs (
    id                UUID        PRIMARY KEY,
    scope_type        VARCHAR(20) NOT NULL,
    scope_id          UUID        NOT NULL,
    tool_type         VARCHAR(20) NOT NULL,
    tool_name         VARCHAR(255) NOT NULL,
    display_name      VARCHAR(255),
    description       TEXT,
    enabled           BOOLEAN     NOT NULL DEFAULT false,
    config            JSONB       NOT NULL DEFAULT '{}',
    mcp_server_config JSONB,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ,

    CONSTRAINT uq_tool_configs_scope_tool_name
        UNIQUE (scope_type, scope_id, tool_name)
);

-- Tool execution records
CREATE TABLE tool_executions (
    id            UUID                  PRIMARY KEY,
    task_id       UUID                  NOT NULL REFERENCES tasks(id),
    suggestion_id UUID,
    tool_name     VARCHAR(255)          NOT NULL,
    parameters    JSONB                 NOT NULL DEFAULT '{}',
    result        JSONB                 NOT NULL DEFAULT '{}',
    status        tool_execution_status NOT NULL,
    executed_by   UUID                  NOT NULL REFERENCES accounts(id),
    executed_at   TIMESTAMPTZ           NOT NULL DEFAULT NOW(),
    duration_ms   INT                   NOT NULL DEFAULT 0
);

-- Indexes for tool_configs
CREATE INDEX idx_tool_configs_scope ON tool_configs (scope_type, scope_id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_tool_configs_tool_name ON tool_configs (tool_name)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_tool_configs_config ON tool_configs USING GIN (config jsonb_path_ops);

-- Indexes for tool_executions
CREATE INDEX idx_tool_executions_task_id ON tool_executions (task_id);
CREATE INDEX idx_tool_executions_executed_by ON tool_executions (executed_by);
CREATE INDEX idx_tool_executions_executed_at ON tool_executions (executed_at DESC);
