use std::collections::HashMap;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::id::generate_id;

use super::error::ProjectError;

/// Mapping of old IDs to new IDs for deep copy operations.
#[derive(Debug, Default)]
pub struct IdMapping {
    pub member_tags: HashMap<Uuid, Uuid>,
    pub task_templates: HashMap<Uuid, Uuid>,
    pub todo_templates: HashMap<Uuid, Uuid>,
    pub data_schemas: HashMap<Uuid, Uuid>,
}

/// Deep copy all project-related entities from source to target.
///
/// Copies: `member_tags`, `task_templates` (with `todo_templates` + `data_schemas`),
/// `task_template_tags`, memories, `tool_configs`, `permission_settings`.
///
/// Does NOT copy: members, tasks, contacts, `data_entries`, messages,
/// todos, conversations.
///
/// # Errors
///
/// Returns `ProjectError::Database` on database failure.
pub async fn deep_copy_project(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
    created_by: Uuid,
) -> Result<IdMapping, ProjectError> {
    let mut mapping = IdMapping::default();

    // 1. Copy member_tags (with external_task_creation settings)
    copy_member_tags(pool, source_project_id, target_project_id, &mut mapping).await?;

    // 2. Copy task_templates
    copy_task_templates(
        pool,
        source_project_id,
        target_project_id,
        created_by,
        &mut mapping,
    )
    .await?;

    // 3. Copy task_template_tags (using ID mapping)
    copy_task_template_tags(pool, source_project_id, &mapping).await?;

    // 4. Copy memories (project / member_tag / task_template scopes)
    copy_memories(
        pool,
        source_project_id,
        target_project_id,
        created_by,
        &mapping,
    )
    .await?;

    // 5. Copy tool_configs (project scope)
    copy_tool_configs(pool, source_project_id, target_project_id).await?;

    // 6. Copy permission_settings
    copy_permission_settings(pool, source_project_id, target_project_id).await?;

    Ok(mapping)
}

