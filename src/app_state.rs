use sqlx::PgPool;

use crate::events::EventBus;
use crate::modules::auth::jwt::JwtConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub event_bus: EventBus,
    pub jwt_config: JwtConfig,
    pub app_base_url: String,
}
