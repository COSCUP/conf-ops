use std::sync::Arc;

use sqlx::PgPool;

use crate::events::EventBus;
use crate::modules::auth::jwt::JwtConfig;
use crate::modules::auth::service::AuthService;
use crate::modules::core::organization::service::OrganizationService;
use crate::modules::core::permission::service::PermissionService;
use crate::modules::core::project::service::ProjectService;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub event_bus: EventBus,
    pub jwt_config: JwtConfig,
    pub app_base_url: String,
    pub auth_service: Arc<AuthService>,
    pub org_service: Arc<OrganizationService>,
    pub project_service: Arc<ProjectService>,
    pub permission_service: Arc<PermissionService>,
}
