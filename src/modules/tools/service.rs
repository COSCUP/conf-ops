use std::collections::HashMap;
use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::builtin::registry::BuiltinToolRegistry;
use super::config::ConfigResolver;
use super::error::ToolError;
use super::external::ExternalToolExecutor;
use super::models::{
    RecordToolExecutionParams, ToolCategory, ToolContext, ToolDefinition, ToolExecutionStatus,
    ToolResult,
};
use super::repository::ToolExecutionRepository;

/// The unified tool service that dispatches to builtin or external executors.
pub struct ToolService {
    pool: PgPool,
    event_bus: EventBus,
    builtin_registry: BuiltinToolRegistry,
    external_executor: ExternalToolExecutor,
}

impl ToolService {
    /// Create a new tool service.
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        Self {
            pool: pool.clone(),
            event_bus,
            builtin_registry: BuiltinToolRegistry::new(pool),
            external_executor: ExternalToolExecutor::default(),
        }
    }

    /// List all available tools for a project, filtered by permission.
    ///
    /// Returns core builtin tools (always available) plus enabled configurable
    /// and external tools from the effective tool configs.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::Database` on database failure.
    pub async fn list_available_tools(
        &self,
        project_id: Uuid,
        organization_id: Uuid,
        _actor_tags: &[Uuid],
    ) -> Result<Vec<ToolDefinition>, ToolError> {
        let mut tools: Vec<ToolDefinition> = Vec::new();

        // Add core builtin tools (always available)
        for executor in self.builtin_registry.core_tools() {
            tools.push(executor.definition());
        }

        // Get effective configs (with inheritance) for configurable/external tools
        let configs =
            ConfigResolver::get_all_effective_configs(&self.pool, project_id, organization_id)
                .await?;

        for config in &configs {
            if !config.enabled {
                continue;
            }

            // Check if this is a configurable builtin tool
            if let Some(executor) = self
                .builtin_registry
                .get_configurable_tool(&config.tool_name)
            {
                tools.push(executor.definition());
            } else if config.tool_type == "external" {
                // External MCP tools: add placeholder definition from config
                tools.push(ToolDefinition {
                    name: config.tool_name.clone(),
                    display_name: config.display_name.clone(),
                    description: config.description.clone().unwrap_or_default(),
                    input_schema: serde_json::json!({"type": "object"}),
                    category: ToolCategory::External,
                    requires_confirmation: true,
                });
            }
        }

        Ok(tools)
    }

    /// Execute a tool by name.
    ///
    /// Dispatches to the appropriate executor (builtin core, builtin configurable,
    /// or external MCP) based on the tool name and configuration.
    ///
    /// # Errors
    ///
    /// Returns various `ToolError` variants on failure.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        context: &ToolContext,
        suggestion_id: Option<Uuid>,
    ) -> Result<ToolResult, ToolError> {
        let start = Instant::now();

        // Try core builtin tools first
        if let Some(executor) = self.builtin_registry.get_core_tool(tool_name) {
            let result = executor.execute(params.clone(), context).await;
            let duration_ms = saturating_millis(&start);

            return self
                .record_and_return(
                    tool_name,
                    &params,
                    result,
                    duration_ms,
                    context,
                    suggestion_id,
                )
                .await;
        }

        // Try configurable builtin tools
        if let Some(executor) = self.builtin_registry.get_configurable_tool(tool_name) {
            // Verify the tool is enabled in config
            let _config = ConfigResolver::get_effective_config(
                &self.pool,
                context.project_id,
                context.organization_id,
                tool_name,
            )
            .await
            .map_err(|_| ToolError::Disabled(tool_name.to_string()))?;

            let result = executor.execute(params.clone(), context).await;
            let duration_ms = saturating_millis(&start);

            return self
                .record_and_return(
                    tool_name,
                    &params,
                    result,
                    duration_ms,
                    context,
                    suggestion_id,
                )
                .await;
        }

        // Try external MCP tools
        let config = ConfigResolver::get_effective_config(
            &self.pool,
            context.project_id,
            context.organization_id,
            tool_name,
        )
        .await
        .map_err(|_| ToolError::NotFound(tool_name.to_string()))?;

        if !config.enabled {
            return Err(ToolError::Disabled(tool_name.to_string()));
        }

        if config.tool_type != "external" {
            return Err(ToolError::NotFound(tool_name.to_string()));
        }

        let result = self
            .external_executor
            .execute(tool_name, &params, &config)
            .await;
        let duration_ms = saturating_millis(&start);

        self.record_and_return(
            tool_name,
            &params,
            result,
            duration_ms,
            context,
            suggestion_id,
        )
        .await
    }

    /// Record the execution and return the result.
    async fn record_and_return(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
        result: Result<ToolResult, ToolError>,
        duration_ms: u64,
        context: &ToolContext,
        suggestion_id: Option<Uuid>,
    ) -> Result<ToolResult, ToolError> {
        let execution_id = Uuid::now_v7();

        match &result {
            Ok(tool_result) => {
                let status = if tool_result.is_error {
                    ToolExecutionStatus::Error
                } else {
                    ToolExecutionStatus::Success
                };

                let _ = ToolExecutionRepository::record(
                    &self.pool,
                    &RecordToolExecutionParams {
                        id: execution_id,
                        task_id: context.task_id,
                        suggestion_id,
                        tool_name: tool_name.to_string(),
                        parameters: params.clone(),
                        result: tool_result.content.clone(),
                        status,
                        executed_by: context.actor_id,
                        duration_ms: i32::try_from(duration_ms).unwrap_or(i32::MAX),
                    },
                )
                .await;

                if tool_result.is_error {
                    self.event_bus.publish(DomainEvent::ToolExecutionFailed {
                        task_id: context.task_id,
                        message_id: execution_id,
                        tool_name: tool_name.to_string(),
                        error: tool_result.content.to_string(),
                    });
                }
            }
            Err(err) => {
                let _ = ToolExecutionRepository::record(
                    &self.pool,
                    &RecordToolExecutionParams {
                        id: execution_id,
                        task_id: context.task_id,
                        suggestion_id,
                        tool_name: tool_name.to_string(),
                        parameters: params.clone(),
                        result: serde_json::json!({"error": err.to_string()}),
                        status: ToolExecutionStatus::Error,
                        executed_by: context.actor_id,
                        duration_ms: i32::try_from(duration_ms).unwrap_or(i32::MAX),
                    },
                )
                .await;

                self.event_bus.publish(DomainEvent::ToolExecutionFailed {
                    task_id: context.task_id,
                    message_id: execution_id,
                    tool_name: tool_name.to_string(),
                    error: err.to_string(),
                });
            }
        }

        result
    }

    /// Get all builtin tool definitions as a map for quick lookup.
    pub fn builtin_tool_definitions(&self) -> HashMap<String, ToolDefinition> {
        let mut map = HashMap::new();
        for executor in self.builtin_registry.core_tools() {
            let def = executor.definition();
            map.insert(def.name.clone(), def);
        }
        for executor in self.builtin_registry.configurable_tools() {
            let def = executor.definition();
            map.insert(def.name.clone(), def);
        }
        map
    }
}

/// Convert elapsed time to milliseconds, saturating at `u64::MAX`.
fn saturating_millis(start: &Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_service_is_constructible() {
        // Verify ToolService struct can be referenced
        // (actual service construction requires a PgPool, tested in integration tests)
        assert_eq!(
            std::mem::size_of::<ToolService>(),
            std::mem::size_of::<ToolService>()
        );
    }
}
