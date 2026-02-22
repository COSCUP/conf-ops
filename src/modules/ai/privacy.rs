use std::sync::Arc;
use std::time::Duration;

use aho_corasick::{AhoCorasick, MatchKind};
use moka::future::Cache;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// A single sensitive value mapped to its placeholder string.
pub struct MaskingEntry {
    pub value: String,
    pub placeholder: String,
}

/// An Aho-Corasick automaton that replaces sensitive values with placeholders.
pub struct MaskingDictionary {
    automaton: Option<AhoCorasick>,
    replacements: Vec<String>,
}

impl MaskingDictionary {
    /// Build a new masking dictionary from the given entries.
    ///
    /// Entries whose value is shorter than 2 characters are excluded.
    pub fn new(entries: Vec<MaskingEntry>) -> Self {
        let filtered: Vec<MaskingEntry> =
            entries.into_iter().filter(|e| e.value.len() >= 2).collect();

        if filtered.is_empty() {
            return Self {
                automaton: None,
                replacements: Vec::new(),
            };
        }

        let patterns: Vec<String> = filtered.iter().map(|e| e.value.clone()).collect();
        let replacements: Vec<String> = filtered.into_iter().map(|e| e.placeholder).collect();

        let automaton = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .ok();

        Self {
            automaton,
            replacements,
        }
    }

    /// Replace all occurrences of sensitive values in `text` with their placeholders.
    pub fn mask_text(&self, text: &str) -> String {
        let Some(ref automaton) = self.automaton else {
            return text.to_string();
        };

        let mut result = String::with_capacity(text.len());
        automaton.replace_all_with(text, &mut result, |mat, _, dst| {
            dst.push_str(&self.replacements[mat.pattern().as_usize()]);
            true
        });
        result
    }

    /// Recursively walk a JSON value, masking all string values.
    pub fn mask_json_values(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.mask_text(s)),
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.mask_json_values(v)).collect())
            }
            Value::Object(map) => {
                let masked = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.mask_json_values(v)))
                    .collect();
                Value::Object(masked)
            }
            other => other.clone(),
        }
    }
}

/// Privacy engine that caches masking dictionaries per (task, account) pair.
pub struct PrivacyEngine {
    pool: PgPool,
    cache: Cache<(Uuid, Uuid), Arc<MaskingDictionary>>,
}

