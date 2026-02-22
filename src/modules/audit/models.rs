use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Actor type for audit logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Account,
    System,
    Ai,
    ApiKey,
}

impl ActorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::System => "system",
            Self::Ai => "ai",
            Self::ApiKey => "api_key",
        }
    }
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ActorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "account" => Ok(Self::Account),
            "system" => Ok(Self::System),
            "ai" => Ok(Self::Ai),
            "api_key" => Ok(Self::ApiKey),
            other => Err(format!("Invalid actor type: {other}")),
        }
    }
}

/// An audit log record from the database.
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub context_type: Option<String>,
    pub context_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating an audit log entry.
pub struct CreateAuditLogParams {
    pub id: Uuid,
    pub actor_type: ActorType,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub context_type: Option<String>,
    pub context_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Query filters for audit logs.
#[derive(Debug, Default)]
pub struct AuditLogFilters {
    pub actor_type: Option<String>,
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub context_type: Option<String>,
    pub context_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cursor: Option<Uuid>,
    pub limit: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_type_roundtrip() {
        let variants = vec![
            (ActorType::Account, "account"),
            (ActorType::System, "system"),
            (ActorType::Ai, "ai"),
            (ActorType::ApiKey, "api_key"),
        ];

        for (variant, expected) in variants {
            assert_eq!(variant.as_str(), expected);
            assert_eq!(variant.to_string(), expected);
            let parsed: ActorType = expected.parse().unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn invalid_actor_type_returns_error() {
        assert!("invalid".parse::<ActorType>().is_err());
    }

    #[test]
    fn actor_type_serde_roundtrip() {
        let json = serde_json::to_string(&ActorType::Account).unwrap();
        assert_eq!(json, "\"account\"");
        let parsed: ActorType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ActorType::Account);
    }
}
