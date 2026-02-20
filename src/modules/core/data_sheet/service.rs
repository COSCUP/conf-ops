use sqlx::PgPool;
use uuid::Uuid;

use crate::events::EventBus;
use crate::id::generate_id;

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
        DataEntryRepository::upsert(&self.pool, id, task_id, data_schema_id, &merged).await
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
    ) -> Result<(), DataSheetError> {
        DataEntryRepository::soft_delete(&self.pool, task_id, data_schema_id).await
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

        let field = fields
            .iter()
            .find(|f| f.key == *key)
            .expect("key existence already checked");

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
                    "Field '{key}' expects a string (file URL)"
                )));
            }
        }
    }
    Ok(())
}
