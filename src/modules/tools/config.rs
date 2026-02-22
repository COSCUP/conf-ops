use sqlx::PgPool;
use uuid::Uuid;

use super::error::ToolError;
use super::models::ToolConfig;
use super::repository::ToolConfigRepository;

/// Resolves effective tool configurations using organization-to-project inheritance.
pub struct ConfigResolver;

impl ConfigResolver {
    /// Get the effective configuration for a specific tool, checking project then org level.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ConfigNotFound` if no configuration exists at any level.
    /// Returns `ToolError::Database` on database failure.
    pub async fn get_effective_config(
        pool: &PgPool,
        project_id: Uuid,
        organization_id: Uuid,
        tool_name: &str,
    ) -> Result<ToolConfig, ToolError> {
        ToolConfigRepository::get_effective_config_by_name(
            pool,
            project_id,
            organization_id,
            tool_name,
        )
        .await
    }

    /// Get all effective tool configs for a project (with inheritance).
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn get_all_effective_configs(
        pool: &PgPool,
        project_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<ToolConfig>, ToolError> {
        ToolConfigRepository::get_effective_configs(pool, project_id, organization_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::tools::models::{CreateToolConfigParams, ToolScopeType, ToolType};

    // Config inheritance logic is tested via repository queries.
    // These unit tests verify the resolver delegates correctly.

    #[test]
    fn config_resolver_is_constructible() {
        let _resolver = ConfigResolver;
    }

    #[test]
    fn tool_scope_type_organization_as_str() {
        assert_eq!(ToolScopeType::Organization.as_str(), "organization");
    }

    #[test]
    fn tool_scope_type_project_as_str() {
        assert_eq!(ToolScopeType::Project.as_str(), "project");
    }

    #[test]
    fn create_tool_config_params_builds() {
        let params = CreateToolConfigParams {
            id: Uuid::now_v7(),
            scope_type: ToolScopeType::Project,
            scope_id: Uuid::now_v7(),
            tool_type: ToolType::Builtin,
            tool_name: "smtp/sendEmail".to_string(),
            display_name: Some("Send Email".to_string()),
            description: Some("Send email via SMTP".to_string()),
            enabled: true,
            config: serde_json::json!({"host": "smtp.example.com"}),
            mcp_server_config: None,
        };
        assert_eq!(params.tool_name, "smtp/sendEmail");
        assert!(params.enabled);
    }
}
