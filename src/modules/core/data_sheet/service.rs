use sqlx::PgPool;
use uuid::Uuid;

use crate::events::EventBus;
use crate::id::generate_id;
use crate::modules::core::crdt::repository::CrdtRepository;

use super::error::DataSheetError;
use super::models::DataEntry;
use super::repository::DataEntryRepository;
use crate::modules::core::task_template::models::{DataSchemaField, FieldType};
use crate::modules::core::task_template::repository::TaskTemplateRepository;

pub struct DataSheetService {
    pool: PgPool,
    _event_bus: EventBus,
}

impl DataSheetService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self {
            pool,
            _event_bus: event_bus,
        }
    }

    /// Upsert a data entry for a task and schema.
    ///
    /// Validates the values against the schema field definitions before writing.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::SchemaNotFound` if the schema does not exist.
    /// Returns `DataSheetError::ValidationFailed` if values fail validation.
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn upsert_entry(
        &self,
        task_id: Uuid,
        data_schema_id: Uuid,
        values: &serde_json::Value,
        account_id: Uuid,
    ) -> Result<DataEntry, DataSheetError> {
        let schema = TaskTemplateRepository::get_data_schema_by_id(&self.pool, data_schema_id)
            .await
            .map_err(|_| DataSheetError::SchemaNotFound)?;

        let fields: Vec<DataSchemaField> = serde_json::from_value(schema.fields)
            .map_err(|e| DataSheetError::ValidationFailed(format!("Invalid schema fields: {e}")))?;

        validate_values(values, &fields)?;

        // Merge with existing values if entry already exists
        let merged = if let Ok(existing) =
            DataEntryRepository::get_by_task_and_schema(&self.pool, task_id, data_schema_id).await
        {
            let mut merged = existing.values;
            if let (Some(existing_obj), Some(new_obj)) =
                (merged.as_object_mut(), values.as_object())
            {
                for (key, value) in new_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            }
            merged
        } else {
            values.clone()
        };

        let id = generate_id();
        let entry =
            DataEntryRepository::upsert(&self.pool, id, task_id, data_schema_id, &merged).await?;

        // Record CRDT operation for data entry change
        let crdt_op = serde_json::json!({
            "type": "upsert",
            "schema_id": data_schema_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        // Best-effort CRDT recording
        let _ = CrdtRepository::insert_operation(
            &self.pool,
            op_id,
            "data_entry",
            task_id,
            &op_bytes,
            account_id,
        )
        .await;

        Ok(entry)
    }

    /// Get a data entry by task and schema.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::NotFound` if the entry does not exist.
    pub async fn get_entry(
        &self,
        task_id: Uuid,
        data_schema_id: Uuid,
    ) -> Result<DataEntry, DataSheetError> {
        DataEntryRepository::get_by_task_and_schema(&self.pool, task_id, data_schema_id).await
    }

    /// List all data entries for a task.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn list_entries(&self, task_id: Uuid) -> Result<Vec<DataEntry>, DataSheetError> {
        DataEntryRepository::list_by_task(&self.pool, task_id).await
    }

    /// Delete a data entry (soft-delete).
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::NotFound` if the entry does not exist.
    pub async fn delete_entry(
        &self,
        task_id: Uuid,
        data_schema_id: Uuid,
        account_id: Uuid,
    ) -> Result<(), DataSheetError> {
        DataEntryRepository::soft_delete(&self.pool, task_id, data_schema_id).await?;

        let crdt_op = serde_json::json!({
            "type": "delete",
            "schema_id": data_schema_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool,
            op_id,
            "data_entry",
            task_id,
            &op_bytes,
            account_id,
        )
        .await;

        Ok(())
    }

    /// List all entries for a schema (aggregation view).
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::Database` on database failure.
    pub async fn get_aggregated(
        &self,
        data_schema_id: Uuid,
    ) -> Result<Vec<DataEntry>, DataSheetError> {
        DataEntryRepository::list_by_schema(&self.pool, data_schema_id).await
    }

    /// Share data fields from one task's data entry to another task.
    ///
    /// Copies specified fields from source entry to target entry, updating
    /// the target's `source_links` to record the sharing relationship.
    ///
    /// # Errors
    ///
    /// Returns `DataSheetError::NotFound` if the source entry does not exist.
    /// Returns `DataSheetError::TargetTaskNotFound` if the target entry cannot be created.
    /// Returns `DataSheetError::SourceFieldNotFound` if a source field key does not exist.
    pub async fn share_data_to_task(
        &self,
        source_task_id: Uuid,
        source_schema_id: Uuid,
        target_task_id: Uuid,
        target_schema_id: Uuid,
        field_mappings: &[(String, String)],
        account_id: Uuid,
    ) -> Result<DataEntry, DataSheetError> {
        let source_entry = DataEntryRepository::get_by_task_and_schema(
            &self.pool,
            source_task_id,
            source_schema_id,
        )
        .await?;

        let source_obj = source_entry.values.as_object().ok_or_else(|| {
            DataSheetError::ValidationFailed("source values not an object".to_string())
        })?;

        // Build target values from field mappings
        let mut shared_values = serde_json::Map::new();
        for (source_key, target_key) in field_mappings {
            let value = source_obj
                .get(source_key)
                .ok_or_else(|| DataSheetError::SourceFieldNotFound(source_key.clone()))?;
            shared_values.insert(target_key.clone(), value.clone());
        }

        // Upsert the target entry with shared values
        let target_entry = self
            .upsert_entry(
                target_task_id,
                target_schema_id,
                &serde_json::Value::Object(shared_values),
                account_id,
            )
            .await
            .map_err(|e| match e {
                DataSheetError::SchemaNotFound => DataSheetError::TargetTaskNotFound,
                other => other,
            })?;

        // Update source_links on the target entry
        let existing_links = target_entry
            .source_links
            .as_ref()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();

        let new_link = serde_json::json!({
            "sourceTaskId": source_task_id.to_string(),
            "sourceSchemaId": source_schema_id.to_string(),
            "fieldMappings": field_mappings.iter().map(|(s, t)| {
                serde_json::json!({"sourceKey": s, "targetKey": t})
            }).collect::<Vec<_>>(),
            "sharedAt": chrono::Utc::now().to_rfc3339(),
        });

        let mut links = existing_links;
        links.push(new_link);
        let links_value = serde_json::Value::Array(links);

        let updated = DataEntryRepository::update_source_links(
            &self.pool,
            target_task_id,
            target_schema_id,
            &links_value,
        )
        .await?;

        // CRDT recording
        let crdt_op = serde_json::json!({
            "type": "share_data",
            "source_task_id": source_task_id.to_string(),
            "source_schema_id": source_schema_id.to_string(),
            "target_schema_id": target_schema_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let op_bytes = serde_json::to_vec(&crdt_op).unwrap_or_default();
        let op_id = generate_id();
        let _ = CrdtRepository::insert_operation(
            &self.pool,
            op_id,
            "data_entry",
            target_task_id,
            &op_bytes,
            account_id,
        )
        .await;

        Ok(updated)
    }
}

fn validate_values(
    values: &serde_json::Value,
    fields: &[DataSchemaField],
) -> Result<(), DataSheetError> {
    let obj = values
        .as_object()
        .ok_or_else(|| DataSheetError::ValidationFailed("values must be an object".to_string()))?;

    // Check that all provided keys exist in the schema
    let field_keys: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
    for key in obj.keys() {
        if !field_keys.contains(&key.as_str()) {
            return Err(DataSheetError::ValidationFailed(format!(
                "Unknown field key: {key}"
            )));
        }
    }

    // Validate each provided value against its field definition
    for (key, value) in obj {
        if value.is_null() {
            continue;
        }

        let field = fields.iter().find(|f| f.key == *key).ok_or_else(|| {
            DataSheetError::ValidationFailed(format!("Field definition not found for key: {key}"))
        })?;

        validate_field_value(key, value, field)?;
    }

    Ok(())
}

fn validate_field_value(
    key: &str,
    value: &serde_json::Value,
    field: &DataSchemaField,
) -> Result<(), DataSheetError> {
    match field.field_type {
        FieldType::SingleLineText
        | FieldType::MultiLineText
        | FieldType::Email
        | FieldType::Url => {
            if !value.is_string() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a string"
                )));
            }
            if let (Some(s), Some(constraints)) = (value.as_str(), &field.constraints) {
                if let Some(max_len) = constraints.max_length {
                    if s.len() > max_len as usize {
                        return Err(DataSheetError::ValidationFailed(format!(
                            "Field '{key}' exceeds max length {max_len}"
                        )));
                    }
                }
            }
        }
        FieldType::Number => {
            if !value.is_number() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a number"
                )));
            }
            if let (Some(n), Some(constraints)) = (value.as_f64(), &field.constraints) {
                if let Some(min) = constraints.min {
                    if n < min {
                        return Err(DataSheetError::ValidationFailed(format!(
                            "Field '{key}' is below minimum {min}"
                        )));
                    }
                }
                if let Some(max) = constraints.max {
                    if n > max {
                        return Err(DataSheetError::ValidationFailed(format!(
                            "Field '{key}' exceeds maximum {max}"
                        )));
                    }
                }
            }
        }
        FieldType::Date => {
            if !value.is_string() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a date string"
                )));
            }
        }
        FieldType::Boolean => {
            if !value.is_boolean() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a boolean"
                )));
            }
        }
        FieldType::Select => {
            if !value.is_string() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a string (select option)"
                )));
            }
            if let Some(constraints) = &field.constraints {
                if let (Some(options), Some(s)) = (&constraints.options, value.as_str()) {
                    if !options.contains(&s.to_string()) {
                        return Err(DataSheetError::ValidationFailed(format!(
                            "Field '{key}' value '{s}' is not a valid option"
                        )));
                    }
                }
            }
        }
        FieldType::Image | FieldType::File => {
            if !value.is_string() {
                return Err(DataSheetError::ValidationFailed(format!(
                    "Field '{key}' expects a string (file ID / UUID)"
                )));
            }
        }
    }
    Ok(())
}
