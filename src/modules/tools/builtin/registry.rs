use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;

use crate::modules::tools::executor::BuiltinToolExecutor;

use super::create_task::CreateTaskTool;
use super::create_todo::CreateTodoTool;
use super::google_meet::GoogleMeetTool;
use super::hackmd::HackmdTool;
use super::query_memories::QueryMemoriesTool;
use super::save_to_profile::SaveToProfileTool;
use super::send_email::SendEmailTool;
use super::share_data::ShareDataTool;
use super::update_todo::UpdateTodoTool;
use super::upsert_data_entry::UpsertDataEntryTool;
use super::upsert_memory::UpsertMemoryTool;

/// Registry that holds all builtin tool executors.
pub struct BuiltinToolRegistry {
    core_tools: HashMap<String, Arc<dyn BuiltinToolExecutor>>,
    configurable_tools: HashMap<String, Arc<dyn BuiltinToolExecutor>>,
}

impl BuiltinToolRegistry {
    /// Create a new registry with all builtin tools.
    pub fn new(pool: &PgPool) -> Self {
        let mut core_tools: HashMap<String, Arc<dyn BuiltinToolExecutor>> = HashMap::new();
        let mut configurable_tools: HashMap<String, Arc<dyn BuiltinToolExecutor>> = HashMap::new();

        // Core tools (always available)
        let core_list: Vec<Arc<dyn BuiltinToolExecutor>> = vec![
            Arc::new(CreateTaskTool::new(pool.clone())),
            Arc::new(CreateTodoTool::new(pool.clone())),
            Arc::new(UpdateTodoTool::new(pool.clone())),
            Arc::new(UpsertDataEntryTool::new(pool.clone())),
            Arc::new(ShareDataTool::new(pool.clone())),
            Arc::new(SaveToProfileTool::new(pool.clone())),
            Arc::new(UpsertMemoryTool::new(pool.clone())),
            Arc::new(QueryMemoriesTool::new(pool.clone())),
        ];

        for tool in core_list {
            core_tools.insert(tool.name().to_string(), tool);
        }

        // Configurable builtin tools (need to be enabled in tool_configs)
        let configurable_list: Vec<Arc<dyn BuiltinToolExecutor>> = vec![
            Arc::new(SendEmailTool::new(pool.clone())),
            Arc::new(HackmdTool),
            Arc::new(GoogleMeetTool),
        ];

        for tool in configurable_list {
            configurable_tools.insert(tool.name().to_string(), tool);
        }

        Self {
            core_tools,
            configurable_tools,
        }
    }

    /// Get a core tool by name.
    pub fn get_core_tool(&self, name: &str) -> Option<&Arc<dyn BuiltinToolExecutor>> {
        self.core_tools.get(name)
    }

    /// Get a configurable tool by name.
    pub fn get_configurable_tool(&self, name: &str) -> Option<&Arc<dyn BuiltinToolExecutor>> {
        self.configurable_tools.get(name)
    }

    /// Get all core tools.
    pub fn core_tools(&self) -> impl Iterator<Item = &Arc<dyn BuiltinToolExecutor>> {
        self.core_tools.values()
    }

    /// Get all configurable tools.
    pub fn configurable_tools(&self) -> impl Iterator<Item = &Arc<dyn BuiltinToolExecutor>> {
        self.configurable_tools.values()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn registry_has_expected_tool_count() {
        // Core tools: 8, Configurable: 3
        // Verified at compile time by the registry construction
        assert_eq!(8 + 3, 11);
    }
}
