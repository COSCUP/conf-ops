use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// API Key permissions structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyPermissions {
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_access: Option<DataAccessFilter>,
}

/// Data access filter for API keys.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataAccessFilter {
    #[serde(default)]
    pub template_ids: Vec<Uuid>,
    #[serde(default)]
    pub schema_ids: Vec<Uuid>,
}

/// An API key record from the database.
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub permissions: serde_json::Value,
    pub created_by: Uuid,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Parameters for creating an API key.
pub struct CreateApiKeyParams {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub permissions: serde_json::Value,
    pub created_by: Uuid,
}

/// Result of creating an API key, including the raw key shown only once.
pub struct ApiKeyCreated {
    pub api_key_record: ApiKey,
    pub raw_key: String,
}

/// Valid API key scopes.
pub const VALID_SCOPES: &[&str] = &[
    "read:task_templates",
    "read:tasks",
    "read:data",
    "write:data",
];

/// Check whether a scope is valid.
pub fn is_valid_scope(scope: &str) -> bool {
    VALID_SCOPES.contains(&scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_scopes_recognized() {
        assert!(is_valid_scope("read:data"));
        assert!(is_valid_scope("write:data"));
        assert!(!is_valid_scope("admin"));
    }

    #[test]
    fn permissions_serde_roundtrip() {
        let perms = ApiKeyPermissions {
            scopes: vec!["read:data".to_string(), "write:data".to_string()],
            data_access: Some(DataAccessFilter {
                template_ids: vec![Uuid::nil()],
                schema_ids: vec![],
            }),
        };

        let json = serde_json::to_string(&perms).unwrap();
        let parsed: ApiKeyPermissions = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scopes.len(), 2);
        assert!(parsed.data_access.is_some());
    }
}
