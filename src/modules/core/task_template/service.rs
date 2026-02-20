use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::TaskTemplateError;
use super::models::{
    DataSchema, DataSchemaField, FieldConstraints, FieldType, TaskTemplate, TaskTemplateTag,
    TodoTemplate,
};
use super::repository::TaskTemplateRepository;

pub struct TaskTemplateService {
    pool: PgPool,
    event_bus: EventBus,
}

impl TaskTemplateService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    // ── Task Template CRUD ──────────────────────────────────────

    /// Create a new task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create_template(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        created_by: Uuid,
    ) -> Result<TaskTemplate, TaskTemplateError> {
        let id = generate_id();
        let template = TaskTemplateRepository::create(
            &self.pool,
            id,
            project_id,
            name,
            description,
            created_by,
        )
        .await?;

        self.event_bus.publish(DomainEvent::TaskTemplateCreated {
            template_id: id,
            project_id,
        });

        Ok(template)
    }

    /// Get a task template by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist.
    pub async fn get_template(&self, id: Uuid) -> Result<TaskTemplate, TaskTemplateError> {
        TaskTemplateRepository::get_by_id(&self.pool, id).await
    }

    /// List all task templates for a project.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_templates(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<TaskTemplate>, TaskTemplateError> {
        TaskTemplateRepository::list_by_project(&self.pool, project_id).await
    }

    /// Update a task template's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist.
    pub async fn update_template(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<TaskTemplate, TaskTemplateError> {
        TaskTemplateRepository::update(&self.pool, id, name, description).await
    }

    /// Delete a task template (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NotFound` if the template does not exist.
    pub async fn delete_template(&self, id: Uuid) -> Result<(), TaskTemplateError> {
        let template = TaskTemplateRepository::get_by_id(&self.pool, id).await?;
        TaskTemplateRepository::soft_delete(&self.pool, id).await?;

        self.event_bus.publish(DomainEvent::TaskTemplateDeleted {
            template_id: id,
            project_id: template.project_id,
        });

        Ok(())
    }

    // ── Tag linking ─────────────────────────────────────────────

    /// Link a member tag to a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TagAlreadyLinked` if already linked.
    /// Returns `TaskTemplateError::Database` on other database failure.
    pub async fn link_tag(
        &self,
        task_template_id: Uuid,
        member_tag_id: Uuid,
    ) -> Result<TaskTemplateTag, TaskTemplateError> {
        let id = generate_id();
        TaskTemplateRepository::link_tag(&self.pool, id, task_template_id, member_tag_id).await
    }

    /// Unlink a member tag from a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TagLinkNotFound` if the link does not exist.
    pub async fn unlink_tag(
        &self,
        task_template_id: Uuid,
        member_tag_id: Uuid,
    ) -> Result<(), TaskTemplateError> {
        TaskTemplateRepository::unlink_tag(&self.pool, task_template_id, member_tag_id).await
    }

    /// List all tags linked to a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_tags(
        &self,
        task_template_id: Uuid,
    ) -> Result<Vec<TaskTemplateTag>, TaskTemplateError> {
        TaskTemplateRepository::list_tags(&self.pool, task_template_id).await
    }

    // ── Todo templates ──────────────────────────────────────────

    /// Create a new todo template with optional parent.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::NestingLimitExceeded` if the parent is not top-level.
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create_todo_template(
        &self,
        task_template_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<TodoTemplate, TaskTemplateError> {
        // Validate nesting: parent must be a top-level todo template
        if let Some(pid) = parent_id {
            let parent = TaskTemplateRepository::get_todo_template_by_id(&self.pool, pid).await?;
            if parent.parent_id.is_some() {
                return Err(TaskTemplateError::NestingLimitExceeded);
            }
        }

        let id = generate_id();
        TaskTemplateRepository::create_todo_template(
            &self.pool,
            id,
            task_template_id,
            parent_id,
            name,
            description,
            sort_order,
        )
        .await
    }

    /// Get a todo template by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn get_todo_template(&self, id: Uuid) -> Result<TodoTemplate, TaskTemplateError> {
        TaskTemplateRepository::get_todo_template_by_id(&self.pool, id).await
    }

    /// List all todo templates for a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_todo_templates(
        &self,
        task_template_id: Uuid,
    ) -> Result<Vec<TodoTemplate>, TaskTemplateError> {
        TaskTemplateRepository::list_todo_templates(&self.pool, task_template_id).await
    }

    /// Update a todo template's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn update_todo_template(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<TodoTemplate, TaskTemplateError> {
        TaskTemplateRepository::update_todo_template(&self.pool, id, name, description).await
    }

    /// Delete a todo template (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::TodoTemplateNotFound` if the todo template does not exist.
    pub async fn delete_todo_template(&self, id: Uuid) -> Result<(), TaskTemplateError> {
        TaskTemplateRepository::delete_todo_template(&self.pool, id).await
    }

    /// Reorder todo templates by updating their sort orders.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn reorder_todo_templates(
        &self,
        orders: &[(Uuid, i32)],
    ) -> Result<(), TaskTemplateError> {
        TaskTemplateRepository::reorder_todo_templates(&self.pool, orders).await
    }

    // ── Data schemas ────────────────────────────────────────────

    /// Create a new data schema with field validation.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DuplicateFieldKey` if field keys are not unique.
    /// Returns `TaskTemplateError::InvalidFieldConstraints` on invalid constraints.
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn create_data_schema(
        &self,
        task_template_id: Uuid,
        name: &str,
        fields: Vec<DataSchemaField>,
    ) -> Result<DataSchema, TaskTemplateError> {
        validate_fields(&fields)?;

        let fields_json = serde_json::to_value(&fields)
            .map_err(|e| TaskTemplateError::InvalidFieldConstraints(e.to_string()))?;

        let id = generate_id();
        TaskTemplateRepository::create_data_schema(
            &self.pool,
            id,
            task_template_id,
            name,
            &fields_json,
        )
        .await
    }

    /// Get a data schema by ID.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn get_data_schema(&self, id: Uuid) -> Result<DataSchema, TaskTemplateError> {
        TaskTemplateRepository::get_data_schema_by_id(&self.pool, id).await
    }

    /// List all data schemas for a task template.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::Database` on database failure.
    pub async fn list_data_schemas(
        &self,
        task_template_id: Uuid,
    ) -> Result<Vec<DataSchema>, TaskTemplateError> {
        TaskTemplateRepository::list_data_schemas(&self.pool, task_template_id).await
    }

    /// Update a data schema's name and/or fields.
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DuplicateFieldKey` if field keys are not unique.
    /// Returns `TaskTemplateError::InvalidFieldConstraints` on invalid constraints.
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn update_data_schema(
        &self,
        id: Uuid,
        name: Option<&str>,
        fields: Option<Vec<DataSchemaField>>,
    ) -> Result<DataSchema, TaskTemplateError> {
        let fields_json = if let Some(f) = &fields {
            validate_fields(f)?;
            Some(
                serde_json::to_value(f)
                    .map_err(|e| TaskTemplateError::InvalidFieldConstraints(e.to_string()))?,
            )
        } else {
            None
        };

        TaskTemplateRepository::update_data_schema(&self.pool, id, name, fields_json.as_ref()).await
    }

    /// Delete a data schema (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `TaskTemplateError::DataSchemaNotFound` if the schema does not exist.
    pub async fn delete_data_schema(&self, id: Uuid) -> Result<(), TaskTemplateError> {
        TaskTemplateRepository::delete_data_schema(&self.pool, id).await
    }
}

fn validate_fields(fields: &[DataSchemaField]) -> Result<(), TaskTemplateError> {
    // Check unique keys
    let mut keys = HashSet::new();
    for field in fields {
        if !keys.insert(&field.key) {
            return Err(TaskTemplateError::DuplicateFieldKey(field.key.clone()));
        }
    }

    // Validate constraints match field type
    for field in fields {
        validate_field_constraints(field)?;
    }

    Ok(())
}

fn validate_field_constraints(field: &DataSchemaField) -> Result<(), TaskTemplateError> {
    let Some(constraints) = &field.constraints else {
        // select requires non-empty options
        if field.field_type == FieldType::Select {
            return Err(TaskTemplateError::InvalidFieldConstraints(format!(
                "field '{}': select type requires constraints with non-empty options",
                field.key
            )));
        }
        return Ok(());
    };

    match field.field_type {
        FieldType::SingleLineText => {
            reject_invalid_constraints(&field.key, constraints, &["max_length"])?;
        }
        FieldType::MultiLineText => {
            reject_invalid_constraints(&field.key, constraints, &["max_length", "max_lines"])?;
        }
        FieldType::Number => {
            reject_invalid_constraints(&field.key, constraints, &["min", "max", "decimal"])?;
        }
        FieldType::Date => {
            reject_invalid_constraints(&field.key, constraints, &["min_date", "max_date"])?;
        }
        FieldType::Email | FieldType::Url | FieldType::Boolean => {
            reject_invalid_constraints(&field.key, constraints, &[])?;
        }
        FieldType::Select => {
            reject_invalid_constraints(&field.key, constraints, &["options"])?;
            if !matches!(&constraints.options, Some(opts) if !opts.is_empty()) {
                return Err(TaskTemplateError::InvalidFieldConstraints(format!(
                    "field '{}': select type requires non-empty options",
                    field.key
                )));
            }
        }
        FieldType::Image | FieldType::File => {
            reject_invalid_constraints(&field.key, constraints, &["max_file_size", "accept"])?;
        }
    }

    Ok(())
}

fn reject_invalid_constraints(
    key: &str,
    c: &FieldConstraints,
    allowed: &[&str],
) -> Result<(), TaskTemplateError> {
    let checks: &[(&str, bool)] = &[
        ("max_length", c.max_length.is_some()),
        ("max_lines", c.max_lines.is_some()),
        ("min", c.min.is_some()),
        ("max", c.max.is_some()),
        ("decimal", c.decimal.is_some()),
        ("min_date", c.min_date.is_some()),
        ("max_date", c.max_date.is_some()),
        ("options", c.options.is_some()),
        ("max_file_size", c.max_file_size.is_some()),
        ("accept", c.accept.is_some()),
    ];

    for (name, present) in checks {
        if *present && !allowed.contains(name) {
            return Err(TaskTemplateError::InvalidFieldConstraints(format!(
                "field '{key}': constraint '{name}' is not valid for this field type"
            )));
        }
    }

    Ok(())
}