impl PrivacyEngine {
    /// Create a new `PrivacyEngine` with a 10-minute TTL cache (max 500 entries).
    pub fn new(pool: PgPool) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(600))
            .max_capacity(500)
            .build();

        Self { pool, cache }
    }

    /// Build a `MaskingDictionary` by querying account profile data and task data entries.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    async fn build_dictionary(
        &self,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<MaskingDictionary, sqlx::Error> {
        let mut entries = Vec::new();

        // Query account profile_data
        let profile_row: Option<(Value,)> =
            sqlx::query_as("SELECT profile_data FROM accounts WHERE id = $1")
                .bind(account_id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((Value::Object(map),)) = profile_row {
            for (key, val) in map {
                if let Value::String(s) = val {
                    entries.push(MaskingEntry {
                        value: s,
                        placeholder: format!("{{{{profile.{key}}}}}"),
                    });
                }
            }
        }

        // Query data_entries values for the task
        let data_rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT values FROM data_entries WHERE task_id = $1 AND deleted_at IS NULL",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        for (values,) in data_rows {
            if let Value::Object(map) = values {
                for (key, val) in map {
                    if let Value::String(s) = val {
                        entries.push(MaskingEntry {
                            value: s,
                            placeholder: format!("{{{{data.{key}}}}}"),
                        });
                    }
                }
            }
        }

        // Query contacts associated with the task via owner_tag → member_tag_assignments
        let contact_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT c.name, c.email \
             FROM contacts c \
             JOIN member_tag_assignments mta ON mta.contact_id = c.id \
             JOIN tasks t ON t.owner_tag_id = mta.tag_id \
             WHERE t.id = $1 AND c.deleted_at IS NULL",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        for (idx, (name, email)) in contact_rows.into_iter().enumerate() {
            entries.push(MaskingEntry {
                value: name,
                placeholder: format!("{{{{contact.{idx}.name}}}}"),
            });
            entries.push(MaskingEntry {
                value: email,
                placeholder: format!("{{{{contact.{idx}.email}}}}"),
            });
        }

        Ok(MaskingDictionary::new(entries))
    }

    /// Get (or build and cache) the masking dictionary for a task/account pair.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn get_dictionary(
        &self,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<Arc<MaskingDictionary>, sqlx::Error> {
        let key = (task_id, account_id);
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let dict = self.build_dictionary(task_id, account_id).await?;
        let arc = Arc::new(dict);
        self.cache.insert(key, Arc::clone(&arc)).await;
        Ok(arc)
    }

    /// Mask a data schema by stripping all values, keeping only the field structure.
    ///
    /// Returns a JSON object with field names as keys and their types as values
    /// (e.g. `{ "companyName": "string", "revenue": "number" }`).
    pub fn mask_data_schema(schema: &Value) -> Value {
        match schema {
            Value::Object(map) => {
                let masked = map
                    .iter()
                    .map(|(k, v)| {
                        let type_str = match v {
                            Value::String(_) => "string",
                            Value::Number(_) => "number",
                            Value::Bool(_) => "boolean",
                            Value::Array(_) => "array",
                            Value::Object(_) => "object",
                            Value::Null => "null",
                        };
                        (k.clone(), Value::String(type_str.to_string()))
                    })
                    .collect();
                Value::Object(masked)
            }
            other => other.clone(),
        }
    }

    /// Mask a profile by stripping all values, keeping only the field names.
    ///
    /// Returns a JSON object with field names as keys and `"string"` as placeholder type.
    pub fn mask_profile(profile: &Value) -> Value {
        Self::mask_data_schema(profile)
    }

    /// Mask sensitive values in a slice of conversation messages.
    ///
    /// - `member` and `email_inbound` messages: mask free text content.
    /// - `tool_execution` and `system` messages: mask values in structured JSONB.
    /// - `ai_suggestion` messages: pass through as-is (already masked).
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure when building the dictionary.
    pub async fn mask_messages(
        &self,
        task_id: Uuid,
        account_id: Uuid,
        messages: &[Value],
    ) -> Result<Vec<Value>, sqlx::Error> {
        let dict = self.get_dictionary(task_id, account_id).await?;

        let masked = messages
            .iter()
            .map(|msg| {
                let source_type = msg["sourceType"].as_str().unwrap_or("");
                match source_type {
                    "member" | "email_inbound" => {
                        let mut m = msg.clone();
                        if let Some(content) = m.get_mut("content") {
                            if let Some(Value::String(s)) = content.get_mut("text") {
                                *s = dict.mask_text(s);
                            }
                        }
                        m
                    }
                    "tool_execution" | "system" => {
                        let mut m = msg.clone();
                        if let Some(content) = m.get("content") {
                            let masked_content = dict.mask_json_values(content);
                            m["content"] = masked_content;
                        }
                        m
                    }
                    // ai_suggestion and any other types pass through unchanged
                    _ => msg.clone(),
                }
            })
            .collect();

        Ok(masked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_text_replaces_sensitive_values() {
        let dict = MaskingDictionary::new(vec![
            MaskingEntry {
                value: "John Doe".to_string(),
                placeholder: "{{profile.name}}".to_string(),
            },
            MaskingEntry {
                value: "john@example.com".to_string(),
                placeholder: "{{profile.email}}".to_string(),
            },
        ]);

        let result = dict.mask_text("Hello John Doe, your email is john@example.com");
        assert_eq!(
            result,
            "Hello {{profile.name}}, your email is {{profile.email}}"
        );
    }

    #[test]
    fn mask_text_filters_short_values() {
        let dict = MaskingDictionary::new(vec![
            MaskingEntry {
                value: "A".to_string(),
                placeholder: "{{profile.initial}}".to_string(),
            },
            MaskingEntry {
                value: "AB".to_string(),
                placeholder: "{{profile.code}}".to_string(),
            },
        ]);

        let result = dict.mask_text("Values: A and AB here");
        assert_eq!(result, "Values: A and {{profile.code}} here");
    }

    #[test]
    fn mask_text_empty_entries() {
        let dict = MaskingDictionary::new(vec![]);
        let result = dict.mask_text("Nothing to mask here");
        assert_eq!(result, "Nothing to mask here");
    }

    #[test]
    fn mask_data_schema_strips_values_keeps_types() {
        let schema = serde_json::json!({
            "companyName": "Acme Corp",
            "revenue": 1000000,
            "active": true,
            "tags": ["tech", "ai"],
            "address": { "city": "Taipei" }
        });

        let result = PrivacyEngine::mask_data_schema(&schema);
        assert_eq!(result["companyName"], "string");
        assert_eq!(result["revenue"], "number");
        assert_eq!(result["active"], "boolean");
        assert_eq!(result["tags"], "array");
        assert_eq!(result["address"], "object");
    }

    #[test]
    fn mask_profile_strips_values() {
        let profile = serde_json::json!({
            "name": "Alice",
            "email": "alice@example.com",
            "phone": "0912345678"
        });

        let result = PrivacyEngine::mask_profile(&profile);
        assert_eq!(result["name"], "string");
        assert_eq!(result["email"], "string");
        assert_eq!(result["phone"], "string");
    }

    #[test]
    fn mask_json_values_recurses() {
        let dict = MaskingDictionary::new(vec![MaskingEntry {
            value: "secret".to_string(),
            placeholder: "{{data.field}}".to_string(),
        }]);

        let input = serde_json::json!({
            "name": "secret",
            "nested": {
                "value": "has secret inside"
            },
            "list": ["secret", "other"],
            "number": 42
        });

        let result = dict.mask_json_values(&input);
        assert_eq!(result["name"], "{{data.field}}");
        assert_eq!(result["nested"]["value"], "has {{data.field}} inside");
        assert_eq!(result["list"][0], "{{data.field}}");
        assert_eq!(result["list"][1], "other");
        assert_eq!(result["number"], 42);
    }
}