async fn copy_member_tags(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
    mapping: &mut IdMapping,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT id, name, description, external_task_creation
         FROM member_tags
         WHERE project_id = $1 AND deleted_at IS NULL",
    )
    .bind(source_project_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let old_id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let external_task_creation: serde_json::Value = row.try_get("external_task_creation")?;

        let new_id = generate_id();
        mapping.member_tags.insert(old_id, new_id);

        sqlx::query(
            "INSERT INTO member_tags (id, project_id, name, description, external_task_creation)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_id)
        .bind(target_project_id)
        .bind(&name)
        .bind(description.as_deref())
        .bind(&external_task_creation)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn copy_task_templates(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
    created_by: Uuid,
    mapping: &mut IdMapping,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT id, name, description
         FROM task_templates
         WHERE project_id = $1 AND deleted_at IS NULL",
    )
    .bind(source_project_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let old_id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;

        let new_template_id = generate_id();
        mapping.task_templates.insert(old_id, new_template_id);

        sqlx::query(
            "INSERT INTO task_templates (id, project_id, name, description, created_by)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_template_id)
        .bind(target_project_id)
        .bind(&name)
        .bind(description.as_deref())
        .bind(created_by)
        .execute(pool)
        .await?;

        // Copy todo_templates for this task template
        copy_todo_templates(pool, old_id, new_template_id, mapping).await?;

        // Copy data_schemas for this task template
        copy_data_schemas(pool, old_id, new_template_id, mapping).await?;
    }

    Ok(())
}

async fn copy_todo_templates(
    pool: &PgPool,
    source_template_id: Uuid,
    target_template_id: Uuid,
    mapping: &mut IdMapping,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT id, parent_id, name, description, sort_order
         FROM todo_templates
         WHERE task_template_id = $1 AND deleted_at IS NULL
         ORDER BY sort_order ASC",
    )
    .bind(source_template_id)
    .fetch_all(pool)
    .await?;

    // First pass: create ID mappings
    let mut todo_data: Vec<TodoTemplateData> = Vec::new();
    for row in &rows {
        let old_id: Uuid = row.try_get("id")?;
        let parent_id: Option<Uuid> = row.try_get("parent_id")?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let sort_order: i32 = row.try_get("sort_order")?;

        let new_id = generate_id();
        mapping.todo_templates.insert(old_id, new_id);
        todo_data.push(TodoTemplateData {
            old_id,
            parent_id,
            name,
            description,
            sort_order,
        });
    }

    // Second pass: insert with correct parent references
    for td in &todo_data {
        let new_id = mapping.todo_templates[&td.old_id];
        let new_parent_id = td
            .parent_id
            .and_then(|pid| mapping.todo_templates.get(&pid).copied());

        sqlx::query(
            "INSERT INTO todo_templates (id, task_template_id, parent_id, name, description, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(new_id)
        .bind(target_template_id)
        .bind(new_parent_id)
        .bind(&td.name)
        .bind(td.description.as_deref())
        .bind(td.sort_order)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn copy_data_schemas(
    pool: &PgPool,
    source_template_id: Uuid,
    target_template_id: Uuid,
    mapping: &mut IdMapping,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT id, name, fields
         FROM data_schemas
         WHERE task_template_id = $1 AND deleted_at IS NULL",
    )
    .bind(source_template_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let old_id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let fields: serde_json::Value = row.try_get("fields")?;

        let new_id = generate_id();
        mapping.data_schemas.insert(old_id, new_id);

        sqlx::query(
            "INSERT INTO data_schemas (id, task_template_id, name, fields)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(new_id)
        .bind(target_template_id)
        .bind(&name)
        .bind(&fields)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn copy_task_template_tags(
    pool: &PgPool,
    source_project_id: Uuid,
    mapping: &IdMapping,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT ttt.task_template_id, ttt.member_tag_id
         FROM task_template_tags ttt
         JOIN task_templates tt ON tt.id = ttt.task_template_id
         WHERE tt.project_id = $1 AND tt.deleted_at IS NULL",
    )
    .bind(source_project_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let template_id: Uuid = row.try_get("task_template_id")?;
        let tag_id: Uuid = row.try_get("member_tag_id")?;

        // Only copy if both the template and tag exist in the mapping
        if let (Some(&new_template_id), Some(&new_tag_id)) = (
            mapping.task_templates.get(&template_id),
            mapping.member_tags.get(&tag_id),
        ) {
            let new_id = generate_id();
            sqlx::query(
                "INSERT INTO task_template_tags (id, task_template_id, member_tag_id)
                 VALUES ($1, $2, $3)",
            )
            .bind(new_id)
            .bind(new_template_id)
            .bind(new_tag_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn copy_memories(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
    created_by: Uuid,
    mapping: &IdMapping,
) -> Result<(), ProjectError> {
    // Copy project-scope memories
    copy_memories_for_scope(
        pool,
        "project",
        source_project_id,
        target_project_id,
        created_by,
    )
    .await?;

    // Copy member_tag-scope memories (re-map scope_id)
    for (&old_tag_id, &new_tag_id) in &mapping.member_tags {
        copy_memories_for_scope(pool, "member_tag", old_tag_id, new_tag_id, created_by).await?;
    }

    // Copy task_template-scope memories (re-map scope_id)
    for (&old_template_id, &new_template_id) in &mapping.task_templates {
        copy_memories_for_scope(
            pool,
            "task_template",
            old_template_id,
            new_template_id,
            created_by,
        )
        .await?;
    }

    Ok(())
}

async fn copy_memories_for_scope(
    pool: &PgPool,
    scope_type: &str,
    source_scope_id: Uuid,
    target_scope_id: Uuid,
    created_by: Uuid,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT content, library_ref, source
         FROM memories
         WHERE scope_type = $1 AND scope_id = $2 AND deleted_at IS NULL",
    )
    .bind(scope_type)
    .bind(source_scope_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let content: String = row.try_get("content")?;
        let library_ref: Option<Uuid> = row.try_get("library_ref")?;
        let source: String = row.try_get("source")?;

        let new_id = generate_id();
        sqlx::query(
            "INSERT INTO memories (id, scope_type, scope_id, content, library_ref, source, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(new_id)
        .bind(scope_type)
        .bind(target_scope_id)
        .bind(&content)
        .bind(library_ref)
        .bind(&source)
        .bind(created_by)
        .execute(pool)
        .await?;

        // Create initial version
        sqlx::query(
            "INSERT INTO memory_versions (id, memory_id, content, changed_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(generate_id())
        .bind(new_id)
        .bind(&content)
        .bind(created_by)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn copy_tool_configs(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
) -> Result<(), ProjectError> {
    let rows = sqlx::query(
        "SELECT tool_type, tool_name, display_name, description, enabled, config, mcp_server_config
         FROM tool_configs
         WHERE scope_type = 'project' AND scope_id = $1 AND deleted_at IS NULL",
    )
    .bind(source_project_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let tool_type: String = row.try_get("tool_type")?;
        let tool_name: String = row.try_get("tool_name")?;
        let display_name: Option<String> = row.try_get("display_name")?;
        let description: Option<String> = row.try_get("description")?;
        let enabled: bool = row.try_get("enabled")?;
        let config: serde_json::Value = row.try_get("config")?;
        let mcp_server_config: Option<serde_json::Value> = row.try_get("mcp_server_config")?;

        let new_id = generate_id();
        sqlx::query(
            "INSERT INTO tool_configs (id, scope_type, scope_id, tool_type, tool_name, display_name, description, enabled, config, mcp_server_config)
             VALUES ($1, 'project', $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(new_id)
        .bind(target_project_id)
        .bind(&tool_type)
        .bind(&tool_name)
        .bind(display_name.as_deref())
        .bind(description.as_deref())
        .bind(enabled)
        .bind(&config)
        .bind(&mcp_server_config)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn copy_permission_settings(
    pool: &PgPool,
    source_project_id: Uuid,
    target_project_id: Uuid,
) -> Result<(), ProjectError> {
    sqlx::query(
        "UPDATE projects
         SET permission_settings = (
             SELECT permission_settings FROM projects WHERE id = $1
         )
         WHERE id = $2",
    )
    .bind(source_project_id)
    .bind(target_project_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Internal helper types ──────────────────────────────────────

struct TodoTemplateData {
    old_id: Uuid,
    parent_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    sort_order: i32,
}
