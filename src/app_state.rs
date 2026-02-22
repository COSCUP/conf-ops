use std::sync::Arc;

use sqlx::PgPool;

use crate::api::routes::ws::WsTokenStore;
use crate::events::EventBus;
use crate::modules::ai::decision::DecisionService;
use crate::modules::ai::memory::service::MemoryService;
use crate::modules::ai::placeholder::PlaceholderResolver;
use crate::modules::ai::privacy::PrivacyEngine;
use crate::modules::auth::jwt::JwtConfig;
use crate::modules::auth::service::AuthService;
use crate::modules::conversation::awareness::AwarenessManager;
use crate::modules::conversation::service::ConversationService;
use crate::modules::conversation::ws_manager::WsManager;
use crate::modules::core::contact::service::ContactService;
use crate::modules::core::data_sheet::service::DataSheetService;
use crate::modules::core::member::service::MemberService;
use crate::modules::core::member_tag::service::MemberTagService;
use crate::modules::core::organization::service::OrganizationService;
use crate::modules::core::permission::service::PermissionService;
use crate::modules::core::project::service::ProjectService;
use crate::modules::core::task::service::TaskService;
use crate::modules::core::task_template::service::TaskTemplateService;
use crate::modules::core::todo::service::TodoService;
use crate::modules::email::inbound::InboundEmailService;
use crate::modules::email::service::EmailOutboundService;
use crate::modules::storage::service::FileService;
use crate::modules::tools::service::ToolService;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub event_bus: EventBus,
    pub jwt_config: JwtConfig,
    pub app_base_url: String,
    pub auth_service: Arc<AuthService>,
    pub org_service: Arc<OrganizationService>,
    pub member_service: Arc<MemberService>,
    pub member_tag_service: Arc<MemberTagService>,
    pub contact_service: Arc<ContactService>,
    pub project_service: Arc<ProjectService>,
    pub permission_service: Arc<PermissionService>,
    pub task_template_service: Arc<TaskTemplateService>,
    pub task_service: Arc<TaskService>,
    pub todo_service: Arc<TodoService>,
    pub data_sheet_service: Arc<DataSheetService>,
    pub file_service: Arc<FileService>,
    pub conversation_service: Arc<ConversationService>,
    pub email_outbound_service: Arc<EmailOutboundService>,
    pub inbound_email_service: Arc<InboundEmailService>,
    pub email_inbound_api_key: Option<String>,
    pub ws_token_store: Arc<WsTokenStore>,
    pub ws_manager: Arc<WsManager>,
    pub awareness_manager: Arc<AwarenessManager>,
    pub crdt_ws_heartbeat_interval_secs: u64,
    pub crdt_ws_idle_timeout_secs: u64,
    pub memory_service: Arc<MemoryService>,
    pub decision_service: Arc<DecisionService>,
    pub placeholder_resolver: Arc<PlaceholderResolver>,
    pub privacy_engine: Arc<PrivacyEngine>,
    pub tool_service: Arc<ToolService>,
}
